use ::tracing::info;
use coex::error::ExecutorError;
use coex::initialization::initialize_coex;
use coex::{cancellation, check_in, state};
use coex::{config, events, execution, refresh, traces};
use ethers::prelude::k256::ecdsa::SigningKey;
use ethers::prelude::NonceManagerMiddleware;
use ethers::providers::{Http, Provider, Ws};
use std::collections::HashSet;
use std::error::Error;
use std::sync::Arc;

use ethers::providers::Middleware;
use ethers::providers::StreamExt;
use ethers::types::{H256, U256, U64};

//TODO: move this to bin

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    traces::init_tracing();

    //TODO: get the last block from the initialize coex function before we initialize markets or active orders

    let (configuration, state, pending_transactions_sender, stream_provider_endpoint, middleware) =
        initialize_coex::<NonceManagerMiddleware<ethers::providers::Provider<Http>>>()
            .await
            .unwrap();

    let current_block = middleware.get_block_number().await?;

    check_in::spawn_check_in_service(
        configuration.executor_address,
        configuration.wallet_address,
        configuration.wallet_key.clone(),
        configuration.chain,
        middleware.clone(),
    )
    .await?;

    //NOTE: TODO: maybe sync before execution to update markets from any missed logs during other parts of initialization
    info!("Checking for orders at execution price...");
    execution::fill_orders_at_execution_price(
        &configuration,
        &state,
        state
            .markets
            .keys()
            .map(|f| f.to_owned())
            .collect::<HashSet<U256>>(),
        pending_transactions_sender.clone(),
        middleware.clone(),
    )
    .await
    .expect("Could not execute orders on initialization"); //TODO: bubble up this error, just using expect for fast development

    //Run an infinite loop, executing orders that are ready and updating local structures with each new block
    run_loop(
        configuration,
        stream_provider_endpoint,
        state,
        pending_transactions_sender,
        current_block,
        middleware,
    )
    .await?;

    Ok(())
}

async fn run_loop<M: 'static + Middleware>(
    configuration: config::Config,
    stream_provider_endpoint: String,
    mut state: state::State,
    pending_transactions_sender: Arc<tokio::sync::mpsc::Sender<(H256, Vec<H256>)>>,
    mut last_synced_block: U64,
    middleware: Arc<M>,
) -> Result<(), ExecutorError<M>> {
    let stream_provider = Provider::<Ws>::connect(stream_provider_endpoint.clone()).await?;

    let mut block_stream = stream_provider.subscribe_blocks().await?;
    let block_filter = events::initialize_block_filter(&configuration.dexes);

    //Get a mapping of event signature to event for quick lookup
    let event_sig_to_belt_event = events::get_event_signature_to_belt_event();

    tracing::info!("Listening for execution conditions...");
    //Listen for new blocks to be published. On every block, check for sync logs, update weights and run bellman ford
    while let Some(block) = block_stream.next().await {
        let block_number = block.number.expect("Could not unwrap block number");

        if last_synced_block < block_number {
            let current_block_number = middleware
                .get_block_number()
                .await
                .map_err(ExecutorError::MiddlewareError)?;

            tracing::info!("Checking block {:?}", current_block_number);

            let (order_events, pool_events) = events::sort_events(
                &middleware
                    .get_logs(
                        &block_filter
                            .clone()
                            .from_block(last_synced_block)
                            .to_block(current_block_number),
                    )
                    .await
                    .map_err(ExecutorError::MiddlewareError)?,
                &event_sig_to_belt_event,
            );

            last_synced_block = current_block_number;

            //Handle order updates
            let mut affected_markets = state
                .handle_order_updates(
                    order_events,
                    configuration.sandbox_limit_order_book,
                    configuration.limit_order_book,
                    configuration.weth_address,
                    &configuration.dexes,
                    middleware.clone(),
                )
                .await?;

            //Update markets
            affected_markets.extend(state.handle_market_updates(&pool_events));

            //Check orders for cancellation
            if configuration.order_cancellation {
                cancellation::check_orders_for_cancellation(
                    &configuration,
                    &state,
                    block.timestamp,
                    pending_transactions_sender.clone(),
                    middleware.clone(),
                )
                .await?;
            }

            //Check orders that are ready to be refreshed and send a refresh tx
            if configuration.order_refresh {
                refresh::check_orders_for_refresh(
                    &configuration,
                    &state,
                    block.timestamp,
                    pending_transactions_sender.clone(),
                    middleware.clone(),
                )
                .await?;
            }

            //Evaluate orders for execution
            if !affected_markets.is_empty() {
                execution::fill_orders_at_execution_price(
                    &configuration,
                    &state,
                    affected_markets,
                    pending_transactions_sender.clone(),
                    middleware.clone(),
                )
                .await?;
            }
        }
    }
    Ok(())
}
