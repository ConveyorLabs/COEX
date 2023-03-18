use std::{collections::HashMap, sync::Arc};

use cfmms::pool::{Pool, UniswapV2Pool};
use ethers::{
    providers::Middleware,
    types::{H160, U256},
};
use futures::future::join_all;

use crate::{
    abi::IUniswapV3Quoter,
    error::ExecutorError,
    markets::{self, Market},
};

use crate::order::sandbox_limit_order::SandboxLimitOrder;

pub async fn find_best_a_to_x_to_b_route<M: 'static + Middleware>(
    token_in: H160,
    x_token: H160,
    token_out: H160,
    amount_in: U256,
    simulated_markets: &HashMap<U256, HashMap<H160, Pool>>,
    middleware: Arc<M>,
) -> Result<(Vec<U256>, Vec<U256>, Vec<Pool>), ExecutorError<M>> {
    let markets_in_route: Vec<&Market> = {
        // Simulate order along route for token_a -> weth -> token_b
        let a_to_x_market = simulated_markets.get(&markets::get_market_id(token_in, x_token));
        let x_to_b_market = simulated_markets.get(&markets::get_market_id(x_token, token_out));
        if a_to_x_market.is_some() && x_to_b_market.is_some() {
            let a_to_x_market = a_to_x_market.unwrap();
            let x_to_b_market = x_to_b_market.unwrap();

            vec![a_to_x_market, x_to_b_market]
        } else if a_to_x_market.is_none() {
            return Err(ExecutorError::MarketDoesNotExistForPair(token_in, x_token));
        } else {
            //x to b market is none
            return Err(ExecutorError::MarketDoesNotExistForPair(x_token, token_out));
        }
    };

    find_best_route_across_markets(amount_in, token_in, markets_in_route, middleware.clone()).await
}

pub async fn find_best_a_to_b_route<M: 'static + Middleware>(
    token_in: H160,
    token_out: H160,
    amount_in: U256,
    simulated_markets: &mut HashMap<U256, HashMap<H160, Pool>>,
    middleware: Arc<M>,
) -> Result<(Vec<U256>, Vec<U256>, Vec<Pool>), ExecutorError<M>> {
    // Simulate order along route for token_a -> weth -> token_b
    if let Some(a_to_b_market) = simulated_markets.get(&markets::get_market_id(token_in, token_out))
    {
        Ok(find_best_route_across_markets(
            amount_in,
            token_in,
            vec![a_to_b_market],
            middleware.clone(),
        )
        .await?)
    } else {
        Ok((
            vec![U256::zero()],
            vec![U256::zero()],
            vec![Pool::UniswapV2(UniswapV2Pool::default())],
        ))
    }
}

pub const V3_QUOTER_ADDRESS: H160 = H160([
    178, 115, 8, 249, 249, 13, 96, 116, 99, 187, 51, 234, 27, 235, 180, 28, 39, 206, 90, 182,
]);

//Returns the amounts in, amount out and a reference to the pools that it took through the route
pub async fn find_best_route_across_markets<M: 'static + Middleware>(
    amount_in: U256,
    mut token_in: H160,
    markets: Vec<&Market>,
    middleware: Arc<M>,
) -> Result<(Vec<U256>, Vec<U256>, Vec<Pool>), ExecutorError<M>> {
    let mut amount_in = amount_in;
    let mut amounts_in: Vec<U256> = vec![];
    let mut amounts_out: Vec<U256> = vec![];
    let mut route: Vec<Pool> = vec![];

    for market in markets {
        //TODO: apply tax to amount in
        let mut best_amount_out = U256::zero();
        let mut best_pool = Pool::UniswapV2(UniswapV2Pool::default());

        amounts_in.push(amount_in);

        let mut handles = vec![];

        for pool in market.values() {
            let pool = *pool;
            match pool {
                Pool::UniswapV2(_) => {
                    let swap_amount_out = pool
                        .simulate_swap(token_in, amount_in, middleware.clone())
                        .await?;
                    if swap_amount_out > best_amount_out {
                        best_amount_out = swap_amount_out;
                        best_pool = pool;
                    }
                }

                Pool::UniswapV3(uniswap_v3_pool) => {
                    let uniswap_v3_quoter =
                        IUniswapV3Quoter::new(V3_QUOTER_ADDRESS, middleware.clone());

                    let (token_in, token_out) = if token_in == uniswap_v3_pool.token_a {
                        (uniswap_v3_pool.token_a, uniswap_v3_pool.token_b)
                    } else {
                        (uniswap_v3_pool.token_b, uniswap_v3_pool.token_a)
                    };

                    handles.push(tokio::spawn(async move {
                        let swap_amount_out = uniswap_v3_quoter
                            .quote_exact_input_single(
                                token_in,
                                token_out,
                                pool.fee(),
                                amount_in,
                                U256::zero(),
                            )
                            .call()
                            .await?;
                        Result::<(U256, Pool), ExecutorError<M>>::Ok((swap_amount_out, pool))
                    }))
                }
            };
        }

        for join_result in join_all(handles).await {
            match join_result {
                Ok(ok) => {
                    if let Ok((swap_amount_out, pool)) = ok {
                        if swap_amount_out > best_amount_out {
                            best_amount_out = swap_amount_out;
                            best_pool = pool;
                        }
                    }
                }
                Err(_err) => {}
            }
        }

        amount_in = best_amount_out;
        amounts_out.push(best_amount_out);
        route.push(best_pool);

        //update token in
        //Get the token out from the market to set as the new token in, we can use any pool in the market since the token out and token in for each pool in the market are the same.
        // Have the same token in and out to be in the same market.
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

    Ok((amounts_in, amounts_out, route))
}

//Returns the weth exit amount in, weth amount out and the weth pool
pub async fn find_best_weth_exit_from_route<M: 'static + Middleware>(
    order: &SandboxLimitOrder,
    amount_due_to_owner: U256,
    route: Vec<Pool>,
    markets: &mut HashMap<U256, Market>,
    weth: H160,
    middleware: Arc<M>,
) -> Result<(U256, U256, Pool), ExecutorError<M>> {
    let mut swap_token = order.token_in;
    let mut swap_amount = U256::from(order.amount_in_remaining);
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

        //TODO: handle errors gracefully
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

    //Find best token out to weth pool
    let (_, amounts_out, route) = find_best_a_to_b_route(
        order.token_out,
        weth,
        swap_amount - amount_due_to_owner,
        &mut markets,
        middleware.clone(),
    )
    .await?;

    Ok((
        swap_amount - amount_due_to_owner,
        *amounts_out.last().unwrap(),
        *route.last().unwrap(),
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

//TODO: take a look at this and update as needed
//TODO: handle errors gracefully

pub async fn update_pools_along_route_with_weth_exit<M: Middleware>(
    order: &SandboxLimitOrder,
    amount_in_to_weth_exit: U256,
    route: Vec<Pool>,
    markets: &mut HashMap<U256, Market>,
    weth: H160,
    weth_exit_address: H160,
    middleware: Arc<M>,
) -> Result<(), ExecutorError<M>> {
    let mut swap_token = order.token_in;
    let mut swap_amount = U256::from(order.amount_in_remaining);

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

    let token_out_to_weth_market_id = markets::get_market_id(order.token_out, weth);

    markets
        .get_mut(&token_out_to_weth_market_id)
        .unwrap()
        .get_mut(&weth_exit_address)
        .unwrap()
        .simulate_swap_mut(order.token_out, amount_in_to_weth_exit, middleware.clone())
        .await?;

    Ok(())
}
