use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::sync::{Arc, Mutex};

use error::ExecutorError;
use ethers::providers::{Http, JsonRpcClient, Provider, ProviderError, Ws};

pub mod abi;
pub mod config;
pub mod error;
pub mod events;
pub mod markets;
pub mod orders;

use ethers::providers::Middleware;
use ethers::providers::StreamExt;
use ethers::types::{H160, H256, U256};
use markets::market::{self, Market};
use orders::execution;
use orders::order::{self, Order};

//TODO: move this to bin
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    //Initialize a new configuration
    let configuration = config::Config::new();
    //Initialize the providers
    let provider: Arc<Provider<Http>> = Arc::new(Provider::try_from(&configuration.http_endpoint)?);
    let stream_provider = Provider::<Ws>::connect(&configuration.ws_endpoint).await?;
    //Initialize the markets and order structures
    let (active_orders, pool_address_to_market_id, markets, market_to_affected_orders) =
        initialize_executor(&configuration, provider.clone()).await?;

    //Run an infinite loop, executing orders that are ready and updating local structures with each new block
    run_loop(
        configuration,
        provider,
        stream_provider,
        active_orders,
        pool_address_to_market_id,
        markets,
        market_to_affected_orders,
    )
    .await?;

    Ok(())
}

async fn initialize_executor<P: 'static + JsonRpcClient>(
    configuration: &config::Config,
    provider: Arc<Provider<P>>,
) -> Result<
    (
        Arc<Mutex<HashMap<H256, Order>>>,         //active orders
        HashMap<H160, U256>,                      //pool_address_to_market_id
        Arc<Mutex<HashMap<U256, Market>>>,        //markets
        Arc<Mutex<HashMap<U256, HashSet<H256>>>>, //market to affected orders
    ),
    ExecutorError<P>,
> {
    //Initialize active orders
    let active_orders = orders::order::initialize_active_orders(
        configuration.sandbox_limit_order_book,
        configuration.limit_order_book,
        configuration.protocol_creation_block,
        provider.clone(),
    )
    .await
    .expect("There was an issue while initializing active orders");

    //initialize markets
    let (pool_address_to_market_id, markets, market_to_affected_orders) =
        market::initialize_market_structures(
            active_orders.clone(),
            &configuration.dexes,
            configuration.weth_address,
            provider.clone(),
        )
        .await
        .expect("There was an issue while initializing market structures");

    Ok((
        active_orders,
        pool_address_to_market_id,
        markets,
        market_to_affected_orders,
    ))
}

async fn run_loop<P: 'static + JsonRpcClient>(
    configuration: config::Config,
    provider: Arc<Provider<P>>,
    stream_provider: Provider<Ws>,
    active_orders: Arc<Mutex<HashMap<H256, Order>>>,
    pool_address_to_market_id: HashMap<H160, U256>,
    markets: Arc<Mutex<HashMap<U256, Market>>>,
    market_to_affected_orders: Arc<Mutex<HashMap<U256, HashSet<H256>>>>,
) -> Result<(), ExecutorError<P>> {
    let mut block_stream = stream_provider.subscribe_blocks().await?;
    let block_filter = events::initialize_block_filter(&configuration.dexes);
    //Get a mapping of event signature to event for quick lookup
    let event_sig_to_belt_event = events::get_event_signature_to_belt_event();

    //Listen for new blocks to be published. On every block, check for sync logs, update weights and run bellman ford
    while let Some(block) = block_stream.next().await {
        let (order_events, pool_events) = events::sort_events(
            &provider
                .get_logs(
                    &block_filter.clone().from_block(
                        block
                            .number
                            .expect("Could not unwrap block number from block header"),
                    ),
                )
                .await?,
            &event_sig_to_belt_event,
        );

        //Handle order updates
        order::handle_order_updates(order_events, active_orders.clone(), provider.clone()).await?;

        //Update markets
        let markets_updated = market::handle_market_updates(
            &pool_events,
            &pool_address_to_market_id,
            markets.clone(),
        );

        //TODO: add logic to check order cancellation and refresh orders

        //Evaluate orders for execution
        if markets_updated.len() > 0 {
            execution::evaluate_and_execute_orders(
                markets_updated,
                market_to_affected_orders.clone(),
                active_orders.clone(),
                markets.clone(),
                &configuration,
                provider.clone(),
            )
            .await?;
        }
    }

    Ok(())
}
