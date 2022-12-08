use std::{
    borrow::BorrowMut,
    collections::{HashMap, HashSet},
    default,
    hash::{Hash, Hasher},
    sync::Arc,
};

use ethers::{
    abi::{self, ethabi::Bytes, FixedBytes, Token},
    providers::{JsonRpcClient, Provider},
    types::{H160, H256, U256},
};
use pair_sync::pool::{Pool, UniswapV2Pool};

use crate::{
    error::BeltError,
    markets::market::{get_market_id, Market},
};

use super::{limit_order::LimitOrder, sandbox_limit_order::SandboxLimitOrder};

//Takes a hashmap of market to sandbox limit orders that are ready to execute
pub async fn simulate_and_batch_sandbox_limit_orders<P: 'static + JsonRpcClient>(
    sandbox_limit_orders: HashMap<H256, &SandboxLimitOrder>,
    simulated_markets: HashMap<U256, HashMap<H160, Pool>>,
    v3_quoter_address: H160,
    provider: Arc<Provider<P>>,
) -> Result<(), BeltError<P>> {
    //Go through the slice of sandbox limit orders and group the orders by market
    let mut orders_grouped_by_market: HashMap<U256, Vec<&SandboxLimitOrder>> = HashMap::new();
    for (_, order) in sandbox_limit_orders {
        let market_id = get_market_id(order.token_in, order.token_out);
        if let Some(order_group) = orders_grouped_by_market.get_mut(&market_id) {
            order_group.push(order);
        } else {
            orders_grouped_by_market.insert(market_id, vec![order]);
        }
    }

    let sorted_orders_grouped_by_market =
        sort_sandbox_limit_orders_by_amount_in(orders_grouped_by_market);

    //For each order that can execute, add it to the execution calldata, including partial fills
    for (market_id, orders) in sorted_orders_grouped_by_market {
        for order in orders {}
    }

    //When the market is tapped out for the orders, move onto the next market

    //TODO: Return the calldata
    Ok(())
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
pub fn simulate_and_batch_limit_orders(
    limit_orders: HashMap<H256, &LimitOrder>,
    simulated_markets: HashMap<U256, HashMap<H160, Pool>>,
    weth: H160,
) -> Vec<Token> {
    //:: First group the orders by market and sort each of the orders by the amount in (ie quantity)
    //Go through the slice of sandbox limit orders and group the orders by market
    let orders_grouped_by_market = group_orders_by_market(limit_orders);
    let sorted_orders_grouped_by_market = sort_limit_orders_by_amount_in(orders_grouped_by_market);

    //TODO: update this comment later, but we add order ids to this hashset so that we dont recalc orders for execution viability if they are already in an order group
    // since orders can be affected by multiple markets changing, its possible that the same order is in here twice, hence why we need to check if the order is already
    // in the execution calldata
    let mut order_ids_in_calldata: HashSet<H256> = HashSet::new();

    let mut execution_order_groups: Vec<Token> = vec![];

    //:: Go through each sorted order group, and simulate the order. If the order can execute, add it to the batch
    for (_, orders) in sorted_orders_grouped_by_market {
        let mut order_group_to_execute: Vec<Token> = vec![];

        for order in orders {
            //:: If the order is not already added to calldata, continue simulating and checking for execution
            if let None = order_ids_in_calldata.get(&order.order_id) {
                order_ids_in_calldata.insert(order.order_id);

                //:: First get the a to weth market and then get the weth to b market from the simulated markets
                //Simulate order along route for token_a -> weth -> token_b
                let a_to_weth_market = simulated_markets
                    .get(&get_market_id(order.token_in, weth))
                    .expect("Could not get token_a to weth markets");
                let weth_to_b_market = simulated_markets
                    .get(&get_market_id(order.token_in, weth))
                    .expect("Could not get token_a to weth markets");

                //:: Then find the route that yields the best amount out across the markets
                let (amount_out, mut route) = find_best_route_across_markets(
                    order.quantity,
                    order.token_in,
                    vec![a_to_weth_market, weth_to_b_market],
                );

                //:: If that amount out is greater than or equal to the amount out min of the order update the pools along the route and add the order Id to the order group read for exectuion
                if amount_out >= order.amount_out_min {
                    update_pools_along_route(order.token_in, order.quantity, &mut route);

                    //:: For each order group, there is a new array that is intialized and order ids that are ready for execution are added to this array.
                    //:: Then that array is appended to the execution calldata
                    order_group_to_execute.push(Token::FixedBytes(FixedBytes::from(
                        order.order_id.as_bytes(),
                    )));
                }
            }
        }

        execution_order_groups.push(Token::Array(order_group_to_execute));
    }

    execution_order_groups
}

//TODO:
//Returns the amount out and a reference to the pools that it took through the route
fn find_best_route_across_markets<'a>(
    amount_in: u128,
    token_in: H160,
    markets: Vec<&Market>,
) -> (u128, Vec<&'a Pool>) {
    let mut amount_out = amount_in;
    let mut route = vec![];

    for market in markets {
        let mut best_amount_out = 0;
        let mut best_pool = &Pool::UniswapV2(UniswapV2Pool::default());

        for (_, pool) in market {
            //let swap_amount_out = pool.simulate_swap();
            // if swap_amount_out > best_amount_out {
            // best_amount_out = swap_amount_out;
            // best_pool = pool;
        }

        // amount_out = best_amount_out;
        // route.push(best_pool);
    }

    (amount_out, route)
}

//TODO:
fn update_pools_along_route(token_in: H160, amount_in: u128, route: &mut [&Pool]) {
    for pool in route {
        // pool.simulate_swap_mut(token_in, amount_in);
    }
}

fn group_orders_by_market(
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
