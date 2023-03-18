use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
    sync::{Arc, Mutex},
    time::Duration,
};

use crate::{
    abi::{self, OrderPlacedFilter},
    cancellation, check_in, config,
    error::ExecutorError,
    execution,
    markets::{self, get_market_id, Market},
    order::{self},
    refresh, state, transactions,
};
use cfmms::{dex::Dex, pool::Pool};
use ethers::{
    abi::RawLog,
    prelude::{
        gas_escalator::{self, GasEscalatorMiddleware, LinearGasPrice},
        EthLogDecode, NonceManagerMiddleware,
    },
    providers::{Http, Middleware, Provider, Ws},
    types::{BlockNumber, Filter, ValueOrArray, H160, H256, U256},
};
use ethers::{middleware::gas_escalator::*, prelude::nonce_manager};
use tokio::sync::mpsc::Sender;
use tracing::info;

pub async fn initialize_coex<M: Middleware>() -> Result<
    (
        config::Config,
        state::State,
        Arc<Sender<(H256, Vec<H256>)>>,
        String,
        Arc<NonceManagerMiddleware<GasEscalatorMiddleware<Provider<Http>, GeometricGasPrice>>>,
    ),
    ExecutorError<M>,
> {
    //Initialize a new configuration
    let configuration = config::Config::new();
    //Initialize the providers
    let provider: Provider<Http> = Provider::try_from(&configuration.http_endpoint)
        .expect("Could not initialize HTTP provider");
    let stream_provider_endpoint = configuration.ws_endpoint.to_owned();

    let geometric_gas_price = GeometricGasPrice::new::<u32, u64>(2.0, 10_u64, None);
    let gas_escalator =
        GasEscalatorMiddleware::new(provider.clone(), geometric_gas_price, Frequency::PerBlock);
    let nonce_manager = NonceManagerMiddleware::new(gas_escalator, configuration.wallet_address);
    let middleware = Arc::new(nonce_manager);

    //Initialize the markets and order structures
    let state = initialize_state(&configuration, middleware.clone())
        .await
        .expect("Could not initialize state"); //TODO: bubble up this error, just using expect for fast development

    let pending_transactions_sender = Arc::new(
        transactions::initialize_pending_transaction_handler(
            state.pending_order_ids.clone(),
            Duration::new(0, 10000000), //10 ms
            middleware.clone(),
        )
        .await,
    );

    Ok((
        configuration,
        state,
        pending_transactions_sender,
        stream_provider_endpoint,
        middleware,
    ))
}

async fn initialize_state<M: 'static + Middleware>(
    configuration: &config::Config,
    middleware: Arc<M>,
) -> Result<state::State, ExecutorError<M>> {
    tracing::info!("Initializing active orders...");

    let mut state = state::State::new();
    //Initialize active orders
    let (active_orders, number_of_orders) = initialize_active_orders(
        configuration.sandbox_limit_order_book,
        configuration.limit_order_book,
        configuration.protocol_creation_block,
        middleware.clone(),
    )
    .await?;
    tracing::info!("Active orders initialized ({:?} orders)", number_of_orders);

    tracing::info!("Initializing markets...");
    for (_, order) in active_orders.iter() {
        //Add markets for order
        state
            .add_markets_for_order(
                order,
                configuration.weth_address,
                &configuration.dexes,
                middleware.clone(),
            )
            .await?;

        //Add order to market to affected orders
        state.add_order_to_market_to_affected_orders(&order, configuration.weth_address);
    }

    tracing::info!("Markets initialized");

    state.active_orders = active_orders;

    Ok(state)
}

pub async fn initialize_active_orders<M: Middleware>(
    sandbox_limit_order_book_address: H160,
    limit_order_book_address: H160,
    protocol_creation_block: BlockNumber,
    middleware: Arc<M>,
) -> Result<(HashMap<H256, order::Order>, usize), ExecutorError<M>> {
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

                    let order = match order::get_remote_order(
                        order_id,
                        sandbox_limit_order_book_address,
                        order::OrderVariant::SandboxLimitOrder,
                        middleware.clone(),
                    )
                    .await
                    {
                        Ok(order) => order,
                        Err(err) => {
                            //TODO: match contract error, panic on provider error
                            continue;
                        }
                    };

                    active_orders.insert(order_id, order);
                }
            } else if log.address == limit_order_book_address {
                for order_id in order_placed_log.order_ids {
                    let order_id = H256::from(order_id);

                    let order = match order::get_remote_order(
                        order_id,
                        limit_order_book_address,
                        order::OrderVariant::LimitOrder,
                        middleware.clone(),
                    )
                    .await
                    {
                        Ok(order) => order,
                        Err(err) => {
                            //TODO: match contract error, panic on provider error
                            continue;
                        }
                    };

                    active_orders.insert(order_id, order);
                }
            }
        }
    }

    let number_of_orders = active_orders.len();
    Ok((active_orders, number_of_orders))
}
