pub mod limit_order;
pub mod sandbox_limit_order;

use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
    sync::{Arc, Mutex},
};

use ethers::{
    abi::ethabi::Bytes,
    providers::Middleware,
    types::{H256, U256},
};

use crate::{
    config::{self},
    error::ExecutorError,
    markets::market::Market,
    orders::{
        limit_order::LimitOrder, order::Order, sandbox_limit_order::SandboxLimitOrder, simulate,
    },
    transaction_utils,
};

use self::{
    limit_order::LimitOrderExecutionBundle,
    sandbox_limit_order::{execute_sandbox_limit_order_bundles, SandboxLimitOrderExecutionBundle},
};

pub trait ExecutionCalldata {
    fn to_bytes(&self) -> Bytes;
}

pub async fn fill_all_orders_at_execution_price<M: Middleware>(
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

    //Simulate sandbox limit orders and generate execution transaction calldata
    let sandbox_execution_bundles = simulate::simulate_and_batch_sandbox_limit_orders(
        slo_at_execution_price,
        &mut simulated_markets,
        configuration.weth_address,
        configuration.executor_address,
        configuration.sandbox_limit_order_router,
        configuration.wallet_address,
        middleware.clone(),
    )
    .await?;

    for bundle in sandbox_execution_bundles.iter() {
        println!("Slo multicall: {:?}", bundle);
        println!("");
        println!("");
    }
    //Execute orders if there are any order groups
    if !sandbox_execution_bundles.is_empty() {
        execute_sandbox_limit_order_bundles(
            sandbox_execution_bundles,
            configuration,
            pending_transactions_sender.clone(),
            middleware.clone(),
        )
        .await?;
    }

    println!("gothere");

    //simulate and batch limit orders
    //:: Simulate sandbox limit orders and generate execution transaction calldata
    let limit_order_execution_bundle: LimitOrderExecutionBundle =
        simulate::simulate_and_batch_limit_orders(
            lo_at_execution_price,
            &mut simulated_markets,
            configuration.weth_address,
            middleware.clone(),
        )
        .await?;

    //TODO: rename the limit order execution bundle order groups to just be execution bundles and return a vec of bundle
    //Execute orders if there are any order groups
    if !limit_order_execution_bundle.order_groups.is_empty() {
        //execute sandbox limit orders
        limit_order::execute_limit_order_groups(
            limit_order_execution_bundle,
            configuration,
            pending_transactions_sender,
            middleware.clone(),
        )
        .await?;
    }
    Ok(())
}

pub async fn fill_orders_at_execution_price<M: 'static + Middleware>(
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
        //TODO: FIXME: sanity check that the a -> b and a -> weth and weth -> markets are all covered when they need to be.

        //TODO: FIXME: For the love of god, do this ^^^^^^^^^^

        if let Some(affected_orders) = market_to_affected_orders.get(&market_id) {
            for order_id in affected_orders {
                if let Some(order) = active_orders.get(order_id) {
                    if order.can_execute(&markets, configuration.weth_address) {
                        //TODO: make sure that we are checking if the order owner has the balance somewhere

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

    //::Now we have all of the orders that are at execution price, and the markets that are affected
    //:: Next we will simulate and batch sandbox limit orders and then sim and batch limit orders, then execute slo, afterwards executing lo

    //Simulate sandbox limit orders and generate execution transaction calldata
    let sandbox_execution_bundles = simulate::simulate_and_batch_sandbox_limit_orders(
        slo_at_execution_price,
        &mut simulated_markets,
        configuration.weth_address,
        configuration.executor_address,
        configuration.sandbox_limit_order_router,
        configuration.wallet_address,
        middleware.clone(),
    )
    .await?;

    //simulate and batch limit orders
    //:: Simulate sandbox limit orders and generate execution transaction calldata
    let limit_order_execution_bundle: LimitOrderExecutionBundle =
        simulate::simulate_and_batch_limit_orders(
            lo_at_execution_price,
            &mut simulated_markets,
            configuration.weth_address,
            middleware.clone(),
        )
        .await?;

    //Execute orders if there are any order groups
    if !sandbox_execution_bundles.is_empty() {
        execute_sandbox_limit_order_bundles(
            sandbox_execution_bundles,
            configuration,
            pending_transactions_sender.clone(),
            middleware.clone(),
        )
        .await?;
    }

    //TODO: rename the limit order execution bundle order groiups to just be execution bundles and return a vec of bundle
    //Execute orders if there are any order groups
    if !limit_order_execution_bundle.order_groups.is_empty() {
        //execute sandbox limit orders
        limit_order::execute_limit_order_groups(
            limit_order_execution_bundle,
            configuration,
            pending_transactions_sender,
            middleware.clone(),
        )
        .await?;
    }
    Ok(())
}
