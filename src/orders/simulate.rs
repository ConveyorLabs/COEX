use std::{
    borrow::BorrowMut,
    collections::{HashMap, HashSet},
    sync::Arc,
};

use ethers::{
    providers::{JsonRpcClient, Provider},
    types::{H160, H256},
};
use pair_sync::pool::Pool;

use crate::markets::market::{self, get_market_id};

use super::sandbox_limit_order::SandboxLimitOrder;

//Takes a hashmap of market to sandbox limit orders that are ready to execute
pub fn simulate_and_batch_sandbox_limit_orders<P: 'static + JsonRpcClient>(
    sandbox_limit_orders: HashMap<H256, &SandboxLimitOrder>,
    mut simulated_markets: HashMap<u64, HashMap<H160, Pool>>,
    v3_quoter_address: H160,
    provider: Arc<Provider<P>>,
) {
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
                let mut best_amount_out = 0 as u128;
                let mut best_pool = &H160::zero();

                for (pool_address, pool) in simulated_market {
                    if pool.calculate_price(order.token_in) >= order.price {
                        let amount_out = match pool {
                            Pool::UniswapV2(uniswap_v2_pool) => uniswap_v2_pool
                                .simulate_swap(order.token_in, order.amount_in_remaining),
                            Pool::UniswapV3(uniswap_v3_pool) => {
                                uniswap_v3_pool
                                    .simulate_swap(
                                        order.token_in,
                                        order.amount_in_remaining,
                                        v3_quoter_address,
                                        provider,
                                    )
                                    .await?
                            }
                        };

                        if amount_out > best_amount_out {
                            best_amount_out = amount_out;
                            best_pool = pool_address;
                        }
                    }
                }
            }
        }
    }

    //When the market is tapped out for the orders, move onto the next market
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

fn simulate_sandbox_limit_order() {}
