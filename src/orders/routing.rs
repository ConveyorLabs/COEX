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
    markets::{self, Market},
};

use super::simulate::{div_uu, mul_64_u};

pub async fn find_best_a_to_weth_to_b_route<M: Middleware>(
    token_in: H160,
    token_out: H160,
    amount_in: U256,
    weth: H160,
    simulated_markets: &mut HashMap<U256, HashMap<H160, Pool>>,
    middleware: Arc<M>,
) -> Result<(Vec<U256>, Vec<Pool>), ExecutorError<M>> {
    //:: First get the a to weth market and then get the weth to b market from the simulated markets
    //TODO: check if there is a better way than to unwrap, some markets might not have the pairing

    let markets_in_route = if token_out == weth {
        // Simulate order along route for token_a -> weth -> token_b
        let a_to_weth_market = simulated_markets
            .get(&markets::get_market_id(token_in, weth))
            .expect("Could not get token_a to weth market");

        vec![a_to_weth_market]
    } else if token_in == weth {
        let weth_to_b_market = simulated_markets
            .get(&markets::get_market_id(token_out, weth))
            .expect("Could not get weth to token_b market");

        vec![weth_to_b_market]
    } else {
        // Simulate order along route for token_a -> weth -> token_b
        let a_to_weth_market = simulated_markets
            .get(&markets::get_market_id(token_in, weth))
            .expect("Could not get token_a to weth market");

        let weth_to_b_market = simulated_markets
            .get(&markets::get_market_id(token_out, weth))
            .expect("Could not get weth to token_b market");

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
    token_in: H160,
    token_out: H160,
    amount_in: U256,
    simulated_markets: &mut HashMap<U256, HashMap<H160, Pool>>,
    middleware: Arc<M>,
) -> Result<(Vec<U256>, Vec<Pool>), ExecutorError<M>> {
    //:: First get the a to weth market and then get the weth to b market from the simulated markets
    // Simulate order along route for token_a -> weth -> token_b
    if let Some(a_to_b_market) = simulated_markets.get(&markets::get_market_id(token_in, token_out))
    {
        Ok(find_best_route_across_markets(
            U256::from(amount_in),
            token_in,
            vec![a_to_b_market],
            middleware.clone(),
        )
        .await?)
    } else {
        Err(ExecutorError::MarketDoesNotExistForPair(
            token_in, token_out,
        ))
    }
}

//Returns the amount out and a reference to the pools that it took through the route
pub async fn find_best_route_across_markets<M: Middleware>(
    amount_in: U256,
    mut token_in: H160,
    markets: Vec<&Market>,
    middleware: Arc<M>,
) -> Result<(Vec<U256>, Vec<Pool>), ExecutorError<M>> {
    let mut amount_in = amount_in;
    let mut amounts_out: Vec<U256> = vec![];
    let mut route: Vec<Pool> = vec![];

    for market in markets {
        //TODO: apply tax to amount in
        let mut best_amount_out = U256::zero();
        let mut best_pool = Pool::UniswapV2(UniswapV2Pool::default());

        for (_, pool) in market {
            let swap_amount_out = pool
                .simulate_swap(token_in, amount_in, middleware.clone())
                .await?;

            if swap_amount_out > best_amount_out {
                best_amount_out = swap_amount_out;
                best_pool = pool.clone();
            }
        }

        amount_in = best_amount_out;
        amounts_out.push(best_amount_out);
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

    Ok((amounts_out, route))
}

//Returns the weth exit amount in, weth amount out and the weth pool
pub async fn find_best_weth_exit_from_route<M: Middleware>(
    token_in: H160,
    order_amount_in_remaining: U256,
    order_amount_out_remaining: U256,
    route: Vec<Pool>,
    markets: &mut HashMap<U256, Market>,
    weth: H160,
    middleware: Arc<M>,
) -> Result<(U256, U256, Pool), ExecutorError<M>> {
    let mut swap_token = token_in;
    let mut swap_amount = order_amount_in_remaining;
    //We need to clone because we are checking the weth amount exit after the route is completed
    let mut markets = markets.clone();

    for pool in route {
        let (token_in, token_out) = match pool {
            Pool::UniswapV2(uniswap_v2_pool) => {
                if swap_token == uniswap_v2_pool.token_a {
                    (uniswap_v2_pool.token_a, uniswap_v2_pool.token_b)
                } else {
                    (uniswap_v2_pool.token_b, uniswap_v2_pool.token_a)
                }
            }

            Pool::UniswapV3(uniswap_v3_pool) => {
                if swap_token == uniswap_v3_pool.token_a {
                    (uniswap_v3_pool.token_a, uniswap_v3_pool.token_b)
                } else {
                    (uniswap_v3_pool.token_b, uniswap_v3_pool.token_a)
                }
            }
        };

        let market_id = markets::get_market_id(token_in, token_out);

        //simulate the swap and update the swap amount
        swap_amount = markets
            .get_mut(&market_id)
            .unwrap()
            .get_mut(&pool.address())
            .unwrap()
            .simulate_swap_mut(token_in, swap_amount, middleware.clone())
            .await?;

        //update token in
        swap_token = token_out;
    }

    // FIXME: we are using the mul_64_u function to calc the amount sent to the user, but in the future the contract will change
    // Where we will only calc this value on partial fills
    // Add a call to send the exact amount to the user
    let amount_due_to_owner = mul_64_u(
        div_uu(
            U256::from(order_amount_out_remaining),
            U256::from(order_amount_in_remaining),
        ),
        U256::from(order_amount_in_remaining),
    );

    //Find best token out to weth pool
    let (amounts_out, route) = find_best_a_to_b_route(
        swap_token,
        weth,
        swap_amount - amount_due_to_owner,
        &mut markets,
        middleware.clone(),
    )
    .await?;

    Ok((
        swap_amount - amount_due_to_owner,
        *amounts_out.last().unwrap(),
        route[0],
    ))
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

        let market_id = markets::get_market_id(pool_token_in, pool_token_out);
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

pub async fn update_pools_along_route_with_weth_exit<M: Middleware>(
    mut token_in: H160,
    amount_in: U256,
    markets: &mut HashMap<U256, Market>,
    route: Vec<Pool>,
    order_fill_amount: U256,
    weth: H160,
    weth_exit_address: H160,
    middleware: Arc<M>,
) -> Result<(), ExecutorError<M>> {
    let mut amount_in = amount_in;

    //TODO: conform this to find_best_weth_exit_from_route and sanity check
    for pool_in_route in route {
        let (pool_token_in, pool_token_out) = match pool_in_route {
            Pool::UniswapV2(uniswap_v2_pool) => (uniswap_v2_pool.token_a, uniswap_v2_pool.token_b),

            Pool::UniswapV3(uniswap_v3_pool) => (uniswap_v3_pool.token_a, uniswap_v3_pool.token_b),
        };

        let market_id = markets::get_market_id(pool_token_in, pool_token_out);
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

    let token_out_to_weth_market_id = markets::get_market_id(token_in, weth);
    let token_out_to_weth_pool = markets
        .get_mut(&token_out_to_weth_market_id)
        .unwrap()
        .get_mut(&weth_exit_address)
        .unwrap();

    token_out_to_weth_pool
        .simulate_swap_mut(token_in, amount_in - order_fill_amount, middleware)
        .await?;

    Ok(())
}
