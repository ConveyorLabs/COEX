use std::{
    collections::HashMap,
    fmt::Debug,
    str::FromStr,
    sync::{Arc, Mutex},
};

use cfmms::pool::Pool;
use ethers::{
    abi::RawLog,
    prelude::EthLogDecode,
    providers::Middleware,
    types::{BlockNumber, Filter, Log, ValueOrArray, H160, H256, U256},
};
use tracing::info;

use crate::{
    abi::{
        self, OrderCanceledFilter, OrderExecutionCreditUpdatedFilter, OrderFufilledFilter,
        OrderPartialFilledFilter, OrderPlacedFilter, OrderRefreshedFilter, OrderUpdatedFilter,
    },
    error::ExecutorError,
    events::BeltEvent,
    markets,
    orders::{self, order::OrderVariant},
};

use super::State;

impl State {
    pub async fn place_order<M: Middleware>(
        &self,
        order_id: H256,
        order_book_address: H160,
        order_variant: OrderVariant,
        middleware: Arc<M>,
    ) -> Result<(), ExecutorError<M>> {
        let order = orders::order::get_remote_order(
            order_id,
            order_book_address,
            order_variant,
            middleware.clone(),
        )
        .await?;

        self.active_orders
            .lock()
            .expect("Could not acquire mutex lock.")
            .insert(order_id, order);

        Ok(())
    }

    pub async fn update_order<M: Middleware>(
        &self,

        order_id: H256,
        order_book_address: H160,
        order_variant: OrderVariant,
        middleware: Arc<M>,
    ) -> Result<(), ExecutorError<M>> {
        let order = orders::order::get_remote_order(
            order_id,
            order_book_address,
            order_variant,
            middleware.clone(),
        )
        .await?;

        self.active_orders
            .lock()
            .expect("Could not acquire mutex lock.")
            .insert(order_id, order);
        Ok(())
    }

    pub fn cancel_order(&self, order_id: H256) {
        self.active_orders
            .lock()
            .expect("Error when unlocking active orders mutex")
            .remove(&order_id);
    }

    pub fn fill_order(&self, order_id: H256) {
        self.active_orders
            .lock()
            .expect("Error when unlocking active orders mutex")
            .remove(&order_id);
    }

    //TODO:
    pub fn partial_fill_order(
        &self,
        order_id: H256,
        _amount_in_remaining: u128,
        _amount_out_remaining: u128,
        _execution_credit_remaining: u128,
        _fee_remaining: u128,
    ) {
        let mut active_orders = self
            .active_orders
            .lock()
            .expect("Error when unlocking active orders mutex");

        if let Some(order) = active_orders.get_mut(&order_id) {
            match order {
                orders::order::Order::SandboxLimitOrder(_sandbox_limit_order) => {}

                orders::order::Order::LimitOrder(_limit_order) => {}
            }
        }
    }

    pub fn refresh_order(
        &self,
        order_id: H256,
        last_refresh_timestamp: u32,
        updated_expiration_timestamp: u32,
    ) {
        let mut active_orders = self
            .active_orders
            .lock()
            .expect("Error when unlocking active orders mutex");

        if let Some(order) = active_orders.get_mut(&order_id) {
            match order {
                orders::order::Order::SandboxLimitOrder(sandbox_limit_order) => {
                    sandbox_limit_order.last_refresh_timestamp = last_refresh_timestamp;
                    sandbox_limit_order.expiration_timestamp = updated_expiration_timestamp;
                }

                orders::order::Order::LimitOrder(limit_order) => {
                    limit_order.last_refresh_timestamp = last_refresh_timestamp;
                    limit_order.expiration_timestamp = updated_expiration_timestamp;
                }
            }
        }
    }

    pub fn update_execution_credit(&self, order_id: H256, updated_execution_credit: u128) {
        let mut active_orders = self
            .active_orders
            .lock()
            .expect("Error when unlocking active orders mutex");

        if let Some(order) = active_orders.get_mut(&order_id) {
            match order {
                orders::order::Order::SandboxLimitOrder(sandbox_limit_order) => {
                    sandbox_limit_order.execution_credit_remaining = updated_execution_credit;
                }

                orders::order::Order::LimitOrder(limit_order) => {
                    limit_order.execution_credit = updated_execution_credit;
                }
            }
        }
    }
}
