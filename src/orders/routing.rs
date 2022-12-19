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
    markets::{
        self,
        market::{self, Market},
    },
};

use super::order::Order;

pub async fn find_best_a_to_weth_to_b_route<M: Middleware>(
    order: &Order,
    weth: H160,
    simulated_markets: &mut HashMap<U256, HashMap<H160, Pool>>,
    middleware: Arc<M>,
) -> Result<(U256, Vec<Pool>), ExecutorError<M>> {
    let (token_in, amount_in, token_out) = match order {
        Order::SandboxLimitOrder(slo) => (slo.token_in, slo.amount_in_remaining, slo.token_out),
        Order::LimitOrder(lo) => (lo.token_in, lo.quantity, lo.token_out),
    };

    //:: First get the a to weth market and then get the weth to b market from the simulated markets
    // Simulate order along route for token_a -> weth -> token_b
    let a_to_weth_market = simulated_markets
        .get(&market::get_market_id(token_in, weth))
        .expect("Could not get token_a to weth market");

    let weth_to_b_market = simulated_markets
        .get(&market::get_market_id(token_out, weth))
        .expect("Could not get weth to token_b market");

    let markets_in_route = if token_out == weth {
        vec![a_to_weth_market]
    } else if token_in == weth {
        vec![weth_to_b_market]
    } else {
        vec![a_to_weth_market, weth_to_b_market]
    };

    Ok(find_best_route_across_markets(
        U256::from(amount_in),
        token_in,
        markets_in_route,
        middleware.clone(),
    )
    .await?)
}

pub async fn find_best_a_to_b_route<M: Middleware>(
    order: &Order,
    simulated_markets: &mut HashMap<U256, HashMap<H160, Pool>>,
    middleware: Arc<M>,
) -> Result<(U256, Vec<Pool>), ExecutorError<M>> {
    let (token_in, amount_in, token_out) = match order {
        Order::SandboxLimitOrder(slo) => (slo.token_in, slo.amount_in_remaining, slo.token_out),
        Order::LimitOrder(lo) => (lo.token_in, lo.quantity, lo.token_out),
    };

    //:: First get the a to weth market and then get the weth to b market from the simulated markets
    // Simulate order along route for token_a -> weth -> token_b
    let a_to_b_market = simulated_markets
        .get(&market::get_market_id(token_in, token_out))
        .expect("Could not get token_a to weth market");

    Ok(find_best_route_across_markets(
        U256::from(amount_in),
        token_in,
        vec![a_to_b_market],
        middleware.clone(),
    )
    .await?)
}

//Returns the amount out and a reference to the pools that it took through the route
pub async fn find_best_route_across_markets<M: Middleware>(
    amount_in: U256,
    mut token_in: H160,
    markets: Vec<&Market>,
    middleware: Arc<M>,
) -> Result<(U256, Vec<Pool>), ExecutorError<M>> {
    let mut amount_in = amount_in;
    let mut route: Vec<Pool> = vec![];

    for market in markets {
        //TODO: apply tax to amount in
        let mut best_amount_out = U256::zero();
        let mut best_pool = Pool::UniswapV2(UniswapV2Pool::default());

        for (_, pool) in market {
            let swap_amount_out = pool
                .simulate_swap(token_in, amount_in, middleware.clone())
                .await?;

            if swap_amount_out > best_amount_out || best_amount_out == U256::zero() {
                best_amount_out = swap_amount_out;
                best_pool = pool.clone();
            }
        }

        amount_in = best_amount_out;
        route.push(best_pool);

        //update token in
        token_in = match market.values().next().unwrap() {
            Pool::UniswapV2(uniswap_v2_pool) => {
                if uniswap_v2_pool.token_a == token_in {
                    uniswap_v2_pool.token_b
                } else {
                    uniswap_v2_pool.token_a
                }
            }
            Pool::UniswapV3(uniswap_v3_pool) => {
                if uniswap_v3_pool.token_a == token_in {
                    uniswap_v3_pool.token_b
                } else {
                    uniswap_v3_pool.token_a
                }
            }
        };
    }

    Ok((amount_in, route))
}

pub async fn update_pools_along_route<M: Middleware>(
    mut token_in: H160,
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

        let market_id = markets::market::get_market_id(pool_token_in, pool_token_out);
        let pool_in_market = markets
            .get_mut(&market_id)
            .unwrap()
            .get_mut(&pool_in_route.address())
            .unwrap();

        amount_in = pool_in_market
            .simulate_swap_mut(token_in, amount_in, middleware.clone())
            .await?;

        //update token in
        token_in = match pool_in_market {
            Pool::UniswapV2(uniswap_v2_pool) => {
                if uniswap_v2_pool.token_a == token_in {
                    uniswap_v2_pool.token_b
                } else {
                    uniswap_v2_pool.token_a
                }
            }
            Pool::UniswapV3(uniswap_v3_pool) => {
                if uniswap_v3_pool.token_a == token_in {
                    uniswap_v3_pool.token_b
                } else {
                    uniswap_v3_pool.token_a
                }
            }
        };
    }

    Ok(())
}
