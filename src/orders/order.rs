use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

use ethers::{
    abi::{decode, ethabi::Bytes, Detokenize, Param, ParamType, RawLog, Tokenizable},
    prelude::EthLogDecode,
    providers::{JsonRpcClient, JsonRpcClientWrapper, Middleware, Provider},
    types::{BlockNumber, Filter, Log, ValueOrArray, H160, H256},
};
use pair_sync::pool::Pool;

use crate::{
    abi::{
        self, ISandboxLimitOrderBook, OrderCanceledFilter, OrderExecutionCreditUpdatedFilter,
        OrderFufilledFilter, OrderPartialFilledFilter, OrderPlacedFilter, OrderRefreshedFilter,
        OrderUpdatedFilter,
    },
    error::BeltError,
    events::BeltEvent,
};

use super::{limit_order::LimitOrder, sandbox_limit_order::SandboxLimitOrder};

#[derive(Debug)]
pub enum Order {
    LimitOrder(LimitOrder),
    SandboxLimitOrder(SandboxLimitOrder),
}

pub enum OrderVariant {
    LimitOrder,
    SandboxLimitOrder,
}

impl Order {
    fn from_bytes<P: JsonRpcClient>(
        data: &[u8],
        order_variant: OrderVariant,
    ) -> Result<Order, BeltError<P>> {
        match order_variant {
            OrderVariant::LimitOrder => {
                let return_types = vec![
                    ParamType::Bool,           //buy
                    ParamType::Bool,           //taxed
                    ParamType::Bool,           //stoploss
                    ParamType::Uint(32),       //last refresh timestamp
                    ParamType::Uint(32),       //expiration timestamp
                    ParamType::Uint(32),       //feeIn
                    ParamType::Uint(32),       //feeOut
                    ParamType::Uint(16),       //taxIn
                    ParamType::Uint(128),      //price
                    ParamType::Uint(128),      //amount out min
                    ParamType::Uint(128),      //quantity
                    ParamType::Uint(128),      //execution credit
                    ParamType::Address,        //owner
                    ParamType::Address,        //token in
                    ParamType::Address,        //token out
                    ParamType::FixedBytes(32), //order Id
                ];

                let limit_order = decode(&return_types, data).expect("Could not decode order data");

                Ok(Order::LimitOrder(LimitOrder {
                    buy: limit_order[0]
                        .to_owned()
                        .into_bool()
                        .expect("Could not convert token into bool"),
                    taxed: limit_order[1]
                        .to_owned()
                        .into_bool()
                        .expect("Could not convert token into bool"),
                    stop_loss: limit_order[2]
                        .to_owned()
                        .into_bool()
                        .expect("Could not convert token into bool"),
                    last_refresh_timestamp: limit_order[3]
                        .to_owned()
                        .into_uint()
                        .expect("Could not convert token into uint")
                        .as_u32(),
                    expiration_timestamp: limit_order[4]
                        .to_owned()
                        .into_uint()
                        .expect("Could not convert token into uint")
                        .as_u32(),
                    fee_in: limit_order[5]
                        .to_owned()
                        .into_uint()
                        .expect("Could not convert token into uint")
                        .as_u32(),
                    fee_out: limit_order[6]
                        .to_owned()
                        .into_uint()
                        .expect("Could not convert token into uint")
                        .as_u32(),
                    tax_in: limit_order[7]
                        .to_owned()
                        .into_uint()
                        .expect("Could not convert token into uint")
                        .as_u32() as u16,
                    price: limit_order[8]
                        .to_owned()
                        .into_uint()
                        .expect("Could not convert token into uint")
                        .as_u128(),
                    amount_out_min: limit_order[10]
                        .to_owned()
                        .into_uint()
                        .expect("Could not convert token into uint")
                        .as_u128(),
                    quantity: limit_order[9]
                        .to_owned()
                        .into_uint()
                        .expect("Could not convert token into uint")
                        .as_u128(),
                    execution_credit: limit_order[11]
                        .to_owned()
                        .into_uint()
                        .expect("Could not convert token into uint")
                        .as_u128(),
                    owner: H160::from_token(limit_order[12].to_owned())
                        .expect("Could not convert bytes into H160"),
                    token_in: H160::from_token(limit_order[13].to_owned())
                        .expect("Could not convert bytes into H160"),
                    token_out: H160::from_token(limit_order[14].to_owned())
                        .expect("Could not convert bytes into H160"),
                    order_id: H256::from_token(limit_order[15].to_owned())
                        .expect("Could not convert bytes into H256"),
                }))
            }
            OrderVariant::SandboxLimitOrder => {
                let return_types = vec![
                    ParamType::Uint(32),       //last refresh timestamp
                    ParamType::Uint(32),       //expiration timestamp
                    ParamType::Uint(128),      //fill percent
                    ParamType::Uint(128),      //fee remaining
                    ParamType::Uint(128),      //amount in remaining
                    ParamType::Uint(128),      //amount out remaining
                    ParamType::Uint(128),      //execution credit remaining
                    ParamType::Address,        //owner
                    ParamType::Address,        //token in
                    ParamType::Address,        //token out
                    ParamType::FixedBytes(32), //order Id
                ];

                let sandbox_limit_order =
                    decode(&return_types, data).expect("Could not decode order data");

                Ok(Order::SandboxLimitOrder(SandboxLimitOrder {
                    last_refresh_timestamp: sandbox_limit_order[0]
                        .to_owned()
                        .into_uint()
                        .expect("Could not convert token into uint")
                        .as_u32(),
                    expiration_timestamp: sandbox_limit_order[1]
                        .to_owned()
                        .into_uint()
                        .expect("Could not convert token into uint")
                        .as_u32(),
                    fill_percent: sandbox_limit_order[2]
                        .to_owned()
                        .into_uint()
                        .expect("Could not convert token into uint")
                        .as_u128(),
                    fee_remaining: sandbox_limit_order[3]
                        .to_owned()
                        .into_uint()
                        .expect("Could not convert token into uint")
                        .as_u128(),
                    amount_in_remaining: sandbox_limit_order[4]
                        .to_owned()
                        .into_uint()
                        .expect("Could not convert token into uint")
                        .as_u128(),
                    amount_out_remaining: sandbox_limit_order[5]
                        .to_owned()
                        .into_uint()
                        .expect("Could not convert token into uint")
                        .as_u128(),
                    execution_credit_remaining: sandbox_limit_order[6]
                        .to_owned()
                        .into_uint()
                        .expect("Could not convert token into uint")
                        .as_u128(),
                    owner: H160::from_token(sandbox_limit_order[7].to_owned())
                        .expect("Could not convert bytes into H160"),
                    token_in: H160::from_token(sandbox_limit_order[8].to_owned())
                        .expect("Could not convert bytes into H160"),
                    token_out: H160::from_token(sandbox_limit_order[9].to_owned())
                        .expect("Could not convert bytes into H160"),
                    order_id: H256::from_token(sandbox_limit_order[10].to_owned())
                        .expect("Could not convert bytes into H256"),
                }))
            }
        }
    }
}

pub async fn initialize_active_orders<P: JsonRpcClient>(
    sandbox_limit_order_book_address: H160,
    limit_order_book_address: H160,
    protocol_creation_block: BlockNumber,
    provider: Arc<Provider<P>>,
) -> Result<Arc<Mutex<HashMap<H256, Order>>>, BeltError<P>> {
    let mut active_orders = HashMap::new();

    //Define the step for searching a range of blocks for pair created events
    let step = 100000;

    //Unwrap can be used here because the creation block was verified within `Dex::new()`
    let from_block = protocol_creation_block
        .as_number()
        .expect("Could not unwrap the protocol creation block when initializing active orders.")
        .as_u64();

    let current_block = provider.get_block_number().await?.as_u64();

    for from_block in (from_block..=current_block).step_by(step) {
        let to_block = from_block + step as u64;

        let logs = provider
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
            .await?;

        for log in logs {
            let order_placed_log: OrderPlacedFilter = EthLogDecode::decode_log(&RawLog {
                topics: log.topics,
                data: log.data.to_vec(),
            })
            .unwrap();

            if log.address == sandbox_limit_order_book_address {
                for order_id in order_placed_log.order_ids {
                    let order_id = H256::from(order_id);

                    let order = get_remote_sandbox_limit_order(
                        order_id,
                        sandbox_limit_order_book_address,
                        provider.clone(),
                    )
                    .await?;

                    active_orders.insert(order_id, order);
                }
            } else if log.address == limit_order_book_address {
                for order_id in order_placed_log.order_ids {
                    let order_id = H256::from(order_id);

                    let order = get_remote_limit_order(
                        order_id,
                        limit_order_book_address,
                        provider.clone(),
                    )
                    .await?;

                    active_orders.insert(order_id, order);
                }
            }
        }
    }

    Ok(Arc::new(Mutex::new(active_orders)))
}

pub async fn handle_order_updates<P: JsonRpcClient>(
    order_events: Vec<(BeltEvent, Log)>,
    active_orders: Arc<Mutex<HashMap<H256, Order>>>,
    provider: Arc<Provider<P>>,
) -> Result<(), BeltError<P>> {
    //Handle order updates
    for order_event in order_events {
        let belt_event = order_event.0;
        let event_log = order_event.1;

        match belt_event {
            BeltEvent::OrderPlaced => {
                let order_placed_log: OrderPlacedFilter = EthLogDecode::decode_log(&RawLog {
                    topics: event_log.topics,
                    data: event_log.data.to_vec(),
                })
                .unwrap();

                for order_id in order_placed_log.order_ids {
                    place_order(order_id.into(), active_orders.clone(), provider.clone()).await?;
                }
            }
            BeltEvent::OrderCanceled => {
                let order_canceled_log: OrderCanceledFilter = EthLogDecode::decode_log(&RawLog {
                    topics: event_log.topics,
                    data: event_log.data.to_vec(),
                })
                .unwrap();

                for order_id in order_canceled_log.order_ids {
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
                    update_order(order_id.into(), active_orders.clone(), provider.clone()).await?;
                }
            }
            BeltEvent::OrderFufilled => {
                let order_fufilled_log: OrderFufilledFilter = EthLogDecode::decode_log(&RawLog {
                    topics: event_log.topics,
                    data: event_log.data.to_vec(),
                })
                .unwrap();
                for order_id in order_fufilled_log.order_ids {
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

pub async fn get_remote_sandbox_limit_order<P: JsonRpcClient>(
    order_id: H256,
    order_book_address: H160,
    provider: Arc<Provider<P>>,
) -> Result<Order, BeltError<P>> {
    let slob = abi::ISandboxLimitOrderBook::new(order_book_address, provider);

    let order_bytes = slob
        .get_sandbox_limit_order_by_id(order_id.into())
        .call()
        .await?;

    Order::from_bytes(&order_bytes, OrderVariant::SandboxLimitOrder)
}

pub async fn get_remote_limit_order<P: JsonRpcClient>(
    order_id: H256,
    order_book_address: H160,
    provider: Arc<Provider<P>>,
) -> Result<Order, BeltError<P>> {
    let lob = abi::ILimitOrderBook::new(order_book_address, provider);
    let order_bytes = lob.get_limit_order_by_id(order_id.into()).call().await?;
    Order::from_bytes(&order_bytes, OrderVariant::LimitOrder)
}

pub async fn place_order<P: JsonRpcClient>(
    order_id: H256,
    active_orders: Arc<Mutex<HashMap<H256, Order>>>,
    provider: Arc<Provider<P>>,
) -> Result<(), BeltError<P>> {
    Ok(())
}

pub async fn update_order<P: JsonRpcClient>(
    order_id: H256,
    active_orders: Arc<Mutex<HashMap<H256, Order>>>,
    provider: Arc<Provider<P>>,
) -> Result<(), BeltError<P>> {
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
    amount_in_remaining: u128,
    amount_out_remaining: u128,
    execution_credit_remaining: u128,
    fee_remaining: u128,
    active_orders: Arc<Mutex<HashMap<H256, Order>>>,
) {
    let mut active_orders = active_orders
        .lock()
        .expect("Error when unlocking active orders mutex");

    if let Some(order) = active_orders.get_mut(&order_id) {
        match order {
            Order::SandboxLimitOrder(sandbox_limit_order) => {}

            Order::LimitOrder(limit_order) => {}
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

pub fn evaluate_and_execute_orders(
    affected_markets: HashSet<u64>,
    market_to_affected_orders: Arc<Mutex<HashMap<u64, HashSet<H256>>>>,
) {
    let market_to_affected_orders = market_to_affected_orders
        .lock()
        .expect("Could not acquire mutex lock");

    for market_id in affected_markets {
        if let Some(affected_orders) = market_to_affected_orders.get(&market_id) {

            //TODO: check for affected order exectution
        }
    }
}