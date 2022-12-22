use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

use cfmms::{dex::Dex, pool::Pool};
use ethers::{
    abi::{decode, ParamType},
    providers::Middleware,
    types::{Log, H160, H256, U256},
    utils::keccak256,
};

use crate::{error::ExecutorError, initialization::State, orders::order::Order};

pub type Market = HashMap<H160, Pool>;

pub fn get_market_id(token_a: H160, token_b: H160) -> U256 {
    if token_a > token_b {
        U256::from_little_endian(&keccak256(
            vec![token_a.as_bytes(), token_b.as_bytes()].concat(),
        ))
    } else {
        U256::from_little_endian(&keccak256(
            vec![token_b.as_bytes(), token_a.as_bytes()].concat(),
        ))
    }
}

pub async fn add_order_to_markets_to_affected_orders<M: 'static + Middleware>(
    order_id: H256,
    state: &mut State,
    dexes: &[Dex],
    middleware: Arc<M>,
) -> Result<(), ExecutorError<M>> {
    let active_orders = state
        .active_orders
        .lock()
        .expect("Could not acquire lock on active orders");
    let mut markets = state
        .markets
        .lock()
        .expect("Could not acquire lock on markets");
    let mut market_to_affected_orders = state
        .market_to_affected_orders
        .lock()
        .expect("Could not acquire lock on market_to_affected_orders");

    let order = active_orders.get(&order_id).unwrap();

    //Initialize a to b market
    let market_id = get_market_id(order.token_in(), order.token_out());
    if markets.get(&market_id).is_some() {
        if let Some(order_ids) = market_to_affected_orders.get_mut(&market_id) {
            order_ids.insert(order_id);
        }
    } else {
        if let Some(market) = get_market(
            order.token_in(),
            order.token_out(),
            middleware.clone(),
            dexes,
        )
        .await?
        {
            for (pool_address, _) in &market {
                state
                    .pool_address_to_market_id
                    .insert(pool_address.to_owned(), market_id);
            }

            markets.insert(market_id, market);

            let mut order_ids = HashSet::new();
            order_ids.insert(order_id);
            market_to_affected_orders.insert(market_id, order_ids);
        }
    }

    Ok(())
}

//TODO: add helper function to add market to markets

pub async fn add_markets_for_order() {}

pub async fn add_order_to_market_state<M: 'static + Middleware>(
    order_id: H256,
    state: &mut State,
    dexes: &[Dex],
    middleware: Arc<M>,
) -> Result<(), ExecutorError<M>> {
    add_order_to_markets_to_affected_orders(order_id, state, dexes, middleware);

    Ok(())
}

async fn get_market<M: 'static + Middleware>(
    token_a: H160,
    token_b: H160,
    middleware: Arc<M>,
    dexes: &[Dex],
) -> Result<Option<HashMap<H160, Pool>>, ExecutorError<M>> {
    let mut market = HashMap::new();

    for dex in dexes {
        if let Some(pools) = dex
            .get_all_pools_for_pair(token_a, token_b, middleware.clone())
            .await?
        {
            for pool in pools {
                market.insert(pool.address(), pool);
            }
        }
    }

    if !market.is_empty() {
        Ok(Some(market))
    } else {
        Ok(None)
    }
}

//Returns markets affected
pub fn handle_market_updates(
    pool_events: &[Log],
    pool_address_to_market_id: &HashMap<H160, U256>,
    markets: Arc<Mutex<HashMap<U256, Market>>>,
) -> HashSet<U256> {
    let mut markets_updated: HashSet<U256> = HashSet::new();

    for event_log in pool_events {
        if let Some(market_id) = pool_address_to_market_id.get(&event_log.address) {
            if let Some(market) = markets
                .lock()
                .expect("Could not acquire mutex lock")
                .get(market_id)
            {
                markets_updated.insert(*market_id);

                if let Some(pool) = market.get(&event_log.address) {
                    match pool {
                        Pool::UniswapV2(mut uniswap_v2_pool) => {
                            let log_data = decode(
                                &[
                                    ParamType::Uint(128), //reserve0
                                    ParamType::Uint(128),
                                ],
                                &event_log.data,
                            )
                            .expect("Could not get log data");

                            uniswap_v2_pool.reserve_0 =
                                log_data[0].clone().into_uint().unwrap().as_u128();

                            uniswap_v2_pool.reserve_1 =
                                log_data[1].clone().into_uint().unwrap().as_u128();
                        }
                        Pool::UniswapV3(mut uniswap_v3_pool) => {
                            // decode log data, get liquidity and sqrt price
                            let log_data = decode(
                                &[
                                    ParamType::Int(256),  //amount0
                                    ParamType::Int(256),  //amount1
                                    ParamType::Uint(160), //sqrtPriceX96
                                    ParamType::Uint(128), //liquidity
                                    ParamType::Int(24),
                                ],
                                &event_log.data,
                            )
                            .expect("Could not get log data");

                            //Update the pool data
                            uniswap_v3_pool.sqrt_price =
                                log_data[2].to_owned().into_uint().unwrap();
                            uniswap_v3_pool.liquidity =
                                log_data[3].to_owned().into_uint().unwrap().as_u128();
                        }
                    }
                }
            }
        }
    }

    markets_updated
}

pub fn get_best_market_price(
    buy: bool,
    base_token: H160,
    quote_token: H160,
    markets: &HashMap<U256, HashMap<H160, Pool>>,
) -> f64 {
    let mut best_price = if buy { f64::MAX } else { 0.0 };

    let market_id = get_market_id(base_token, quote_token);
    if let Some(market) = markets.get(&market_id) {
        for (_, pool) in market {
            let price = pool.calculate_price(base_token);

            if buy {
                if price < best_price {
                    best_price = price;
                }
            } else if price > best_price {
                best_price = price;
            }
        }
    }

    best_price
}
