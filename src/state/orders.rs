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
    pub fn place_order(&self, order: orders::order::Order) {
        self.active_orders.insert(order.order_id(), order);
    }

    pub fn update_order(&self, order: orders::order::Order) {
        self.active_orders.insert(order.order_id(), order);
    }

    pub fn remove_order(&self, order_id: H256) {
        self.active_orders.remove(&order_id);
    }

    pub fn fill_order(&self, order_id: H256) {
        self.active_orders.remove(&order_id);
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
        if let Some(order) = self.active_orders.get_mut(&order_id) {
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
        if let Some(order) = self.active_orders.get_mut(&order_id) {
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
        if let Some(order) = self.active_orders.get_mut(&order_id) {
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
