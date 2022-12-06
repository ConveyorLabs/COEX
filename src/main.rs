use std::error::Error;
use std::sync::Arc;

use ethers::providers::{Http, Provider, Ws};

pub mod abi;
pub mod config;
pub mod error;
pub mod events;
pub mod markets;
pub mod orders;

use ethers::providers::Middleware;
use ethers::providers::StreamExt;
use ethers::types::Log;
use events::BeltEvent;
use markets::market::{self};
use orders::order::{self};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let configuration = config::Config::new();

    let provider: Arc<Provider<Http>> = Arc::new(
        Provider::try_from(configuration.http_endpoint)
            .expect("Could not initialize http provider from endpoint"),
    );
    let stream_provider = Provider::<Ws>::connect(configuration.ws_endpoint).await?;

    //Initialize active orders
    let active_orders = orders::order::initialize_active_orders(
        configuration.sandbox_limit_order_book,
        configuration.limit_order_book,
        configuration.protocol_creation_block,
        provider.clone(),
    )
    .await?;

    //initialize markets
    let (pool_address_to_market_id, markets, market_to_affected_orders) =
        market::initialize_market_structures(
            active_orders.clone(),
            &configuration.dexes,
            configuration.weth_address,
            provider.clone(),
        )
        .await?;

    let mut block_stream = stream_provider.subscribe_blocks().await?;
    let block_filter = events::initialize_block_filter(configuration.dexes);
    //Get a mapping of event signature to event for quick lookup
    let event_sig_to_belt_event = events::get_event_signature_to_belt_event();

    //Listen for new blocks to be published. On every block, check for sync logs, update weights and run bellman ford
    while let Some(block) = block_stream.next().await {
        //Update block filter to get logs from the header block number
        let block_filter = block_filter.clone().from_block(
            block
                .number
                .expect("Could not unwrap block number from block header"),
        );

        let event_logs = provider.get_logs(&block_filter).await?;

        //Separate order event logs and pool event logs
        let mut order_events: Vec<(BeltEvent, Log)> = vec![];
        let mut pool_events: Vec<Log> = vec![];
        for log in event_logs {
            if let Some(belt_event) = event_sig_to_belt_event.get(&log.topics[0]) {
                match belt_event {
                    BeltEvent::UniswapV2PoolUpdate => pool_events.push(log),
                    BeltEvent::UniswapV3PoolUpdate => pool_events.push(log),
                    _ => order_events.push((*belt_event, log)),
                }
            }
        }

        //Handle order updates
        order::handle_order_updates(order_events, active_orders.clone(), provider.clone()).await?;

        //Update markets
        let markets_updated = market::handle_market_updates(
            &pool_events,
            &pool_address_to_market_id,
            markets.clone(),
        );

        //Evaluate orders for execution
        if markets_updated.len() > 0 {
            order::evaluate_and_execute_orders(
                markets_updated,
                market_to_affected_orders.clone(),
                active_orders.clone(),
                markets.clone(),
                configuration.weth_address,
                configuration.uni_v3_quoter,
                provider.clone(),
            )
            .await?;
        }
    }

    Ok(())
}
