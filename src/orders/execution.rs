use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex, MutexGuard},
};

use cfmms::pool::Pool;
use ethers::{
    abi::{ethabi::Bytes, Token},
    prelude::NonceManagerMiddleware,
    providers::Middleware,
    types::{
        transaction::eip2718::TypedTransaction, Eip1559TransactionRequest, TransactionRequest,
        H160, H256, U256,
    },
};

use crate::{
    abi,
    config::{self, Chain},
    error::ExecutorError,
    markets::market::Market,
    transaction_utils,
};

use super::{
    limit_order::LimitOrder, order::Order, sandbox_limit_order::SandboxLimitOrder, simulate,
};

pub trait ExecutionCalldata {
    fn to_bytes(&self) -> Bytes;
}

#[derive(Debug, Default)]
pub struct SandboxLimitOrderExecutionCalldata {
    current_id_bundle_idx: usize,
    pub order_id_bundles: Vec<Vec<H256>>, //bytes32[][] orderIdBundles
    pub fill_amounts: Vec<u128>,          // uint128[] fillAmounts
    pub transfer_addresses: Vec<H160>,    // address[] transferAddresses
    pub calls: Vec<Call>,                 // Call[] calls
}

#[derive(Debug, Default)]
pub struct Call {
    pub target: H160,       // address target
    pub call_data: Vec<u8>, // bytes callData
}

impl SandboxLimitOrderExecutionCalldata {
    pub fn new() -> SandboxLimitOrderExecutionCalldata {
        let calldata = SandboxLimitOrderExecutionCalldata::default();
        calldata.add_new_order_id_bundle();
        calldata
    }

    pub fn add_order_id_to_current_bundle(&mut self, order_id: H256) {
        self.order_id_bundles[self.current_id_bundle_idx].push(order_id);
    }

    pub fn add_new_order_id_bundle(&mut self) {
        if self.order_id_bundles.is_empty() {
            self.order_id_bundles.push(vec![]);
            self.current_id_bundle_idx += 1;
        } else if !self.order_id_bundles[self.current_id_bundle_idx].is_empty() {
            self.order_id_bundles.push(vec![]);
            self.current_id_bundle_idx += 1;
        }
    }

    pub fn add_fill_amount(&mut self, fill_amount: u128) {
        self.fill_amounts.push(fill_amount);
    }

    pub fn add_transfer_address(&mut self, transfer_address: H160) {
        self.transfer_addresses.push(transfer_address);
    }

    pub fn add_call(&mut self, call: Call) {
        self.calls.push(call);
    }
}

impl Call {
    pub fn new(target: H160, call_data: Vec<u8>) -> Call {
        Call { target, call_data }
    }
}

#[derive(Default, Debug)]
pub struct LimitOrderExecutionBundle {
    pub order_groups: Vec<LimitOrderExecutionOrderIds>, // bytes32[] calldata orderIds
}

impl LimitOrderExecutionBundle {
    pub fn new() -> LimitOrderExecutionBundle {
        LimitOrderExecutionBundle::default()
    }

    pub fn add_order_group(&mut self, order_group: LimitOrderExecutionOrderIds) {
        self.order_groups.push(order_group);
    }

    pub fn add_empty_order_group(&mut self) {
        if let Some(order_group) = self.order_groups.last() {
            if !order_group.order_ids.is_empty() {
                self.order_groups
                    .push(LimitOrderExecutionOrderIds::default());
            }
        } else {
            self.order_groups
                .push(LimitOrderExecutionOrderIds::default());
        };
    }

    pub fn append_order_id_to_latest_order_group(&mut self, order_id: H256) {
        if let Some(order_group) = self.order_groups.last_mut() {
            order_group.add_order_id(order_id);
        } else {
            self.add_empty_order_group();
            self.append_order_id_to_latest_order_group(order_id);
        }
    }
}

impl ExecutionCalldata for LimitOrderExecutionBundle {
    fn to_bytes(&self) -> Bytes {
        self.order_groups
            .iter()
            .flat_map(|order_group| order_group.to_bytes())
            .collect::<Vec<u8>>()
    }
}

#[derive(Default, Debug)]
pub struct LimitOrderExecutionOrderIds {
    pub order_ids: Vec<[u8; 32]>, // bytes32[] calldata orderIds
}

impl LimitOrderExecutionOrderIds {
    pub fn new() -> LimitOrderExecutionOrderIds {
        LimitOrderExecutionOrderIds::default()
    }

    pub fn add_order_id(&mut self, order_id: H256) {
        self.order_ids.push(order_id.to_fixed_bytes());
    }
}

impl ExecutionCalldata for LimitOrderExecutionOrderIds {
    fn to_bytes(&self) -> Bytes {
        ethers::abi::encode(
            &self
                .order_ids
                .iter()
                .map(|order_id| Token::FixedBytes(order_id.to_vec()))
                .collect::<Vec<Token>>(),
        )
    }
}

pub async fn fill_orders_at_execution_price<M: Middleware>(
    active_orders: Arc<Mutex<HashMap<H256, Order>>>,
    markets: Arc<Mutex<HashMap<U256, Market>>>,
    configuration: &config::Config,
    pending_transactions_sender: Arc<tokio::sync::mpsc::Sender<(H256, Vec<H256>)>>,
    middleware: Arc<M>,
) -> Result<(), ExecutorError<M>> {
    //:: Get to each order in the affected orders, check if they are ready for execution and then add them to the data structures mentioned above, which will then be used to simulate orders and generate execution calldata.
    let markets = markets.lock().expect("Could not acquire mutex lock");
    let active_orders = active_orders.lock().expect("Could not acquire mutex lock");
    //NOTE: remove this note with a better comment
    //Clone the markets to simulate all active orders, only do this on initialization, this would be heavy on every time checking order execution, select simulated markets instead
    let mut simulated_markets = markets.clone();

    //TODO: package this in a function

    //:: group all of the orders that are ready to execute and separate them by sandbox limit orders and limit orders
    //Accumulate sandbox limit orders at execution price
    let mut slo_at_execution_price: HashMap<H256, &SandboxLimitOrder> = HashMap::new();
    //Accumulate limit orders at execution price
    let mut lo_at_execution_price: HashMap<H256, &LimitOrder> = HashMap::new();

    for order in active_orders.values() {
        if order.can_execute(&markets, configuration.weth_address) {
            if order.has_sufficient_balance(middleware.clone()).await? {
                //Add order to orders at execution price
                match order {
                    Order::SandboxLimitOrder(sandbox_limit_order) => {
                        if slo_at_execution_price
                            .get(&sandbox_limit_order.order_id)
                            .is_none()
                        {
                            slo_at_execution_price
                                .insert(sandbox_limit_order.order_id, sandbox_limit_order);
                        }
                    }

                    Order::LimitOrder(limit_order) => {
                        if lo_at_execution_price.get(&limit_order.order_id).is_none() {
                            lo_at_execution_price.insert(limit_order.order_id, limit_order);
                        }
                    }
                }
            }
        }
    }

    //::TODO: Simulate sandbox limit orders and generate execution transaction calldata
    simulate::simulate_and_batch_sandbox_limit_orders(
        slo_at_execution_price,
        &mut simulated_markets,
        configuration.weth_address,
        middleware.clone(),
    )
    .await?;

    //simulate and batch limit orders
    //:: Simulate sandbox limit orders and generate execution transaction calldata
    let limit_order_execution_bundle = simulate::simulate_and_batch_limit_orders(
        lo_at_execution_price,
        &mut simulated_markets,
        configuration.weth_address,
        middleware.clone(),
    )
    .await?;

    //Execute orders if there are any order groups for execution
    if !limit_order_execution_bundle.order_groups.is_empty() {
        //execute sandbox limit orders
        execute_limit_order_groups(
            limit_order_execution_bundle,
            configuration,
            pending_transactions_sender,
            middleware.clone(),
        )
        .await?;
    }
    Ok(())
}

pub async fn execute_limit_order_groups<M: Middleware>(
    limit_order_execution_bundle: LimitOrderExecutionBundle,
    configuration: &config::Config,
    pending_transactions_sender: Arc<tokio::sync::mpsc::Sender<(H256, Vec<H256>)>>,
    middleware: Arc<M>,
) -> Result<(), ExecutorError<M>> {
    // execute limit orders
    for order_group in limit_order_execution_bundle.order_groups {
        if !order_group.order_ids.is_empty() {
            let tx = transaction_utils::construct_and_simulate_lo_execution_transaction(
                configuration,
                order_group.order_ids.clone(),
                middleware.clone(),
            )
            .await?;

            let pending_tx_hash = transaction_utils::sign_and_send_transaction(
                tx,
                &configuration.wallet_key,
                &configuration.chain,
                middleware.clone(),
            )
            .await?;

            tracing::info!("Pending limit order execution tx: {:?}", pending_tx_hash);

            let order_ids = order_group
                .order_ids
                .iter()
                .map(|f| H256::from_slice(f.as_slice()))
                .collect::<Vec<H256>>();

            pending_transactions_sender
                .send((pending_tx_hash, order_ids))
                .await?;
        }
    }

    Ok(())
}

pub fn filter_orders_at_execution_price() {}

pub async fn evaluate_and_execute_orders<M: 'static + Middleware>(
    affected_markets: HashSet<U256>,
    market_to_affected_orders: Arc<Mutex<HashMap<U256, HashSet<H256>>>>,
    active_orders: Arc<Mutex<HashMap<H256, Order>>>,
    markets: Arc<Mutex<HashMap<U256, Market>>>,
    configuration: &config::Config,
    middleware: Arc<M>,
    pending_transactions_sender: Arc<tokio::sync::mpsc::Sender<(H256, Vec<H256>)>>,
) -> Result<(), ExecutorError<M>> {
    //:: Acquire the lock on all of the data structures that have a mutex
    let market_to_affected_orders = market_to_affected_orders
        .lock()
        .expect("Could not acquire mutex lock");
    let markets = markets.lock().expect("Could not acquire mutex lock");
    let active_orders = active_orders.lock().expect("Could not acquire mutex lock");

    //:: Initialize a new structure to hold a clone of the current state of the markets.
    //:: This will allow you to simulate order execution and mutate the simluated markets without having to change/unwind the market state.
    let mut simulated_markets: HashMap<U256, Market> = HashMap::new();

    //:: group all of the orders that are ready to execute and separate them by sandbox limit orders and limit orders
    //Accumulate sandbox limit orders at execution price
    let mut slo_at_execution_price: HashMap<H256, &SandboxLimitOrder> = HashMap::new();
    //Accumulate limit orders at execution price
    let mut lo_at_execution_price: HashMap<H256, &LimitOrder> = HashMap::new();

    //:: Get to each order in the affected orders, check if they are ready for execution and then add them to the data structures mentioned above, which will then be used to simulate orders and generate execution calldata.
    for market_id in affected_markets {
        if let Some(affected_orders) = market_to_affected_orders.get(&market_id) {
            for order_id in affected_orders {
                if let Some(order) = active_orders.get(order_id) {
                    if order.can_execute(&markets, configuration.weth_address) {
                        //Add the market to the simulation markets structure
                        simulated_markets.insert(
                            market_id,
                            markets
                                .get(&market_id)
                                .expect("Could not get market from markets")
                                .clone(),
                        );

                        match order {
                            Order::SandboxLimitOrder(sandbox_limit_order) => {
                                if slo_at_execution_price
                                    .get(&sandbox_limit_order.order_id)
                                    .is_none()
                                {
                                    slo_at_execution_price
                                        .insert(sandbox_limit_order.order_id, sandbox_limit_order);
                                }
                            }

                            Order::LimitOrder(limit_order) => {
                                if lo_at_execution_price.get(&limit_order.order_id).is_none() {
                                    lo_at_execution_price.insert(limit_order.order_id, limit_order);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    //:: Simulate sandbox limit orders and generate execution transaction calldata
    //simulate and batch sandbox limit orders
    // simulate::simulate_and_batch_sandbox_limit_orders(
    //     slo_at_execution_price,
    //     simulated_markets,
    //     v3_quoter_address,
    //     middleware,
    // )
    // .await?;

    //simulate and batch limit orders

    //:: Simulate sandbox limit orders and generate execution transaction calldata
    let limit_order_execution_bundle = simulate::simulate_and_batch_limit_orders(
        lo_at_execution_price,
        &mut simulated_markets,
        configuration.weth_address,
        middleware.clone(),
    )
    .await?;

    //execute sandbox limit orders

    //execute  limit orders
    execute_limit_order_groups(
        limit_order_execution_bundle,
        configuration,
        pending_transactions_sender,
        middleware,
    )
    .await?;
    Ok(())
}
