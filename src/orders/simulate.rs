use std::{
    borrow::BorrowMut,
    collections::{HashMap, HashSet},
    sync::Arc,
};

use ethers::{
    abi::ethabi::Bytes,
    providers::{JsonRpcClient, Provider},
    types::{H160, H256, U256},
};
use pair_sync::pool::{Pool, UniswapV2Pool};

use crate::{error::BeltError, markets::market::get_market_id};

use super::{limit_order::LimitOrder, sandbox_limit_order::SandboxLimitOrder};

//Takes a hashmap of market to sandbox limit orders that are ready to execute
pub async fn simulate_and_batch_sandbox_limit_orders<P: 'static + JsonRpcClient>(
    sandbox_limit_orders: HashMap<H256, &SandboxLimitOrder>,
    simulated_markets: HashMap<u64, HashMap<H160, Pool>>,
    v3_quoter_address: H160,
    provider: Arc<Provider<P>>,
) -> Result<(), BeltError<P>> {
    //Go through the slice of sandbox limit orders and group the orders by market
    let mut orders_grouped_by_market: HashMap<u64, Vec<&SandboxLimitOrder>> = HashMap::new();
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
        for order in orders {
            if let Some(simulated_market) = simulated_markets.get(&market_id) {
                let (best_amount_out, best_pool) = get_best_pool_for_sandbox_limit_order(
                    simulated_market,
                    order,
                    v3_quoter_address,
                    provider.clone(),
                )
                .await?;

                if best_pool.is_some() {
                    if best_amount_out.as_u128() >= order.amount_out_remaining {
                        //update reserves with simulated swap values
                        match best_pool.unwrap() {
                            //TODO: write a function to make this cleaner and easier to read
                            Pool::UniswapV2(mut uniswap_v2_pool) => {
                                if order.token_out == uniswap_v2_pool.token_b {
                                    if uniswap_v2_pool.a_to_b {
                                        uniswap_v2_pool.reserve_0 += order.amount_in_remaining
                                            - (order.amount_in_remaining
                                                * uniswap_v2_pool.fee as u128);

                                        uniswap_v2_pool.reserve_1 -= best_amount_out.as_u128();
                                    } else {
                                        uniswap_v2_pool.reserve_0 -= best_amount_out.as_u128();
                                        uniswap_v2_pool.reserve_1 += order.amount_in_remaining
                                            - (order.amount_in_remaining
                                                * uniswap_v2_pool.fee as u128);
                                    }
                                } else {
                                    if uniswap_v2_pool.a_to_b {
                                        uniswap_v2_pool.reserve_0 -= best_amount_out.as_u128();
                                        uniswap_v2_pool.reserve_1 += order.amount_in_remaining
                                            - (order.amount_in_remaining
                                                * uniswap_v2_pool.fee as u128);
                                    } else {
                                        uniswap_v2_pool.reserve_1 -= best_amount_out.as_u128();
                                        uniswap_v2_pool.reserve_0 += order.amount_in_remaining
                                            - (order.amount_in_remaining
                                                * uniswap_v2_pool.fee as u128);
                                    }
                                }
                            }

                            Pool::UniswapV3(uniswap_v3_pool) => {
                                //TODO:
                            }
                        }
                    }
                    //TODO: add the calldata to the execution calldata to fill the entire sandbox limit order
                } else {
                    //Partial fill and add the partial fill calldata to the execution calldata
                }
            }
        }
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
    mut orders_grouped_by_market: HashMap<u64, Vec<&SandboxLimitOrder>>,
) -> HashMap<u64, Vec<&SandboxLimitOrder>> {
    //Go through each group of orders and sort it by amount_in
    for (_, order_group) in orders_grouped_by_market.borrow_mut() {
        order_group.sort_by(|a, b| a.amount_in_remaining.cmp(&b.amount_in_remaining))
    }
    orders_grouped_by_market
}

//Takes a hashmap of market to sandbox limit orders that are ready to execute
pub async fn simulate_and_batch_limit_orders<P: 'static + JsonRpcClient>(
    limit_orders: HashMap<H256, &LimitOrder>,
    simulated_markets: HashMap<u64, HashMap<H160, Pool>>,
    v3_quoter_address: H160,
    provider: Arc<Provider<P>>,
) -> Result<(), BeltError<P>> {
    //Go through the slice of sandbox limit orders and group the orders by market
    let mut orders_grouped_by_market: HashMap<u64, Vec<&LimitOrder>> = HashMap::new();
    for (_, order) in limit_orders {
        let market_id = get_market_id(order.token_in, order.token_out);
        if let Some(order_group) = orders_grouped_by_market.get_mut(&market_id) {
            order_group.push(order);
        } else {
            orders_grouped_by_market.insert(market_id, vec![order]);
        }
    }

    let sorted_orders_grouped_by_market = sort_limit_orders_by_amount_in(orders_grouped_by_market);
    let execution_calldata: Vec<Bytes> = vec![];

    //Go through each sorted order group, and simulate the order. If the order can execute, add it to the batch
    for (market_id, orders) in sorted_orders_grouped_by_market {
        for order in orders {
            if let Some(simulated_market) = simulated_markets.get(&market_id) {
                //TODO: this needs to be two hop, not just one pool
                let (best_amount_out, best_pool) = get_best_pool_for_limit_order(
                    simulated_market,
                    order,
                    v3_quoter_address,
                    provider.clone(),
                )
                .await?;

                if best_pool.is_some() {
                    if best_amount_out.as_u128() >= order.amount_out_min {
                        //TODO: append calldata to execution calldata

                        //update reserves with simulated swap values
                        match best_pool.unwrap() {
                            Pool::UniswapV2(uniswap_v2_pool) => {

                                //TODO: update pool reserves
                            }
                            Pool::UniswapV3(uniswap_v2_pool) => {
                                //TODO: update pool reserves
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

fn sort_limit_orders_by_amount_in(
    mut orders_grouped_by_market: HashMap<u64, Vec<&LimitOrder>>,
) -> HashMap<u64, Vec<&LimitOrder>> {
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
