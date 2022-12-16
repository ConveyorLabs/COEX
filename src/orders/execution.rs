use std::{
    collections::{HashMap, HashSet},
    future::pending,
    str::FromStr,
    sync::{Arc, Mutex},
};

use ethers::{
    abi::{ethabi::Bytes, Token},
    providers::{JsonRpcClient, Middleware, Provider},
    signers::LocalWallet,
    types::{
        transaction::eip2718::TypedTransaction, Eip1559TransactionRequest, TransactionRequest,
        H160, H256, U256,
    },
};

use crate::{
    abi,
    config::{self, Chain},
    error::ExecutorError,
    markets::market::{get_market_id, Market},
};

use super::{
    execution,
    limit_order::LimitOrder,
    order::{self, Order},
    sandbox_limit_order::SandboxLimitOrder,
    simulate,
};

pub trait ExecutionCalldata {
    fn to_bytes(&self) -> Bytes;
}

pub struct SandboxLimitOrderExecutionCalldata {
    pub order_id_bundles: Vec<Vec<H256>>, //bytes32[][] orderIdBundles
    pub fill_amounts: Vec<u128>,          // uint128[] fillAmounts
    pub transfer_addresses: Vec<H160>,    // address[] transferAddresses
    pub calls: Vec<Call>,                 // Call[] calls
}

pub struct Call {
    pub target: H160,       // address target
    pub call_data: Vec<u8>, // bytes callData
}

#[derive(Default)]
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
        self.order_groups
            .push(LimitOrderExecutionOrderIds::default());
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

#[derive(Default)]
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

pub async fn fill_orders_at_execution_price<P: 'static + JsonRpcClient>(
    active_orders: Arc<Mutex<HashMap<H256, Order>>>,
    markets: Arc<Mutex<HashMap<U256, Market>>>,
    configuration: &config::Config,
    pending_transactions_sender: Arc<tokio::sync::mpsc::Sender<(H256, Vec<H256>)>>,
    provider: Arc<Provider<P>>,
) -> Result<(), ExecutorError<P>> {
    //:: Get to each order in the affected orders, check if they are ready for execution and then add them to the data structures mentioned above, which will then be used to simulate orders and generate execution calldata.
    let markets = markets.lock().expect("Could not acquire mutex lock");
    let active_orders = active_orders.lock().expect("Could not acquire mutex lock");
    //NOTE: remove this note with a better comment
    //Clone the markets to simulate all active orders, only do this on initialization, this would be heavy on every time checking order execution, select simulated markets instead
    let simulated_markets = markets.clone();

    //:: group all of the orders that are ready to execute and separate them by sandbox limit orders and limit orders
    //Accumulate sandbox limit orders at execution price
    let mut slo_at_execution_price: HashMap<H256, &SandboxLimitOrder> = HashMap::new();
    //Accumulate limit orders at execution price
    let mut lo_at_execution_price: HashMap<H256, &LimitOrder> = HashMap::new();

    for order in active_orders.values() {
        if order.can_execute(&markets, configuration.weth_address) {
            //Add the market to the simulation markets structure

            match order {
                Order::SandboxLimitOrder(sandbox_limit_order) => {
                    if let None = slo_at_execution_price.get(&sandbox_limit_order.order_id) {
                        slo_at_execution_price
                            .insert(sandbox_limit_order.order_id, sandbox_limit_order);
                    }
                }

                Order::LimitOrder(limit_order) => {
                    if let None = lo_at_execution_price.get(&limit_order.order_id) {
                        lo_at_execution_price.insert(limit_order.order_id, limit_order);
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
    //     provider,
    // )
    // .await?;

    //simulate and batch limit orders
    //:: Simulate sandbox limit orders and generate execution transaction calldata
    let limit_order_execution_bundle = simulate::simulate_and_batch_limit_orders(
        lo_at_execution_price,
        simulated_markets,
        configuration.weth_address,
    );

    //execute sandbox limit orders

    //execute  limit orders
    for order_group in limit_order_execution_bundle.order_groups {
        let tx = construct_lo_execution_transaction(
            &configuration,
            vec![order_group.order_ids.clone()[0]],
            provider.clone(),
        )
        .await?;

        //TODO: simulate tx
        let limit_order_router =
            abi::ILimitOrderRouter::new(configuration.limit_order_book, provider.clone());

        println!("presimulation");

        // //TODO: dont clone order group
        // limit_order_router
        //     .execute_limit_orders(order_group.order_ids.clone())
        //     .call()
        //     .await?;

        println!("postsimulation");

        //TODO: sign the tx
        let tx_signature = configuration.wallet_key.sign_transaction_sync(&tx);
        let signed_tx_bytes = tx.rlp_signed(&tx_signature);

        //Send the tx
        let pending_tx = provider.send_raw_transaction(signed_tx_bytes).await?;

        println!("pending tx: {:?}", pending_tx.tx_hash());
        let order_ids = order_group
            .order_ids
            .iter()
            .map(|f| H256::from_slice(f.as_slice()))
            .collect::<Vec<H256>>();

        pending_transactions_sender
            .send((pending_tx.tx_hash(), order_ids))
            .await?;
    }

    println!("done");
    Ok(())
}

//Construct a sandbox limit order execution transaction
pub async fn construct_slo_execution_transaction<P: 'static + JsonRpcClient>(
    execution_address: H160,
    data: Bytes,
    provider: Arc<Provider<P>>,
    chain: &Chain,
) -> Result<TypedTransaction, ExecutorError<P>> {
    //TODO: For the love of god, refactor the transaction composition

    match chain {
        //:: EIP 1559 transaction
        Chain::Ethereum | Chain::Polygon | Chain::Optimism | Chain::Arbitrum => {
            let tx = Eip1559TransactionRequest::new()
                .to(execution_address)
                .data(data);

            //Update transaction gas fees
            let (max_priority_fee_per_gas, max_fee_per_gas) =
                provider.estimate_eip1559_fees(None).await?;
            let tx = tx.max_priority_fee_per_gas(max_priority_fee_per_gas);
            let tx = tx.max_fee_per_gas(max_fee_per_gas);

            let mut tx: TypedTransaction = tx.into();
            let gas_limit = provider.estimate_gas(&tx).await?;
            tx.set_gas(gas_limit);

            Ok(tx)
        }

        //:: Legacy transaction
        Chain::BSC | Chain::Cronos => {
            let tx = TransactionRequest::new().to(execution_address).data(data);

            let gas_price = provider.get_gas_price().await?;
            let tx = tx.gas_price(gas_price);

            let mut tx: TypedTransaction = tx.into();
            let gas_limit = provider.estimate_gas(&tx).await?;
            tx.set_gas(gas_limit);

            Ok(tx)
        }
    }
}

//TODO: change this to construct execution transaction, pass in calldata and execution address,
//TODO: this way we can simulate the tx with the same contract instance that made the calldata
//Construct a limit order execution transaction
pub async fn construct_lo_execution_transaction<P: 'static + JsonRpcClient>(
    configuration: &config::Config,
    order_ids: Vec<[u8; 32]>,
    provider: Arc<Provider<P>>,
) -> Result<TypedTransaction, ExecutorError<P>> {
    //TODO: For the love of god, refactor the transaction composition

    for order_id in order_ids.clone() {
        //TODO: remove this
        println!("{:?}", H256::from(order_id));
    }

    let calldata = abi::ILimitOrderRouter::new(configuration.limit_order_book, provider.clone())
        .execute_limit_orders(order_ids)
        .calldata()
        .unwrap();

    let nonce = provider
        .get_transaction_count(configuration.wallet_address, None)
        .await?;

    match configuration.chain {
        //:: EIP 1559 transaction
        Chain::Ethereum | Chain::Polygon | Chain::Optimism | Chain::Arbitrum => {
            //TODO:FIXME: need to make chainid dynamic, add impl for Chain type to get id
            let tx = Eip1559TransactionRequest::new()
                .to(configuration.limit_order_book)
                .data(calldata)
                .from(configuration.wallet_address)
                .nonce(nonce)
                .chain_id(137);

            //Update transaction gas fees
            let (max_fee_per_gas, max_priority_fee_per_gas) =
                provider.estimate_eip1559_fees(None).await?;

            let tx = tx.max_fee_per_gas(max_fee_per_gas * 120 / 100);
            let tx = tx.max_priority_fee_per_gas(max_priority_fee_per_gas * 120 / 100);

            println!("tx: {:?}", tx.data);

            let mut tx: TypedTransaction = tx.into();
            let gas_limit = provider.estimate_gas(&tx).await?;

            //TODO: need to transform gas limit to adjust for price * buffer?
            tx.set_gas(gas_limit);

            Ok(tx)
        }

        //:: Legacy transaction
        Chain::BSC | Chain::Cronos => {
            let tx = TransactionRequest::new()
                .to(configuration.limit_order_book)
                .data(calldata);

            let gas_price = provider.get_gas_price().await?;
            let tx = tx.gas_price(gas_price);

            let mut tx: TypedTransaction = tx.into();
            let gas_limit = provider.estimate_gas(&tx).await?;
            tx.set_gas(gas_limit);

            Ok(tx)
        }
    }
}

pub async fn evaluate_and_execute_orders<P: 'static + JsonRpcClient>(
    affected_markets: HashSet<U256>,
    market_to_affected_orders: Arc<Mutex<HashMap<U256, HashSet<H256>>>>,
    active_orders: Arc<Mutex<HashMap<H256, Order>>>,
    markets: Arc<Mutex<HashMap<U256, Market>>>,
    configuration: &config::Config,
    provider: Arc<Provider<P>>,
    pending_transactions_sender: Arc<tokio::sync::mpsc::Sender<(H256, Vec<H256>)>>,
) -> Result<(), ExecutorError<P>> {
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
                if let Some(order) = active_orders.get(&order_id) {
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
                                if let None =
                                    slo_at_execution_price.get(&sandbox_limit_order.order_id)
                                {
                                    slo_at_execution_price
                                        .insert(sandbox_limit_order.order_id, sandbox_limit_order);
                                }
                            }

                            Order::LimitOrder(limit_order) => {
                                if let None = lo_at_execution_price.get(&limit_order.order_id) {
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
    //     provider,
    // )
    // .await?;

    //simulate and batch limit orders

    //:: Simulate sandbox limit orders and generate execution transaction calldata
    let limit_order_execution_bundle = simulate::simulate_and_batch_limit_orders(
        lo_at_execution_price,
        simulated_markets,
        configuration.weth_address,
    );

    //execute sandbox limit orders

    //execute  limit orders
    for order_group in limit_order_execution_bundle.order_groups {
        let tx = construct_lo_execution_transaction(
            &configuration,
            order_group.order_ids.clone(),
            provider.clone(),
        )
        .await?;

        //TODO: simulate the tx

        //TODO: sign tx
        let signed_tx = configuration.wallet_key.sign_transaction_sync(&tx);

        //Send the tx
        let pending_tx = provider
            .send_raw_transaction(signed_tx.to_vec().into())
            .await?;

        let order_ids = order_group
            .order_ids
            .iter()
            .map(|f| H256::from_slice(f.as_slice()))
            .collect::<Vec<H256>>();

        pending_transactions_sender
            .send((pending_tx.tx_hash(), order_ids))
            .await?;
    }

    Ok(())
}
