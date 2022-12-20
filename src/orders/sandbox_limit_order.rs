use std::{collections::HashMap, sync::Arc};

use cfmms::pool::Pool;
use ethers::{
    providers::Middleware,
    types::{H160, H256, U256},
};
use num_bigfloat::BigFloat;

use crate::{abi, error::ExecutorError, markets::market::get_best_market_price};

//TODO: FIXME: remove the clone copy, this is not needed, only used in ~ one place, need to update to not use clone or copy
//TODO: regarding clone note, Update when refactoring the codebase
#[derive(Debug, Clone, Copy)]
pub struct SandboxLimitOrder {
    pub last_refresh_timestamp: u32,
    pub expiration_timestamp: u32,
    pub fill_percent: u128,
    pub fee_remaining: u128,
    pub amount_in_remaining: u128,
    pub amount_out_remaining: u128,
    pub price: f64,
    pub execution_credit_remaining: u128,
    pub owner: H160,
    pub token_in: H160,
    pub token_out: H160,
    pub order_id: H256,
}

impl SandboxLimitOrder {
    pub fn new(
        last_refresh_timestamp: u32,
        expiration_timestamp: u32,
        fill_percent: u128,
        fee_remaining: u128,
        amount_in_remaining: u128,
        amount_out_remaining: u128,
        price: f64,
        execution_credit_remaining: u128,
        owner: H160,
        token_in: H160,
        token_out: H160,
        order_id: H256,
    ) -> SandboxLimitOrder {
        SandboxLimitOrder {
            last_refresh_timestamp,
            expiration_timestamp,
            fill_percent,
            fee_remaining,
            amount_in_remaining,
            amount_out_remaining,
            price,
            execution_credit_remaining,
            owner,
            token_in,
            token_out,
            order_id,
        }
    }

    pub async fn new_from_return_data<M: Middleware>(
        return_data: (
            u32,
            u32,
            u128,
            u128,
            u128,
            u128,
            u128,
            H160,
            H160,
            H160,
            [u8; 32],
        ),
        middleware: Arc<M>,
    ) -> Result<SandboxLimitOrder, ExecutorError<M>> {
        let token_in_decimals = abi::IErc20::new(return_data.8, middleware.clone())
            .decimals()
            .call()
            .await?;

        let token_out_decimals = abi::IErc20::new(return_data.9, middleware.clone())
            .decimals()
            .call()
            .await?;

        let (amount_in, amount_out, _) = cfmms::pool::convert_to_common_decimals(
            U256::from(return_data.4),
            token_in_decimals,
            U256::from(return_data.5),
            token_out_decimals,
        );

        let price = BigFloat::from_u128(amount_in.as_u128())
            .div(&BigFloat::from_u128(amount_out.as_u128()))
            .to_f64();

        Ok(SandboxLimitOrder::new(
            return_data.0,
            return_data.1,
            return_data.2,
            return_data.3,
            return_data.4,
            return_data.5,
            price,
            return_data.6,
            return_data.7,
            return_data.8,
            return_data.9,
            return_data.10.into(),
        ))
    }
    pub fn can_execute(&self, markets: &HashMap<U256, HashMap<H160, Pool>>, weth: H160) -> bool {
        self.price >= self.get_best_market_price(markets, weth)
    }

    pub fn get_best_market_price(
        &self,
        markets: &HashMap<U256, HashMap<H160, Pool>>,
        weth: H160,
    ) -> f64 {
        //Check if the order is at execution price

        //Check a -> b price
        let a_to_b_price = get_best_market_price(true, self.token_in, self.token_out, markets);

        //Check a -> weth -> b price
        let a_to_weth_price = get_best_market_price(true, self.token_in, weth, markets);
        let weth_to_b_price = get_best_market_price(true, weth, self.token_out, markets);
        let a_to_weth_to_b_price = a_to_weth_price * weth_to_b_price;

        if a_to_weth_to_b_price > a_to_b_price {
            a_to_weth_to_b_price
        } else {
            a_to_b_price
        }
    }
}
