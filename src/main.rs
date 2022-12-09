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
use ethers::signers::{LocalWallet, Wallet};
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
            order::evaluate_and_execute_orders(
                markets_updated,
                market_to_affected_orders.clone(),
                active_orders.clone(),
                markets.clone(),
                configuration.weth_address,
                configuration.sandbox_limit_order_book,
                configuration.limit_order_book,
                configuration.wallet.clone(),
                &configuration.chain,
                provider.clone(),
            )
            .await?;
        }
    }

    Ok(())
}
