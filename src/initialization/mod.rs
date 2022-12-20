use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
    time::Duration,
};

use cfmms::{dex::Dex, pool::Pool};
use ethers::{
    abi::RawLog,
    prelude::{EthLogDecode, NonceManagerMiddleware},
    providers::{Http, Middleware, Provider, Ws},
    types::{BlockNumber, Filter, ValueOrArray, H160, H256, U256},
};
use tokio::sync::mpsc::Sender;
use tracing::info;

use crate::{
    abi::{self, OrderPlacedFilter},
    config,
    error::ExecutorError,
    execution,
    markets::market::{self, Market},
    orders::{
        self,
        order::{Order, OrderVariant},
    },
    transaction_utils,
};

pub async fn initialize_coex<M: Middleware>() -> Result<
    (
        config::Config,
        State,
        Arc<Sender<(H256, Vec<H256>)>>,
        Provider<Ws>,
        Arc<NonceManagerMiddleware<ethers::providers::Provider<Http>>>,
    ),
    ExecutorError<M>,
> {
    //Initialize a new configuration
    let configuration = config::Config::new();
    //Initialize the providers
    let provider: Provider<Http> = Provider::try_from(&configuration.http_endpoint)
        .expect("Could not initialize HTTP provider");
    let stream_provider = Provider::<Ws>::connect(&configuration.ws_endpoint)
        .await
        .expect("Could not initialize WS provider");

    let middleware = Arc::new(NonceManagerMiddleware::new(
        provider.clone(),
        configuration.wallet_address,
    ));

    //Initialize the markets and order structures
    let state = initialize_state(&configuration, middleware.clone())
        .await
        .expect("Could not initialize state"); //TODO: bubble up this error, just using expect for fast development

    let pending_transactions_sender = Arc::new(
        transaction_utils::initialize_pending_transaction_handler(
            state.pending_order_ids.clone(),
            Duration::new(0, 500000000), //500 ms
            middleware.clone(),
        )
        .await,
    );

    info!("Checking for orders at execution price...");
    execution::fill_all_orders_at_execution_price(
        state.active_orders.clone(),
        state.markets.clone(),
        &configuration,
        pending_transactions_sender.clone(),
        middleware.clone(),
    )
    .await
    .expect("Could not execute orders on initialization"); //TODO: bubble up this error, just using expect for fast development

    Ok((
        configuration,
        state,
        pending_transactions_sender,
        stream_provider,
        middleware,
    ))
}

pub struct State {
    pub active_orders: Arc<Mutex<HashMap<H256, Order>>>, //active orders
    pub pending_order_ids: Arc<Mutex<HashSet<H256>>>,    //pending_order_ids
    pub pool_address_to_market_id: HashMap<H160, U256>,  //pool_address_to_market_id
    pub markets: Arc<Mutex<HashMap<U256, Market>>>,      //markets
    pub market_to_affected_orders: Arc<Mutex<HashMap<U256, HashSet<H256>>>>, //market to affected orders
}

async fn initialize_state<M: 'static + Middleware>(
    configuration: &config::Config,
    middleware: Arc<M>,
) -> Result<State, ExecutorError<M>> {
    info!("Initializing active orders...");
    //Initialize active orders
    let active_orders = initialize_active_orders(
        configuration.sandbox_limit_order_book,
        configuration.limit_order_book,
        configuration.protocol_creation_block,
        middleware.clone(),
    )
    .await?;

    info!("Active orders initialized");

    info!("Initializing markets...");
    //initialize markets
    let (pool_address_to_market_id, markets, market_to_affected_orders) =
        initialize_market_structures(
            active_orders.clone(),
            &configuration.dexes,
            configuration.weth_address,
            middleware.clone(),
        )
        .await?;

    info!("Markets initialized");

    Ok(State {
        active_orders,
        pending_order_ids: Arc::new(Mutex::new(HashSet::new())),
        pool_address_to_market_id,
        markets,
        market_to_affected_orders,
    })
}

//Returns pool addr to market id, markets, market to affected orders,
pub async fn initialize_market_structures<M: 'static + Middleware>(
    active_orders: Arc<Mutex<HashMap<H256, Order>>>,
    dexes: &[Dex],
    weth: H160,
    middleware: Arc<M>,
) -> Result<
    (
        HashMap<H160, U256>,
        Arc<Mutex<HashMap<U256, HashMap<H160, Pool>>>>,
        Arc<Mutex<HashMap<U256, HashSet<H256>>>>,
    ),
    ExecutorError<M>,
> {
    let mut pool_address_to_market_id: HashMap<H160, U256> = HashMap::new();
    let mut market_initialized: HashSet<U256> = HashSet::new();
    let mut markets: HashMap<U256, HashMap<H160, Pool>> = HashMap::new();
    let mut market_to_affected_orders: HashMap<U256, HashSet<H256>> = HashMap::new();

    for (_, order) in active_orders
        .lock()
        .expect("Could not acquire lock on active orders")
        .iter()
    {
        match order {
            Order::SandboxLimitOrder(sandbox_limit_order) => {
                //Update for token a -> token b market
                market::update_market_structures(
                    sandbox_limit_order.order_id,
                    sandbox_limit_order.token_in,
                    sandbox_limit_order.token_out,
                    &mut pool_address_to_market_id,
                    &mut market_initialized,
                    &mut markets,
                    &mut market_to_affected_orders,
                    dexes,
                    middleware.clone(),
                )
                .await?;

                //Update for token a -> weth market
                market::update_market_structures(
                    sandbox_limit_order.order_id,
                    sandbox_limit_order.token_in,
                    weth,
                    &mut pool_address_to_market_id,
                    &mut market_initialized,
                    &mut markets,
                    &mut market_to_affected_orders,
                    dexes,
                    middleware.clone(),
                )
                .await?;

                //Update for token b -> weth market
                market::update_market_structures(
                    sandbox_limit_order.order_id,
                    sandbox_limit_order.token_out,
                    weth,
                    &mut pool_address_to_market_id,
                    &mut market_initialized,
                    &mut markets,
                    &mut market_to_affected_orders,
                    dexes,
                    middleware.clone(),
                )
                .await?;
            }
            Order::LimitOrder(limit_order) => {
                //Update for token a -> weth market
                market::update_market_structures(
                    limit_order.order_id,
                    limit_order.token_in,
                    weth,
                    &mut pool_address_to_market_id,
                    &mut market_initialized,
                    &mut markets,
                    &mut market_to_affected_orders,
                    dexes,
                    middleware.clone(),
                )
                .await?;

                //Update for token b -> weth market
                market::update_market_structures(
                    limit_order.order_id,
                    limit_order.token_out,
                    weth,
                    &mut pool_address_to_market_id,
                    &mut market_initialized,
                    &mut markets,
                    &mut market_to_affected_orders,
                    dexes,
                    middleware.clone(),
                )
                .await?;
            }
        }
    }

    Ok((
        pool_address_to_market_id,
        Arc::new(Mutex::new(markets)),
        Arc::new(Mutex::new(market_to_affected_orders)),
    ))
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

                    let order = if let Ok(order) = orders::order::get_remote_order(
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

                    let order = if let Ok(order) = orders::order::get_remote_order(
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
