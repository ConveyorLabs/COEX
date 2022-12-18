use std::{
    collections::HashMap,
    fmt::Debug,
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
};

use super::{limit_order::LimitOrder, sandbox_limit_order::SandboxLimitOrder};

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

pub async fn initialize_active_orders<M: Middleware>(
    sandbox_limit_order_book_address: H160,
    limit_order_book_address: H160,
    protocol_creation_block: BlockNumber,
    middleware: Arc<M>,
) -> Result<Arc<Mutex<HashMap<H256, Order>>>, ExecutorError<M>> {
    let mut active_orders = HashMap::new();

    //Define the step for searching a range of blocks for pair created events
    let step = 100000;

    //Unwrap can be used here because the creation block was verified within `Dex::new()`
    let from_block = protocol_creation_block
        .as_number()
        .expect("Could not unwrap the protocol creation block when initializing active orders.")
        .as_u64();

    let current_block = middleware
        .get_block_number()
        .await
        .map_err(ExecutorError::MiddlewareError)?
        .as_u64();

    for from_block in (from_block..=current_block).step_by(step) {
        let to_block = from_block + step as u64;

        let logs = middleware
            .get_logs(
                &Filter::new()
                    .topic0(ValueOrArray::Value(
                        abi::ISANDBOXLIMITORDERBOOK_ABI
                            .event("OrderPlaced")
                            .unwrap()
                            .signature(),
                    ))
                    .address(ValueOrArray::Array(vec![
                        sandbox_limit_order_book_address,
                        limit_order_book_address,
                    ]))
                    .from_block(BlockNumber::Number(ethers::types::U64([from_block])))
                    .to_block(BlockNumber::Number(ethers::types::U64([to_block]))),
            )
            .await
            .map_err(ExecutorError::MiddlewareError)?;

        for log in logs {
            let order_placed_log: OrderPlacedFilter = EthLogDecode::decode_log(&RawLog {
                topics: log.topics,
                data: log.data.to_vec(),
            })
            .expect("Error when decoding log");

            if log.address == sandbox_limit_order_book_address {
                for order_id in order_placed_log.order_ids {
                    let order_id = H256::from(order_id);

                    let order = if let Ok(order) = get_remote_order(
                        order_id,
                        sandbox_limit_order_book_address,
                        OrderVariant::SandboxLimitOrder,
                        middleware.clone(),
                    )
                    .await
                    {
                        order
                    } else {
                        continue;
                    };

                    active_orders.insert(order_id, order);
                }
            } else if log.address == limit_order_book_address {
                for order_id in order_placed_log.order_ids {
                    let order_id = H256::from(order_id);

                    let order = if let Ok(order) = get_remote_order(
                        order_id,
                        limit_order_book_address,
                        OrderVariant::LimitOrder,
                        middleware.clone(),
                    )
                    .await
                    {
                        order
                    } else {
                        continue;
                    };

                    active_orders.insert(order_id, order);
                }
            }
        }
    }

    Ok(Arc::new(Mutex::new(active_orders)))
}

pub async fn handle_order_updates<M: Middleware>(
    order_events: Vec<(BeltEvent, Log)>,
    active_orders: Arc<Mutex<HashMap<H256, Order>>>,
    sandbox_limit_order_book_address: H160,
    limit_order_book_address: H160,
    middleware: Arc<M>,
) -> Result<(), ExecutorError<M>> {
    //Handle order updates
    for order_event in order_events {
        let belt_event = order_event.0;
        let event_log = order_event.1;

        let order_variant = if event_log.address == sandbox_limit_order_book_address {
            OrderVariant::SandboxLimitOrder
        } else if event_log.address == limit_order_book_address {
            OrderVariant::LimitOrder
        } else {
            panic!("Unexpected event log address: {:?}", event_log.address);
        };

        match belt_event {
            BeltEvent::OrderPlaced => {
                let order_placed_log: OrderPlacedFilter = EthLogDecode::decode_log(&RawLog {
                    topics: event_log.topics,
                    data: event_log.data.to_vec(),
                })
                .unwrap();

                for order_id in order_placed_log.order_ids {
                    info!(
                        "{:?} Order Placed: {:?}",
                        order_variant,
                        H256::from(order_id)
                    );

                    place_order(
                        order_id.into(),
                        event_log.address,
                        active_orders.clone(),
                        order_variant,
                        middleware.clone(),
                    )
                    .await?;
                }
            }
            BeltEvent::OrderCanceled => {
                let order_canceled_log: OrderCanceledFilter = EthLogDecode::decode_log(&RawLog {
                    topics: event_log.topics,
                    data: event_log.data.to_vec(),
                })
                .unwrap();

                for order_id in order_canceled_log.order_ids {
                    info!(
                        "{:?} Order Canceled: {:?}",
                        order_variant,
                        H256::from(order_id)
                    );

                    cancel_order(order_id.into(), active_orders.clone());
                }
            }

            BeltEvent::OrderUpdated => {
                let order_updated_log: OrderUpdatedFilter = EthLogDecode::decode_log(&RawLog {
                    topics: event_log.topics,
                    data: event_log.data.to_vec(),
                })
                .unwrap();

                for order_id in order_updated_log.order_ids {
                    info!(
                        "{:?} Order Updated: {:?}",
                        order_variant,
                        H256::from(order_id)
                    );

                    update_order(
                        order_id.into(),
                        event_log.address,
                        active_orders.clone(),
                        order_variant,
                        middleware.clone(),
                    )
                    .await?;
                }
            }
            BeltEvent::OrderFilled => {
                let order_fufilled_log: OrderFufilledFilter = EthLogDecode::decode_log(&RawLog {
                    topics: event_log.topics,
                    data: event_log.data.to_vec(),
                })
                .unwrap();
                for order_id in order_fufilled_log.order_ids {
                    info!(
                        "{:?} Order Filled: {:?}",
                        order_variant,
                        H256::from(order_id)
                    );

                    fufill_order(order_id.into(), active_orders.clone())
                }
            }
            BeltEvent::OrderPartialFilled => {
                let order_partial_filled_log: OrderPartialFilledFilter =
                    EthLogDecode::decode_log(&RawLog {
                        topics: event_log.topics,
                        data: event_log.data.to_vec(),
                    })
                    .unwrap();

                info!(
                    "{:?} Order Partial Filled: {:?}",
                    order_variant,
                    H256::from(order_partial_filled_log.order_id)
                );

                partial_fill_order(
                    order_partial_filled_log.order_id.into(),
                    order_partial_filled_log.amount_in_remaining,
                    order_partial_filled_log.amount_out_remaining,
                    order_partial_filled_log.execution_credit_remaining,
                    order_partial_filled_log.fee_remaining,
                    active_orders.clone(),
                )
            }
            BeltEvent::OrderRefreshed => {
                let order_refreshed_log: OrderRefreshedFilter = EthLogDecode::decode_log(&RawLog {
                    topics: event_log.topics,
                    data: event_log.data.to_vec(),
                })
                .unwrap();

                info!(
                    "{:?} Order Refreshed: {:?}",
                    order_variant,
                    H256::from(order_refreshed_log.order_id)
                );

                refresh_order(
                    order_refreshed_log.order_id.into(),
                    order_refreshed_log.last_refresh_timestamp,
                    order_refreshed_log.expiration_timestamp,
                    active_orders.clone(),
                )
            }
            BeltEvent::OrderExecutionCreditUpdated => {
                let order_execution_credit_updated_log: OrderExecutionCreditUpdatedFilter =
                    EthLogDecode::decode_log(&RawLog {
                        topics: event_log.topics,
                        data: event_log.data.to_vec(),
                    })
                    .unwrap();

                info!(
                    "{:?} Order Refreshed: {:?}",
                    order_variant,
                    H256::from(order_execution_credit_updated_log.order_id)
                );

                update_execution_credit(
                    order_execution_credit_updated_log.order_id.into(),
                    order_execution_credit_updated_log.new_execution_credit,
                    active_orders.clone(),
                )
            }

            BeltEvent::UniswapV2PoolUpdate => {}
            BeltEvent::UniswapV3PoolUpdate => {}
        }
    }

    Ok(())
}

pub async fn get_remote_order<M: Middleware>(
    order_id: H256,
    order_book_address: H160,
    order_variant: OrderVariant,
    middleware: Arc<M>,
) -> Result<Order, ExecutorError<M>> {
    match order_variant {
        OrderVariant::SandboxLimitOrder => {
            let slob = abi::ISandboxLimitOrderBook::new(order_book_address, middleware);

            let return_data = slob
                .get_order_by_id(order_id.to_fixed_bytes())
                .call()
                .await?;

            Ok(Order::SandboxLimitOrder(
                SandboxLimitOrder::new_from_return_data(return_data),
            ))
        }

        OrderVariant::LimitOrder => {
            let lob = abi::ILimitOrderBook::new(order_book_address, middleware);

            let return_data = lob
                .get_order_by_id(order_id.to_fixed_bytes())
                .call()
                .await?;

            Ok(Order::LimitOrder(LimitOrder::new_from_return_data(
                return_data,
            )))
        }
    }
}

pub async fn place_order<M: Middleware>(
    order_id: H256,
    order_book_address: H160,
    active_orders: Arc<Mutex<HashMap<H256, Order>>>,
    order_variant: OrderVariant,
    middleware: Arc<M>,
) -> Result<(), ExecutorError<M>> {
    let order = get_remote_order(
        order_id,
        order_book_address,
        order_variant,
        middleware.clone(),
    )
    .await?;

    active_orders
        .lock()
        .expect("Could not acquire mutex lock.")
        .insert(order_id, order);

    Ok(())
}

pub async fn update_order<M: Middleware>(
    order_id: H256,
    order_book_address: H160,
    active_orders: Arc<Mutex<HashMap<H256, Order>>>,
    order_variant: OrderVariant,
    middleware: Arc<M>,
) -> Result<(), ExecutorError<M>> {
    let order = get_remote_order(
        order_id,
        order_book_address,
        order_variant,
        middleware.clone(),
    )
    .await?;

    active_orders
        .lock()
        .expect("Could not acquire mutex lock.")
        .insert(order_id, order);
    Ok(())
}

pub fn cancel_order(order_id: H256, active_orders: Arc<Mutex<HashMap<H256, Order>>>) {
    active_orders
        .lock()
        .expect("Error when unlocking active orders mutex")
        .remove(&order_id);
}

pub fn fufill_order(order_id: H256, active_orders: Arc<Mutex<HashMap<H256, Order>>>) {
    active_orders
        .lock()
        .expect("Error when unlocking active orders mutex")
        .remove(&order_id);
}

pub fn partial_fill_order(
    order_id: H256,
    _amount_in_remaining: u128,
    _amount_out_remaining: u128,
    _execution_credit_remaining: u128,
    _fee_remaining: u128,
    active_orders: Arc<Mutex<HashMap<H256, Order>>>,
) {
    let mut active_orders = active_orders
        .lock()
        .expect("Error when unlocking active orders mutex");

    if let Some(order) = active_orders.get_mut(&order_id) {
        match order {
            Order::SandboxLimitOrder(_sandbox_limit_order) => {}

            Order::LimitOrder(_limit_order) => {}
        }
    }
}

pub fn refresh_order(
    order_id: H256,
    last_refresh_timestamp: u32,
    updated_expiration_timestamp: u32,
    active_orders: Arc<Mutex<HashMap<H256, Order>>>,
) {
    let mut active_orders = active_orders
        .lock()
        .expect("Error when unlocking active orders mutex");

    if let Some(order) = active_orders.get_mut(&order_id) {
        match order {
            Order::SandboxLimitOrder(sandbox_limit_order) => {
                sandbox_limit_order.last_refresh_timestamp = last_refresh_timestamp;
                sandbox_limit_order.expiration_timestamp = updated_expiration_timestamp;
            }

            Order::LimitOrder(limit_order) => {
                limit_order.last_refresh_timestamp = last_refresh_timestamp;
                limit_order.expiration_timestamp = updated_expiration_timestamp;
            }
        }
    }
}

pub fn update_execution_credit(
    order_id: H256,
    updated_execution_credit: u128,
    active_orders: Arc<Mutex<HashMap<H256, Order>>>,
) {
    let mut active_orders = active_orders
        .lock()
        .expect("Error when unlocking active orders mutex");

    if let Some(order) = active_orders.get_mut(&order_id) {
        match order {
            Order::SandboxLimitOrder(sandbox_limit_order) => {
                sandbox_limit_order.execution_credit_remaining = updated_execution_credit;
            }

            Order::LimitOrder(limit_order) => {
                limit_order.execution_credit = updated_execution_credit;
            }
        }
    }
}
