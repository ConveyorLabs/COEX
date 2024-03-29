pub mod markets;
pub mod orders;

use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

use cfmms::{dex::Dex, pool::Pool};
use ethers::{
    abi::{decode, ParamType, RawLog},
    prelude::EthLogDecode,
    providers::Middleware,
    types::{Log, H160, H256, U256},
};
use tracing::info;

use crate::{
    abi::{
        OrderCanceledFilter, OrderExecutionCreditUpdatedFilter, OrderFilledFilter,
        OrderPartialFilledFilter, OrderPlacedFilter, OrderRefreshedFilter, OrderUpdatedFilter,
    },
    error::ExecutorError,
    events::BeltEvent,
    markets::Market,
    order::OrderVariant,
};

#[derive(Debug)]
pub struct State {
    pub active_orders: HashMap<H256, crate::order::Order>, //active orders
    pub pending_order_ids: Arc<Mutex<HashSet<H256>>>,      //pending_order_ids
    pub pool_address_to_market_id: HashMap<H160, U256>,    //pool_address_to_market_id
    pub markets: HashMap<U256, Market>,                    //markets
    pub market_to_affected_orders: HashMap<U256, HashSet<H256>>, //market to affected orders
}

impl State {
    pub fn new() -> State {
        State {
            active_orders: HashMap::new(),
            pending_order_ids: Arc::new(Mutex::new(HashSet::new())),
            pool_address_to_market_id: HashMap::new(),
            markets: HashMap::new(),
            market_to_affected_orders: HashMap::new(),
        }
    }

    pub async fn handle_order_updates<M: 'static + Middleware>(
        &mut self,
        order_events: Vec<(BeltEvent, Log)>,
        sandbox_limit_order_book_address: H160,
        limit_order_book_address: H160,
        weth: H160,
        dexes: &[Dex],
        middleware: Arc<M>,
    ) -> Result<HashSet<U256>, ExecutorError<M>> {
        let mut affected_markets = HashSet::new();

        //Handle order updates
        for order_event in order_events {
            let belt_event = order_event.0;
            let event_log = order_event.1;

            //Check which address the order is from and set the order variant
            let order_variant = if event_log.address == sandbox_limit_order_book_address {
                OrderVariant::SandboxLimitOrder
            } else if event_log.address == limit_order_book_address {
                OrderVariant::LimitOrder
            } else {
                panic!("Unexpected event log address: {:?}", event_log.address);
            };

            //Match the type of event and handle accordingly
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

                        //Get order from remote
                        let order = crate::order::get_remote_order(
                            order_id.into(),
                            event_log.address,
                            order_variant,
                            middleware.clone(),
                        )
                        .await?;

                        affected_markets
                            .extend(self.get_affected_markets_for_order(&order.order_id(), weth));

                        //Add markets for order
                        self.add_markets_for_order(&order, weth, dexes, middleware.clone())
                            .await?;
                        //Add order to market to affected orders
                        self.add_order_to_market_to_affected_orders(&order, weth);
                        //Add the order to active orders
                        self.place_order(order);
                    }
                }
                BeltEvent::OrderCanceled => {
                    let order_canceled_log: OrderCanceledFilter =
                        EthLogDecode::decode_log(&RawLog {
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

                        self.remove_order_from_market_to_affected_orders(&order_id.into(), weth);
                        self.remove_order(order_id.into());
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

                        //Get order from remote
                        let order = crate::order::get_remote_order(
                            order_id.into(),
                            event_log.address,
                            order_variant,
                            middleware.clone(),
                        )
                        .await?;

                        affected_markets
                            .extend(self.get_affected_markets_for_order(&order.order_id(), weth));

                        self.update_order(order);
                    }
                }

                BeltEvent::OrderFilled => {
                    let order_filled_log: OrderFilledFilter = EthLogDecode::decode_log(&RawLog {
                        topics: event_log.topics,
                        data: event_log.data.to_vec(),
                    })
                    .unwrap();
                    for order_id in order_filled_log.order_ids {
                        info!(
                            "{:?} Order Filled: {:?}",
                            order_variant,
                            H256::from(order_id)
                        );

                        self.remove_order_from_market_to_affected_orders(&order_id.into(), weth);
                        self.remove_order(order_id.into());
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

                    self.partial_fill_order(
                        order_partial_filled_log.order_id.into(),
                        order_partial_filled_log.amount_in_remaining,
                        order_partial_filled_log.amount_out_remaining,
                        order_partial_filled_log.execution_credit_remaining,
                        order_partial_filled_log.fee_remaining,
                    )
                }

                BeltEvent::OrderRefreshed => {
                    let order_refreshed_log: OrderRefreshedFilter =
                        EthLogDecode::decode_log(&RawLog {
                            topics: event_log.topics,
                            data: event_log.data.to_vec(),
                        })
                        .unwrap();

                    info!(
                        "{:?} Order Refreshed: {:?}",
                        order_variant,
                        H256::from(order_refreshed_log.order_id)
                    );

                    self.refresh_order(
                        order_refreshed_log.order_id.into(),
                        order_refreshed_log.last_refresh_timestamp,
                        order_refreshed_log.expiration_timestamp,
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

                    self.update_execution_credit(
                        order_execution_credit_updated_log.order_id.into(),
                        order_execution_credit_updated_log.new_execution_credit,
                    );
                }

                //Handling these to explicitly handle every BeltEvent. We could also use _=> {} but we are explicitly handling them to make sure we are not missing anything
                BeltEvent::UniswapV2PoolUpdate => {}
                BeltEvent::UniswapV3PoolUpdate => {}
            }
        }

        Ok(affected_markets)
    }

    //Returns markets affected
    pub fn handle_market_updates(&self, pool_events: &[Log]) -> HashSet<U256> {
        let mut markets_updated: HashSet<U256> = HashSet::new();

        for event_log in pool_events {
            if let Some(market_id) = self.pool_address_to_market_id.get(&event_log.address) {
                if let Some(market) = self.markets.get(market_id) {
                    markets_updated.insert(*market_id);

                    if let Some(pool) = market.get(&event_log.address) {
                        match pool {
                            Pool::UniswapV2(mut uniswap_v2_pool) => {
                                let log_data = decode(
                                    &[
                                        ParamType::Uint(128), //reserve0
                                        ParamType::Uint(128),
                                    ],
                                    &event_log.data,
                                )
                                .expect("Could not get log data");

                                uniswap_v2_pool.reserve_0 =
                                    log_data[0].clone().into_uint().unwrap().as_u128();

                                uniswap_v2_pool.reserve_1 =
                                    log_data[1].clone().into_uint().unwrap().as_u128();
                            }
                            Pool::UniswapV3(mut uniswap_v3_pool) => {
                                // decode log data, get liquidity and sqrt price
                                let log_data = decode(
                                    &[
                                        ParamType::Int(256),  //amount0
                                        ParamType::Int(256),  //amount1
                                        ParamType::Uint(160), //sqrtPriceX96
                                        ParamType::Uint(128), //liquidity
                                        ParamType::Int(24),
                                    ],
                                    &event_log.data,
                                )
                                .expect("Could not get log data");

                                //Update the pool data
                                uniswap_v3_pool.sqrt_price =
                                    log_data[2].to_owned().into_uint().unwrap();
                                uniswap_v3_pool.liquidity =
                                    log_data[3].to_owned().into_uint().unwrap().as_u128();
                            }
                        }
                    }
                }
            }
        }

        markets_updated
    }
}
