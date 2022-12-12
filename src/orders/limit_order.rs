use std::collections::HashMap;

use cfmms::pool::Pool;
<<<<<<< HEAD
use ethers::types::{H160, H256};
=======
use ethers::types::{H160, H256, U256};
>>>>>>> 0xKitsune/limit-order-simulation

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
    pub fn can_execute(&self, markets: &HashMap<U256, HashMap<H160, Pool>>, weth: H160) -> bool {
        self.price >= self.get_best_market_price(markets, weth)
    }

    pub fn get_best_market_price(
        &self,
        markets: &HashMap<U256, HashMap<H160, Pool>>,
        weth: H160,
    ) -> f64 {
        //TODO: need to check buy or sell on the order

        //Check a -> weth -> b price
        let a_to_weth_price = get_best_market_price(self.token_in, weth, markets);
        let weth_to_b_price = get_best_market_price(weth, self.token_out, markets);

        a_to_weth_price * weth_to_b_price
    }
}
