use std::collections::HashMap;

use ethers::types::{H160, H256};
use pair_sync::pool::Pool;

use crate::markets::market::get_best_market_price;

#[derive(Debug)]
pub struct LimitOrder {
    pub buy: bool,
    pub taxed: bool,
    pub stop_loss: bool,
    pub last_refresh_timestamp: u32,
    pub expiration_timestamp: u32,
    pub fee_in: u32,
    pub fee_out: u32,
    pub tax_in: u16,
    pub price: f64,
    pub amount_out_min: u128,
    pub quantity: u128,
    pub execution_credit: u128,
    pub owner: H160,
    pub token_in: H160,
    pub token_out: H160,
    pub order_id: H256,
}

impl LimitOrder {
    pub fn can_execute(&self, markets: &HashMap<u64, HashMap<H160, Pool>>, weth: H160) -> bool {
        self.price >= self.get_best_market_price(markets, weth)
    }

    pub fn get_best_market_price(
        &self,
        markets: &HashMap<u64, HashMap<H160, Pool>>,
        weth: H160,
    ) -> f64 {
        //Check if the order is at execution price
        let mut best_market_price = 0.0;

        //Check a -> weth -> b price
        let a_to_weth_price = get_best_market_price(self.token_in, weth, &markets);
        let weth_to_b_price = get_best_market_price(weth, self.token_out, &markets);

        let a_to_weth_to_b_price = a_to_weth_price / weth_to_b_price;

        if a_to_weth_to_b_price > best_market_price {
            best_market_price = a_to_weth_to_b_price;
        }

        best_market_price
    }
}
