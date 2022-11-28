use std::{
    collections::{hash_map::DefaultHasher, HashMap, HashSet},
    hash::{Hash, Hasher},
    sync::{Arc, Mutex},
};

use ethers::{
    abi::{decode, ParamType},
    providers::{JsonRpcClient, Provider},
    types::{Log, H160, H256},
};
use pair_sync::{dex::Dex, pool::Pool};

use crate::{
    error::BeltError,
    orders::order::{self, Order},
};

pub fn get_market_id(token_a: H160, token_b: H160) -> u64 {
    let mut hasher = DefaultHasher::new();

    if token_a > token_b {
        token_a.hash(&mut hasher);
        hasher.finish()
    } else {
        token_b.hash(&mut hasher);
        hasher.finish()
    }
}

//TODO: update this comment with proper docs
//Returns pool addr to market id markets, market to affected orders,
pub async fn initialize_market_structures<P: 'static + JsonRpcClient>(
    active_orders: Arc<Mutex<HashMap<H256, Order>>>,
    dexes: &[Dex],
    weth: H160,
    provider: Arc<Provider<P>>,
) -> Result<
    (
        HashMap<H160, u64>,
        Arc<Mutex<HashMap<u64, HashMap<H160, Pool>>>>,
        Arc<Mutex<HashMap<u64, HashSet<H256>>>>,
    ),
    BeltError<P>,
> {
    let mut pool_address_to_market_id: HashMap<H160, u64> = HashMap::new();
    let mut market_initialized: HashSet<u64> = HashSet::new();
    let mut markets: HashMap<u64, HashMap<H160, Pool>> = HashMap::new();
    let mut market_to_affected_orders: HashMap<u64, HashSet<H256>> = HashMap::new();

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

async fn update_market_structures<P: 'static + JsonRpcClient>(
    order_id: H256,
    token_a: H160,
    token_b: H160,
    pool_address_to_market_id: &mut HashMap<H160, u64>,
    market_initialized: &mut HashSet<u64>,
    markets: &mut HashMap<u64, HashMap<H160, Pool>>,
    market_to_affected_orders: &mut HashMap<u64, HashSet<H256>>,
    dexes: &[Dex],
    provider: Arc<Provider<P>>,
) -> Result<(), BeltError<P>> {
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

async fn get_market<P: 'static + JsonRpcClient>(
    token_a: H160,
    token_b: H160,
    provider: Arc<Provider<P>>,
    dexes: &[Dex],
) -> Result<Option<HashMap<H160, Pool>>, BeltError<P>> {
    let mut market = HashMap::new();

    for dex in dexes {
        if let Some(pool) = dex
            .get_pool_with_best_liquidity(token_a, token_b, provider.clone())
            .await?
        {}
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
    pool_address_to_market_id: &HashMap<H160, u64>,
    markets: Arc<Mutex<HashMap<u64, HashMap<H160, Pool>>>>,
) -> HashSet<u64> {
    let mut markets_updated: HashSet<u64> = HashSet::new();

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
