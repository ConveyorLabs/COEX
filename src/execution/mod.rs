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
    types::{H160, H256, U256},
};

use crate::{
    config::{self},
    error::ExecutorError,
    markets,
    order::{limit_order::LimitOrder, sandbox_limit_order::SandboxLimitOrder, Order},
    simulation, state, transactions,
};

use self::{
    limit_order::LimitOrderExecutionBundle,
    sandbox_limit_order::{execute_sandbox_limit_order_bundles, SandboxLimitOrderExecutionBundle},
};

pub trait ExecutionCalldata {
    fn to_bytes(&self) -> Bytes;
}

pub async fn fill_all_orders_at_execution_price<M: 'static + Middleware>(
    state: &state::State,
    configuration: &config::Config,
    pending_transactions_sender: Arc<tokio::sync::mpsc::Sender<(H256, Vec<H256>)>>,
    middleware: Arc<M>,
) -> Result<(), ExecutorError<M>> {
    //:: Get to each order in the affected orders, check if they are ready for execution and then add them to the data structures mentioned above, which will then be used to simulate orders and generate execution calldata.
    //NOTE: remove this note with a better comment
    //Clone the markets to simulate all active orders, only do this on initialization, this would be heavy on every time checking order execution, select simulated markets instead
    let mut simulated_markets = state.markets.clone();

    //TODO: package this in a function

    //:: group all of the orders that are ready to execute and separate them by sandbox limit orders and limit orders
    //Accumulate sandbox limit orders at execution price
    let mut slo_at_execution_price: HashMap<H256, &SandboxLimitOrder> = HashMap::new();
    //Accumulate limit orders at execution price
    let mut lo_at_execution_price: HashMap<H256, &LimitOrder> = HashMap::new();

    for order in state.active_orders.values() {
        if order.can_execute(&state.markets, configuration.weth_address) {
            if order.has_sufficient_balance(middleware.clone()).await? {
                let a_to_weth_market_id =
                    markets::get_market_id(order.token_in(), configuration.weth_address);

                if let Some(market) = state.markets.get(&a_to_weth_market_id) {
                    //Add the market to the simulation markets structure
                    simulated_markets.insert(a_to_weth_market_id, market.clone());
                }

                let weth_to_b_market_id =
                    markets::get_market_id(configuration.weth_address, order.token_out());
                if let Some(market) = state.markets.get(&weth_to_b_market_id) {
                    //Add the market to the simulation markets structure
                    simulated_markets.insert(weth_to_b_market_id, market.clone());
                }

                match order {
                    Order::SandboxLimitOrder(sandbox_limit_order) => {
                        let a_to_b_market_id =
                            markets::get_market_id(order.token_in(), order.token_out());
                        if let Some(market) = state.markets.get(&a_to_b_market_id) {
                            //Add the market to the simulation markets structure
                            simulated_markets.insert(a_to_b_market_id, market.clone());
                        }

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
    let sandbox_execution_bundles = simulation::simulate_and_batch_sandbox_limit_orders(
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
        simulation::simulate_and_batch_limit_orders(
            lo_at_execution_price,
            &mut simulated_markets,
            configuration.weth_address,
            middleware.clone(),
        )
        .await?;

    //Execute orders if there are any order groups
    if !sandbox_execution_bundles.is_empty() {
        for bundle in sandbox_execution_bundles.iter() {
            println!("Sloex bundle: {:?}", bundle.order_id_bundles);
        }

        execute_sandbox_limit_order_bundles(
            sandbox_execution_bundles,
            configuration,
            pending_transactions_sender.clone(),
            middleware.clone(),
        )
        .await?;
    }

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

pub fn group_orders_at_execution_price(
    state: &state::State,
    affected_markets: HashSet<U256>,
    weth_address: H160,
) -> (
    HashMap<U256, markets::Market>,
    HashMap<H256, &SandboxLimitOrder>,
    HashMap<H256, &LimitOrder>,
) {
    let pending_order_ids = state
        .pending_order_ids
        .lock()
        .expect("Could not acquire lock on pending_order_ids");

    let mut simulated_markets: HashMap<U256, markets::Market> = HashMap::new();
    let mut slo_at_execution_price: HashMap<H256, &SandboxLimitOrder> = HashMap::new();
    let mut lo_at_execution_price: HashMap<H256, &LimitOrder> = HashMap::new();

    for market_id in affected_markets {
        if let Some(affected_orders) = state.market_to_affected_orders.get(&market_id) {
            for order_id in affected_orders {
                if pending_order_ids.get(order_id).is_none() {
                    if let Some(order) = state.active_orders.get(order_id) {
                        if order.can_execute(&state.markets, weth_address) {
                            let a_to_weth_market_id =
                                markets::get_market_id(order.token_in(), weth_address);

                            if let Some(market) = state.markets.get(&a_to_weth_market_id) {
                                simulated_markets.insert(a_to_weth_market_id, market.clone());
                            }

                            let weth_to_b_market_id =
                                markets::get_market_id(weth_address, order.token_out());
                            if let Some(market) = state.markets.get(&weth_to_b_market_id) {
                                simulated_markets.insert(weth_to_b_market_id, market.clone());
                            }

                            match order {
                                Order::SandboxLimitOrder(sandbox_limit_order) => {
                                    let a_to_b_market_id =
                                        markets::get_market_id(order.token_in(), order.token_out());
                                    if let Some(market) = state.markets.get(&a_to_b_market_id) {
                                        simulated_markets.insert(a_to_b_market_id, market.clone());
                                    }

                                    if slo_at_execution_price
                                        .get(&sandbox_limit_order.order_id)
                                        .is_none()
                                    {
                                        slo_at_execution_price.insert(
                                            sandbox_limit_order.order_id,
                                            sandbox_limit_order,
                                        );
                                    }
                                }
                                Order::LimitOrder(limit_order) => {
                                    if lo_at_execution_price.get(&limit_order.order_id).is_none() {
                                        lo_at_execution_price
                                            .insert(limit_order.order_id, limit_order);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    (
        simulated_markets,
        slo_at_execution_price,
        lo_at_execution_price,
    )
}

pub async fn fill_orders_at_execution_price<M: 'static + Middleware>(
    configuration: &config::Config,
    state: &state::State,
    affected_markets: HashSet<U256>,
    pending_transactions_sender: Arc<tokio::sync::mpsc::Sender<(H256, Vec<H256>)>>,
    middleware: Arc<M>,
) -> Result<(), ExecutorError<M>> {
    let (mut simulated_markets, slo_at_execution_price, lo_at_execution_price) =
        group_orders_at_execution_price(state, affected_markets, configuration.weth_address);

    //Simulate sandbox limit orders and generate execution transaction calldata
    let sandbox_execution_bundles = simulation::simulate_and_batch_sandbox_limit_orders(
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
    let limit_order_execution_bundle: LimitOrderExecutionBundle =
        simulation::simulate_and_batch_limit_orders(
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
