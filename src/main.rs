use ::tracing::info;
use error::ExecutorError;
use ethers::prelude::NonceManagerMiddleware;
use ethers::providers::{Http, Provider, Ws};
use initialization::initialize_coex;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub mod abi;
pub mod config;
pub mod error;
pub mod events;
pub mod initialization;
pub mod markets;
pub mod orders;
pub mod traces;
pub mod transaction_utils;

use ethers::providers::Middleware;
use ethers::providers::StreamExt;
use ethers::types::{H160, H256, U256};
use markets::market::{self, Market};

use orders::execution::{self, fill_orders_at_execution_price};
use orders::order::{self, Order};
use transaction_utils::handle_pending_transactions;

//TODO: move this to bin

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    traces::init_tracing();

    let (configuration, state, pending_transactions_sender, stream_provider, middleware) =
        initialize_coex::<NonceManagerMiddleware<ethers::providers::Provider<Http>>>()
            .await
            .unwrap();

    //Run an infinite loop, executing orders that are ready and updating local structures with each new block
    run_loop(
        configuration,
        middleware,
        stream_provider,
        state.active_orders,
        state.pool_address_to_market_id,
        state.markets,
        state.market_to_affected_orders,
        pending_transactions_sender,
    )
    .await?;

    Ok(())
}

async fn run_loop<M: 'static + Middleware>(
    configuration: config::Config,
    middleware: Arc<M>,
    stream_provider: Provider<Ws>,
    active_orders: Arc<Mutex<HashMap<H256, Order>>>,
    pool_address_to_market_id: HashMap<H160, U256>,
    markets: Arc<Mutex<HashMap<U256, Market>>>,
    market_to_affected_orders: Arc<Mutex<HashMap<U256, HashSet<H256>>>>,
    pending_transactions_sender: Arc<tokio::sync::mpsc::Sender<(H256, Vec<H256>)>>,
) -> Result<(), ExecutorError<M>> {
    let mut block_stream = stream_provider.subscribe_blocks().await?;
    let block_filter = events::initialize_block_filter(&configuration.dexes);
    //Get a mapping of event signature to event for quick lookup
    let event_sig_to_belt_event = events::get_event_signature_to_belt_event();

    //TODO: maybe change this to something else?
    info!("Listening for order execution");
    //Listen for new blocks to be published. On every block, check for sync logs, update weights and run bellman ford
    while let Some(block) = block_stream.next().await {
        let (order_events, pool_events) = events::sort_events(
            &middleware
                .get_logs(
                    &block_filter.clone().from_block(
                        block
                            .number
                            .expect("Could not unwrap block number from block header"),
                    ),
                )
                .await
                .map_err(ExecutorError::MiddlewareError)?,
            &event_sig_to_belt_event,
        );

        //Handle order updates
        order::handle_order_updates(
            order_events,
            active_orders.clone(),
            configuration.sandbox_limit_order_book,
            configuration.limit_order_book,
            middleware.clone(),
        )
        .await?;

        //Update markets
        let markets_updated = market::handle_market_updates(
            &pool_events,
            &pool_address_to_market_id,
            markets.clone(),
        );

        //TODO: add logic to check order cancellation and refresh orders

        //Evaluate orders for execution
        if !markets_updated.is_empty() {
            execution::evaluate_and_execute_orders(
                markets_updated,
                market_to_affected_orders.clone(),
                active_orders.clone(),
                markets.clone(),
                &configuration,
                middleware.clone(),
                pending_transactions_sender.clone(),
            )
            .await?;
        }
    }

    Ok(())
}
