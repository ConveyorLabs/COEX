use std::{
    borrow::BorrowMut,
    collections::{HashMap, HashSet},
    default,
    hash::{Hash, Hasher},
    sync::Arc,
};

use ethers::{
    abi::ethabi::Bytes,
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

//Returns best amount out and pool
async fn get_best_pool_for_sandbox_limit_order<'a, P: 'static + JsonRpcClient>(
    market: &'a HashMap<H160, Pool>,
    order: &'a SandboxLimitOrder,
    v3_quoter_address: H160,
    provider: Arc<Provider<P>>,
) -> Result<(U256, Option<&'a Pool>), BeltError<P>> {
    let mut best_amount_out = U256::zero();
    let mut best_pool = None;

    for (_, pool) in market {
        if pool.calculate_price(order.token_in) >= order.price {
            //simulate the swap and get the amount out
            let amount_out = match pool {
                Pool::UniswapV2(uniswap_v2_pool) => {
                    uniswap_v2_pool.simulate_swap(order.token_in, order.amount_in_remaining)
                }
                Pool::UniswapV3(uniswap_v3_pool) => {
                    uniswap_v3_pool
                        .simulate_swap(
                            order.token_in,
                            order.amount_in_remaining,
                            v3_quoter_address,
                            provider.clone(),
                        )
                        .await?
                }
            };

            if amount_out > best_amount_out {
                best_amount_out = amount_out;
                best_pool = Some(pool);
            }
        }
    }

    Ok((best_amount_out, best_pool))
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
pub async fn simulate_and_batch_limit_orders<P: 'static + JsonRpcClient>(
    limit_orders: HashMap<H256, &LimitOrder>,
    simulated_markets: HashMap<U256, HashMap<H160, Pool>>,
    v3_quoter_address: H160,
    weth: H160,
    provider: Arc<Provider<P>>,
) -> Result<(), BeltError<P>> {
    //Go through the slice of sandbox limit orders and group the orders by market
    let orders_grouped_by_market = group_orders_by_market(limit_orders);
    let sorted_orders_grouped_by_market = sort_limit_orders_by_amount_in(orders_grouped_by_market);
    // let execution_calldata: Vec<Bytes> = vec![];

    //Go through each sorted order group, and simulate the order. If the order can execute, add it to the batch
    for (_, orders) in sorted_orders_grouped_by_market {
        for order in orders {
            //Simulate order along route for token_a -> weth -> token_b

            let a_to_weth_market = simulated_markets
                .get(&get_market_id(order.token_in, weth))
                .expect("Could not get token_a to weth markets");
            let weth_to_b_market = simulated_markets
                .get(&get_market_id(order.token_in, weth))
                .expect("Could not get token_a to weth markets");

            let (amount_out, mut route) = find_best_route_across_markets(
                order.quantity,
                order.token_in,
                vec![a_to_weth_market, weth_to_b_market],
            );

            if amount_out >= order.amount_out_min {
                update_reserves_along_route(order.quantity, &mut route);
            }
        }
    }

    Ok(())
}

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

fn simulate_swap_along_route() {}

fn update_reserves_along_route(amount_in: u128, route: &mut [&Pool]) {}

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

//Returns best amount out and pool
async fn get_best_pool_for_limit_order<'a, P: 'static + JsonRpcClient>(
    market: &'a HashMap<H160, Pool>,
    order: &'a LimitOrder,
    v3_quoter_address: H160,
    provider: Arc<Provider<P>>,
) -> Result<(U256, Option<&'a Pool>), BeltError<P>> {
    let mut best_amount_out = U256::zero();
    let mut best_pool = None;

    for (_, pool) in market {
        if pool.calculate_price(order.token_in) >= order.price {
            //simulate the swap and get the amount out
            let amount_out = match pool {
                Pool::UniswapV2(uniswap_v2_pool) => {
                    uniswap_v2_pool.simulate_swap(order.token_in, order.quantity)
                }
                Pool::UniswapV3(uniswap_v3_pool) => {
                    uniswap_v3_pool
                        .simulate_swap(
                            order.token_in,
                            order.quantity,
                            v3_quoter_address,
                            provider.clone(),
                        )
                        .await?
                }
            };

            if amount_out > best_amount_out {
                best_amount_out = amount_out;
                best_pool = Some(pool);
            }
        }
    }

    Ok((best_amount_out, best_pool))
}
