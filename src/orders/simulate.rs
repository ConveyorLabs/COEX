use std::{
    borrow::BorrowMut,
    collections::{HashMap, HashSet},
    sync::Arc,
};

use cfmms::pool::{self, Pool, UniswapV2Pool};
use ethers::{
    providers::Middleware,
    types::{H160, H256, U256},
};

use crate::{
    error::ExecutorError,
    markets::market::{get_market_id, Market},
};

use super::{
    execution::LimitOrderExecutionBundle, limit_order::LimitOrder,
    sandbox_limit_order::SandboxLimitOrder,
};

//Takes a hashmap of market to sandbox limit orders that are ready to execute
pub async fn simulate_and_batch_sandbox_limit_orders<M: 'static + Middleware>(
    sandbox_limit_orders: HashMap<H256, &SandboxLimitOrder>,
    _simulated_markets: HashMap<U256, HashMap<H160, Pool>>,
    _v3_quoter_address: H160,
    _middleware: Arc<M>,
) -> Result<(), ExecutorError<M>> {
    //Go through the slice of sandbox limit orders and group the orders by market
    let orders_grouped_by_market = group_sandbox_limit_orders_by_market(sandbox_limit_orders);
    let sorted_orders_grouped_by_market =
        sort_sandbox_limit_orders_by_amount_in(orders_grouped_by_market);

    //For each order that can execute, add it to the execution calldata, including partial fills
    for (_market_id, orders) in sorted_orders_grouped_by_market {
        for _order in orders {}
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
    mut simulated_markets: HashMap<U256, HashMap<H160, Pool>>,
    weth: H160,
    middleware: Arc<M>,
) -> Result<LimitOrderExecutionBundle, ExecutorError<M>> {
    //:: First group the orders by market and sort each of the orders by the amount in (ie quantity)
    //Go through the slice of sandbox limit orders and group the orders by market
    let orders_grouped_by_market = group_limit_orders_by_market(limit_orders);
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
                    println!(
                        "order can execute on simulated markets: {:?}",
                        order.order_id
                    );

                    //:: First get the a to weth market and then get the weth to b market from the simulated markets
                    // Simulate order along route for token_a -> weth -> token_b
                    let a_to_weth_market = simulated_markets
                        .get(&get_market_id(order.token_in, weth))
                        .expect("Could not get token_a to weth market");

                    let weth_to_b_market = simulated_markets
                        .get(&get_market_id(order.token_out, weth))
                        .expect("Could not get weth to token_b market");

                    //:: Then find the route that yields the best amount out across the markets
                    let (amount_out, route) = find_best_route_across_markets(
                        U256::from(order.quantity),
                        order.token_in,
                        vec![a_to_weth_market, weth_to_b_market],
                        middleware.clone(),
                    )
                    .await?;

                    //:: If that amount out is greater than or equal to the amount out min of the order update the pools along the route and add the order Id to the order group read for exectuion
                    if amount_out.as_u128() >= order.amount_out_min {
                        update_pools_along_route(
                            order.token_in,
                            U256::from(order.quantity),
                            &mut simulated_markets,
                            route,
                            middleware,
                        )
                        .await?;

                        //:: For each order group, there is a new array that is initialized and order ids that are ready for execution are added to this array.
                        //:: Then that array is appended to the execution calldata
                        execution_calldata.append_order_id_to_latest_order_group(order.order_id);
                    }
                }
            }
        }
    }

    Ok(execution_calldata)
}

//Returns the amount out and a reference to the pools that it took through the route
async fn find_best_route_across_markets<M: Middleware>(
    amount_in: U256,
    token_in: H160,
    markets: Vec<&Market>,
    middleware: Arc<M>,
) -> Result<(U256, Vec<Pool>), ExecutorError<M>> {
    let mut amount_out = amount_in;
    let mut route: Vec<Pool> = vec![];

    for market in markets {
        let mut best_amount_out = U256::zero();
        let mut best_pool = Pool::UniswapV2(UniswapV2Pool::default());

        for (_, pool) in market {
            let swap_amount_out = pool
                .simulate_swap(token_in, amount_out, middleware.clone())
                .await?;

            if swap_amount_out > best_amount_out || best_amount_out == U256::zero() {
                best_amount_out = swap_amount_out;
                best_pool = pool.clone();
            }
        }

        amount_out = best_amount_out;

        route.push(best_pool);
    }

    Ok((amount_out, route))
}

async fn update_pools_along_route<M: Middleware>(
    token_in: H160,
    amount_in: U256,
    markets: &mut HashMap<U256, Market>,
    route: Vec<Pool>,
    middleware: Arc<M>,
) -> Result<(), ExecutorError<M>> {
    let mut amount_in = amount_in;

    for pool_in_route in route {
        let (pool_token_in, pool_token_out) = match pool_in_route {
            Pool::UniswapV2(uniswap_v2_pool) => (uniswap_v2_pool.token_a, uniswap_v2_pool.token_b),

            Pool::UniswapV3(uniswap_v3_pool) => (uniswap_v3_pool.token_a, uniswap_v3_pool.token_b),
        };

        let market_id = get_market_id(pool_token_in, pool_token_out);
        let pool_in_market = markets
            .get_mut(&market_id)
            .unwrap()
            .get_mut(&pool_in_route.address())
            .unwrap();

        amount_in = pool_in_market
            .simulate_swap_mut(token_in, amount_in, middleware.clone())
            .await?;
    }

    Ok(())
}

fn group_limit_orders_by_market(
    limit_orders: HashMap<H256, &LimitOrder>,
) -> HashMap<U256, Vec<&LimitOrder>> {
    let mut orders_grouped_by_market: HashMap<U256, Vec<&LimitOrder>> = HashMap::new();
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

fn sort_limit_orders_by_amount_in(
    mut orders_grouped_by_market: HashMap<U256, Vec<&LimitOrder>>,
) -> HashMap<U256, Vec<&LimitOrder>> {
    //Go through each group of orders and sort it by amount_in
    for (_, order_group) in orders_grouped_by_market.borrow_mut() {
        order_group.sort_by(|a, b| a.quantity.cmp(&b.quantity))
    }

    orders_grouped_by_market
}
