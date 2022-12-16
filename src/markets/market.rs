use std::{
    collections::{hash_map::DefaultHasher, HashMap, HashSet},
    hash::{Hash, Hasher},
    sync::{Arc, Mutex},
};

use cfmms::{dex::Dex, pool::Pool};
use ethers::{
    abi::{decode, token, ParamType},
    prelude::{k256::elliptic_curve::bigint::Encoding, NonceManagerMiddleware},
    providers::{JsonRpcClient, Middleware, Provider},
    types::{Log, H160, H256, U256},
    utils::keccak256,
};

use crate::{
    error::ExecutorError,
    orders::order::{self, Order},
};

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

//Returns pool addr to market id, markets, market to affected orders,
pub async fn initialize_market_structures<P: 'static + JsonRpcClient, M: Middleware>(
    active_orders: Arc<Mutex<HashMap<H256, Order>>>,
    dexes: &[Dex],
    weth: H160,
    middleware: Arc<NonceManagerMiddleware<Provider<P>>>,
) -> Result<
    (
        HashMap<H160, U256>,
        Arc<Mutex<HashMap<U256, HashMap<H160, Pool>>>>,
        Arc<Mutex<HashMap<U256, HashSet<H256>>>>,
    ),
    ExecutorError<P, M>,
> {
    let mut pool_address_to_market_id: HashMap<H160, U256> = HashMap::new();
    let mut market_initialized: HashSet<U256> = HashSet::new();
    let mut markets: HashMap<U256, HashMap<H160, Pool>> = HashMap::new();
    let mut market_to_affected_orders: HashMap<U256, HashSet<H256>> = HashMap::new();

    for (_, order) in active_orders
        .lock()
        .expect("Could not acquire lock on active orders")
        .iter()
    {
        match order {
            Order::SandboxLimitOrder(sandbox_limit_order) => {
                //Update for token a -> token b market
                update_market_structures(
                    sandbox_limit_order.order_id,
                    sandbox_limit_order.token_in,
                    sandbox_limit_order.token_out,
                    &mut pool_address_to_market_id,
                    &mut market_initialized,
                    &mut markets,
                    &mut market_to_affected_orders,
                    &dexes,
                    provider.clone(),
                )
                .await?;

                //Update for token a -> weth market
                update_market_structures(
                    sandbox_limit_order.order_id,
                    sandbox_limit_order.token_in,
                    weth,
                    &mut pool_address_to_market_id,
                    &mut market_initialized,
                    &mut markets,
                    &mut market_to_affected_orders,
                    &dexes,
                    provider.clone(),
                )
                .await?;

                //Update for token b -> weth market
                update_market_structures(
                    sandbox_limit_order.order_id,
                    sandbox_limit_order.token_out,
                    weth,
                    &mut pool_address_to_market_id,
                    &mut market_initialized,
                    &mut markets,
                    &mut market_to_affected_orders,
                    &dexes,
                    provider.clone(),
                )
                .await?;
            }
            Order::LimitOrder(limit_order) => {
                //Update for token a -> weth market
                update_market_structures(
                    limit_order.order_id,
                    limit_order.token_in,
                    weth,
                    &mut pool_address_to_market_id,
                    &mut market_initialized,
                    &mut markets,
                    &mut market_to_affected_orders,
                    &dexes,
                    provider.clone(),
                )
                .await?;

                //Update for token b -> weth market
                update_market_structures(
                    limit_order.order_id,
                    limit_order.token_out,
                    weth,
                    &mut pool_address_to_market_id,
                    &mut market_initialized,
                    &mut markets,
                    &mut market_to_affected_orders,
                    &dexes,
                    provider.clone(),
                )
                .await?;
            }
        }
    }

    Ok((
        pool_address_to_market_id,
        Arc::new(Mutex::new(markets)),
        Arc::new(Mutex::new(market_to_affected_orders)),
    ))
}

async fn update_market_structures<P: 'static + JsonRpcClient, M: Middleware>(
    order_id: H256,
    token_a: H160,
    token_b: H160,
    pool_address_to_market_id: &mut HashMap<H160, U256>,
    market_initialized: &mut HashSet<U256>,
    markets: &mut HashMap<U256, HashMap<H160, Pool>>,
    market_to_affected_orders: &mut HashMap<U256, HashSet<H256>>,
    dexes: &[Dex],
    middleware: Arc<NonceManagerMiddleware<Provider<P>>>,
) -> Result<(), ExecutorError<P, M>> {
    //Initialize a to b market
    let market_id = get_market_id(token_a, token_b);
    if market_initialized.get(&market_id).is_some() {
        if let Some(order_ids) = market_to_affected_orders.get_mut(&market_id) {
            order_ids.insert(order_id);
        }
    } else {
        market_initialized.insert(market_id);

        if let Some(market) = get_market(token_a, token_b, provider.clone(), dexes).await? {
            for (pool_address, _) in &market {
                pool_address_to_market_id.insert(pool_address.to_owned(), market_id);
            }

            markets.insert(market_id, market);

            let mut order_ids = HashSet::new();
            order_ids.insert(order_id);
            market_to_affected_orders.insert(market_id, order_ids);
        }
    }

    Ok(())
}

async fn get_market<P: 'static + JsonRpcClient, M: Middleware>(
    token_a: H160,
    token_b: H160,
    provider: Arc<Provider<P>>,
    dexes: &[Dex],
) -> Result<Option<HashMap<H160, Pool>>, ExecutorError<P, M>> {
    let mut market = HashMap::new();

    for dex in dexes {
        if let Some(pools) = dex
            .get_all_pools_for_pair(token_a, token_b, provider.clone())
            .await?
        {
            for pool in pools {
                market.insert(pool.address(), pool);
            }
        }
    }

    if market.len() > 0 {
        Ok(Some(market))
    } else {
        Ok(None)
    }
}

//Returns markets affected
pub fn handle_market_updates(
    pool_events: &[Log],
    pool_address_to_market_id: &HashMap<H160, U256>,
    markets: Arc<Mutex<HashMap<U256, HashMap<H160, Pool>>>>,
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
                                &vec![
                                    ParamType::Uint(128), //reserve0
                                    ParamType::Uint(128), //reserve1
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
                                &vec![
                                    // ParamType::Address,   //sender is indexed so its not in log data
                                    // ParamType::Address,   //recipient is indexed so its not in log data
                                    ParamType::Int(256),  //amount0
                                    ParamType::Int(256),  //amount1
                                    ParamType::Uint(160), //sqrtPriceX96
                                    ParamType::Uint(128), //liquidity
                                    ParamType::Int(24),   //tick
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
    token_in: H160,
    token_out: H160,
    markets: &HashMap<U256, HashMap<H160, Pool>>,
) -> f64 {
    let mut best_price = if buy { f64::MAX } else { 0.0 };

    let market_id = get_market_id(token_in, token_out);
    if let Some(market) = markets.get(&market_id) {
        for (_, pool) in market {
            let price = pool.calculate_price(token_in);

            if buy {
                if price < best_price {
                    best_price = price;
                }
            } else {
                if price > best_price {
                    best_price = price;
                }
            }
        }
    }

    best_price
}
