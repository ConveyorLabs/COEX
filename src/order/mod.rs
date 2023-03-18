pub mod limit_order;
pub mod sandbox_limit_order;

use std::{collections::HashMap, fmt::Debug, sync::Arc};

use cfmms::pool::Pool;
use ethers::{
    providers::Middleware,
    types::{H160, H256, U256},
};

use crate::{
    abi::{self},
    error::ExecutorError,
    order::{limit_order::LimitOrder, sandbox_limit_order::SandboxLimitOrder},
};

#[derive(Debug)]
pub enum Order {
    LimitOrder(LimitOrder),
    SandboxLimitOrder(SandboxLimitOrder),
}

#[derive(Debug, Clone, Copy)]
pub enum OrderVariant {
    LimitOrder,
    SandboxLimitOrder,
}

//TODO: impl from bytes for each order variant instead of a match statement in order, or in addition

impl Order {
    pub fn can_execute(&self, markets: &HashMap<U256, HashMap<H160, Pool>>, weth: H160) -> bool {
        match self {
            Order::SandboxLimitOrder(sandbox_limit_order) => {
                sandbox_limit_order.can_execute(markets, weth)
            }

            Order::LimitOrder(limit_order) => {
                limit_order.can_execute(limit_order.buy, markets, weth)
            }
        }
    }

    pub fn owner(&self) -> H160 {
        match self {
            Order::SandboxLimitOrder(sandbox_limit_order) => sandbox_limit_order.owner,
            Order::LimitOrder(limit_order) => limit_order.owner,
        }
    }

    pub fn last_refresh_timestamp(&self) -> u32 {
        match self {
            Order::SandboxLimitOrder(sandbox_limit_order) => {
                sandbox_limit_order.last_refresh_timestamp
            }
            Order::LimitOrder(limit_order) => limit_order.last_refresh_timestamp,
        }
    }

    pub fn expiration_timestamp(&self) -> u32 {
        match self {
            Order::SandboxLimitOrder(sandbox_limit_order) => {
                sandbox_limit_order.expiration_timestamp
            }
            Order::LimitOrder(limit_order) => limit_order.expiration_timestamp,
        }
    }

    pub fn amount_in(&self) -> u128 {
        match self {
            Order::SandboxLimitOrder(sandbox_limit_order) => {
                sandbox_limit_order.amount_in_remaining
            }
            Order::LimitOrder(limit_order) => limit_order.quantity,
        }
    }
    pub fn token_in(&self) -> H160 {
        match self {
            Order::SandboxLimitOrder(sandbox_limit_order) => sandbox_limit_order.token_in,
            Order::LimitOrder(limit_order) => limit_order.token_in,
        }
    }

    pub fn token_out(&self) -> H160 {
        match self {
            Order::SandboxLimitOrder(sandbox_limit_order) => sandbox_limit_order.token_out,
            Order::LimitOrder(limit_order) => limit_order.token_out,
        }
    }

    pub fn order_id(&self) -> H256 {
        match self {
            Order::SandboxLimitOrder(sandbox_limit_order) => sandbox_limit_order.order_id,
            Order::LimitOrder(limit_order) => limit_order.order_id,
        }
    }

    pub async fn has_sufficient_balance<M: Middleware>(
        &self,
        middleware: Arc<M>,
    ) -> Result<bool, ExecutorError<M>> {
        match self {
            Order::SandboxLimitOrder(sandbox_limit_order) => {
                let token_in = abi::IErc20::new(sandbox_limit_order.token_in, middleware);

                let balance = token_in
                    .balance_of(sandbox_limit_order.owner)
                    .call()
                    .await?;

                Ok(balance >= U256::from(sandbox_limit_order.amount_in_remaining))
            }

            Order::LimitOrder(limit_order) => {
                let token_in = abi::IErc20::new(limit_order.token_in, middleware);

                let balance = token_in.balance_of(limit_order.owner).call().await?;

                Ok(balance >= U256::from(limit_order.quantity))
            }
        }
    }
}

pub async fn get_remote_order<M: Middleware>(
    order_id: H256,
    order_book_address: H160,
    order_variant: OrderVariant,
    middleware: Arc<M>,
) -> Result<Order, ExecutorError<M>> {
    match order_variant {
        OrderVariant::SandboxLimitOrder => {
            let slob = abi::ISandboxLimitOrderBook::new(order_book_address, middleware.clone());

            let return_data = slob
                .get_sandbox_limit_order_by_id(order_id.to_fixed_bytes())
                .call()
                .await?;

            Ok(Order::SandboxLimitOrder(
                SandboxLimitOrder::new_from_return_data(return_data, middleware).await?,
            ))
        }

        OrderVariant::LimitOrder => {
            let lob = abi::ILimitOrderBook::new(order_book_address, middleware);

            let return_data = lob
                .get_limit_order_by_id(order_id.to_fixed_bytes())
                .call()
                .await?;

            Ok(Order::LimitOrder(LimitOrder::new_from_return_data(
                return_data,
            )))
        }
    }
}
