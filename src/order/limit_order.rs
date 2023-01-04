use std::collections::HashMap;

use cfmms::pool::Pool;
use ethers::types::{H160, H256, U256};
use num_bigfloat::BigFloat;

use crate::markets::get_best_market_price;

//TODO: FIXME: remove the clone copy, this is not needed, only used in ~ one place, need to update to not use clone or copy
//TODO: regarding clone note, Update when refactoring the codebase
#[derive(Debug, Clone, Copy)]
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
    pub fn new(
        buy: bool,
        taxed: bool,
        stop_loss: bool,
        last_refresh_timestamp: u32,
        expiration_timestamp: u32,
        fee_in: u32,
        fee_out: u32,
        tax_in: u16,
        price: f64,
        amount_out_min: u128,
        quantity: u128,
        execution_credit: u128,
        owner: H160,
        token_in: H160,
        token_out: H160,
        order_id: H256,
    ) -> LimitOrder {
        LimitOrder {
            buy,
            taxed,
            stop_loss,
            last_refresh_timestamp,
            expiration_timestamp,
            fee_in,
            fee_out,
            tax_in,
            price,
            amount_out_min,
            quantity,
            execution_credit,
            owner,
            token_in,
            token_out,
            order_id,
        }
    }

    pub fn new_from_return_data(
        return_data: (
            bool,
            bool,
            bool,
            u32,
            u32,
            u32,
            u32,
            u16,
            u128,
            u128,
            u128,
            u128,
            H160,
            H160,
            H160,
            [u8; 32],
        ),
    ) -> LimitOrder {
        let price = BigFloat::from_u128(return_data.8)
            .div(&BigFloat::from_f64(2_f64.powf(64_f64)).sub(&BigFloat::from(1)))
            .to_f64();

        LimitOrder::new(
            return_data.0,
            return_data.1,
            return_data.2,
            return_data.3,
            return_data.4,
            return_data.5,
            return_data.6,
            return_data.7,
            price,
            return_data.9,
            return_data.10,
            return_data.11,
            return_data.12,
            return_data.13,
            return_data.14,
            return_data.15.into(),
        )
    }

    pub fn can_execute(
        &self,
        buy: bool,
        markets: &HashMap<U256, HashMap<H160, Pool>>,
        weth: H160,
    ) -> bool {
        if buy {
            self.get_best_market_price(buy, markets, weth) <= self.price
        } else {
            self.get_best_market_price(buy, markets, weth) >= self.price
        }
    }

    pub fn get_best_market_price(
        &self,
        buy: bool,
        markets: &HashMap<U256, HashMap<H160, Pool>>,
        weth: H160,
    ) -> f64 {
        //Check a -> weth -> b price

        //We are first swapping token_a to weth, so we need the price of weth per 1 token_a
        let a_to_weth_price = get_best_market_price(buy, weth, self.token_in, markets);

        //Then we are swapping weth to token_b, meaning we need the price of 1 token_b per weth
        let weth_to_b_price = get_best_market_price(buy, self.token_out, weth, markets);

        a_to_weth_price * weth_to_b_price
    }
}
