use std::{
    borrow::BorrowMut,
    collections::{HashMap, HashSet},
    str::FromStr,
    sync::Arc,
};

use cfmms::pool::{self, Pool, UniswapV2Pool};
use ethers::{
    providers::Middleware,
    types::{H160, H256, U256},
    utils::keccak256,
};

use crate::{
    error::ExecutorError,
    markets::market::{get_market_id, Market},
    orders::{order::Order, routing},
};

use super::{
    execution::LimitOrderExecutionBundle, limit_order::LimitOrder,
    sandbox_limit_order::SandboxLimitOrder,
};

//Takes a hashmap of market to sandbox limit orders that are ready to execute
pub async fn simulate_and_batch_sandbox_limit_orders<M: Middleware>(
    sandbox_limit_orders: HashMap<H256, &SandboxLimitOrder>,
    simulated_markets: &mut HashMap<U256, HashMap<H160, Pool>>,
    weth: H160,
    middleware: Arc<M>,
) -> Result<(), ExecutorError<M>> {
    //Go through the slice of sandbox limit orders and group the orders by market
    let orders_grouped_by_market = group_sandbox_limit_orders_by_market(sandbox_limit_orders);
    let sorted_orders_grouped_by_market =
        sort_sandbox_limit_orders_by_amount_in(orders_grouped_by_market);

    //TODO: sort these by usd value in the future

    //TODO: update this comment later, but we add order ids to this hashset so that we dont recalc orders for execution viability if they are already in an order group
    // since orders can be affected by multiple markets changing, its possible that the same order is in here twice, hence why we need to check if the order is already
    // in the execution calldata

    //TODO: add equivalent
    // let mut execution_calldata = LimitOrderExecutionBundle::new();

    let mut order_ids_in_calldata: HashSet<H256> = HashSet::new();
    //For each order that can execute, add it to the execution calldata, including partial fills
    for (_market_id, orders) in sorted_orders_grouped_by_market {
        for order in orders {
            let middleware = middleware.clone();

            //:: If the order is not already added to calldata, continue simulating and checking for execution
            if order_ids_in_calldata.get(&order.order_id).is_none() {
                order_ids_in_calldata.insert(order.order_id);

                //Check if the order can execute within the updated simulated markets
                if order.can_execute(&simulated_markets, weth) {
                    let mut amount_out = U256::zero();
                    let mut route: Vec<Pool> = vec![];

                    (amount_out, route) = routing::find_best_a_to_b_route(
                        &Order::SandboxLimitOrder(order.clone()), //TODO: FIXME: going to use Order, updating order to trait, this is temporary, shield your eyes
                        simulated_markets,
                        middleware.clone(),
                    )
                    .await?;

                    let (a_weth_b_amount_out, a_weth_b_route) =
                        routing::find_best_a_to_weth_to_b_route(
                            &Order::SandboxLimitOrder(order.clone()), //TODO: FIXME: going to use Order, updating order to trait, this is temporary, shield your eyes
                            weth,
                            simulated_markets,
                            middleware.clone(),
                        )
                        .await?;

                    if a_weth_b_amount_out > amount_out {
                        amount_out = a_weth_b_amount_out;
                        route = a_weth_b_route;
                    }

                    //TODO: if the order is full filled, then update the route with the full amt, else update with a partial fill

                    //:: If that amount out is greater than or equal to the amount out min of the order update the pools along the route and add the order Id to the order group read for exectuion
                    if amount_out.as_u128() >= order.amount_out_remaining {
                        println!("ao: {}, aor: {}", amount_out, order.amount_out_remaining);
                        routing::update_pools_along_route(
                            order.token_in,
                            U256::from(order.amount_in_remaining),
                            simulated_markets,
                            route,
                            middleware,
                        )
                        .await?;

                        //:: For each order group, there is a new array that is initialized and order ids that are ready for execution are added to this array.
                        //:: Then that array is appended to the execution calldata

                        //TODO: add the data to the calldata
                    }
                }
            }
        }
    }

    //When the market is tapped out for the orders, move onto the next market

    //TODO: Return the calldata
    Ok(())
}

fn group_sandbox_limit_orders_by_market(
    limit_orders: HashMap<H256, &SandboxLimitOrder>,
) -> HashMap<U256, Vec<&SandboxLimitOrder>> {
    let mut orders_grouped_by_market: HashMap<U256, Vec<&SandboxLimitOrder>> = HashMap::new();
    for (_, order) in limit_orders {
        let market_id = get_market_id(order.token_in, order.token_out);
        if let Some(order_group) = orders_grouped_by_market.get_mut(&market_id) {
            order_group.push(order);
        } else {
            orders_grouped_by_market.insert(market_id, vec![order]);
        }
    }

    orders_grouped_by_market
}

fn sort_sandbox_limit_orders_by_amount_in(
    mut orders_grouped_by_market: HashMap<U256, Vec<&SandboxLimitOrder>>,
) -> HashMap<U256, Vec<&SandboxLimitOrder>> {
    //Go through each group of orders and sort it by amount_in
    for (_, order_group) in orders_grouped_by_market.borrow_mut() {
        order_group.sort_by(|a, b| a.amount_in_remaining.cmp(&b.amount_in_remaining))
    }
    orders_grouped_by_market
}

//Takes a hashmap of market to sandbox limit orders that are ready to execute
pub async fn simulate_and_batch_limit_orders<M: Middleware>(
    limit_orders: HashMap<H256, &LimitOrder>,
    simulated_markets: &mut HashMap<U256, HashMap<H160, Pool>>,
    weth: H160,
    middleware: Arc<M>,
) -> Result<LimitOrderExecutionBundle, ExecutorError<M>> {
    //:: First group the orders by market and sort each of the orders by the amount in (ie quantity)
    //Go through the slice of sandbox limit orders and group the orders

    let orders_grouped_by_market = group_limit_orders(limit_orders);

    let sorted_orders_grouped_by_market = sort_limit_orders_by_amount_in(orders_grouped_by_market);

    //TODO: sort these by usd value in the future

    //TODO: update this comment later, but we add order ids to this hashset so that we dont recalc orders for execution viability if they are already in an order group
    // since orders can be affected by multiple markets changing, its possible that the same order is in here twice, hence why we need to check if the order is already
    // in the execution calldata
    let mut order_ids_in_calldata: HashSet<H256> = HashSet::new();

    //:: This is a vec of order groups, ie vec of vec of bytes32
    let mut execution_calldata = LimitOrderExecutionBundle::new();
    //:: Go through each sorted order group, and simulate the order. If the order can execute, add it to the batch

    for (_, orders) in sorted_orders_grouped_by_market {
        //:: Create a new order group which will hold all the order IDs
        execution_calldata.add_empty_order_group();

        for order in orders {
            let middleware = middleware.clone();

            //:: If the order is not already added to calldata, continue simulating and checking for execution
            if order_ids_in_calldata.get(&order.order_id).is_none() {
                order_ids_in_calldata.insert(order.order_id);

                //Check if the order can execute within the updated simulated markets
                if order.can_execute(order.buy, &simulated_markets, weth) {
                    let (amount_out, route) = routing::find_best_a_to_weth_to_b_route(
                        &Order::LimitOrder(order.clone()), //TODO: FIXME: going to use Order, updating order to trait, this is temporary, shield your eyes
                        weth,
                        simulated_markets,
                        middleware.clone(),
                    )
                    .await?;

                    println!(
                        "amount out: {}, amount out min: {}",
                        amount_out, order.amount_out_min
                    );
                    //:: If that amount out is greater than or equal to the amount out min of the order update the pools along the route and add the order Id to the order group read for exectuion
                    if amount_out.as_u128() >= order.amount_out_min {
                        println!("ao: {}, aom: {}", amount_out, order.amount_out_min);
                        routing::update_pools_along_route(
                            order.token_in,
                            U256::from(order.quantity),
                            simulated_markets,
                            route,
                            middleware,
                        )
                        .await?;

                        //TODO: add the data to the calldata
                    }
                }
            }
        }
    }

    Ok(execution_calldata)
}

fn group_limit_orders(limit_orders: HashMap<H256, &LimitOrder>) -> HashMap<U256, Vec<&LimitOrder>> {
    let mut grouped_orders: HashMap<U256, Vec<&LimitOrder>> = HashMap::new();
    for (_, order) in limit_orders {
        let hash = U256::from_little_endian(&keccak256(
            vec![
                order.token_in.as_bytes(),
                order.token_out.as_bytes(),
                &order.fee_in.to_le_bytes(),
                &order.fee_out.to_le_bytes(),
                &[order.taxed as u8],
            ]
            .concat(),
        ));

        if let Some(order_group) = grouped_orders.get_mut(&hash) {
            order_group.push(order);
        } else {
            grouped_orders.insert(hash, vec![order]);
        }
    }

    grouped_orders
}

fn sort_limit_orders_by_amount_in(
    mut orders_grouped_by_market: HashMap<U256, Vec<&LimitOrder>>,
) -> HashMap<U256, Vec<&LimitOrder>> {
    //Go through each group of orders and sort it by amount_in
    for (_, order_group) in orders_grouped_by_market.borrow_mut() {
        order_group.sort_by(|a, b| a.quantity.cmp(&b.quantity))
    }

    orders_grouped_by_market
}
