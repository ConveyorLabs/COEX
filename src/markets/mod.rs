use std::{collections::HashMap, sync::Arc};

use cfmms::{dex::Dex, pool::Pool};
use ethers::{
    providers::Middleware,
    types::{H160, U256},
    utils::keccak256,
};

use crate::error::ExecutorError;

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

pub async fn get_market<M: 'static + Middleware>(
    token_a: H160,
    token_b: H160,
    dexes: &[Dex],
    middleware: Arc<M>,
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
            let price = pool.calculate_price(base_token).unwrap_or(0.0);

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
