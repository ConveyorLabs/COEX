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
use ethers::types::{H256, U256};

//TODO: move this to bin

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    traces::init_tracing();

    let (configuration, state, pending_transactions_sender, stream_provider_endpoint, middleware) =
        initialize_coex::<NonceManagerMiddleware<ethers::providers::Provider<Http>>>()
            .await
            .unwrap();

    check_in::spawn_check_in_service(
        configuration.executor_address,
        configuration.wallet_address,
        configuration.wallet_key.clone(),
        configuration.chain,
        middleware.clone(),
    )
    .await?;

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
        middleware,
        stream_provider_endpoint,
        state,
        pending_transactions_sender,
    )
    .await?;

    Ok(())
}

async fn run_loop<M: 'static + Middleware>(
    configuration: config::Config,
    middleware: Arc<M>,
    stream_provider_endpoint: String,
    mut state: state::State,
    pending_transactions_sender: Arc<tokio::sync::mpsc::Sender<(H256, Vec<H256>)>>,
) -> Result<(), ExecutorError<M>> {
    loop {
        if let Ok(stream_provider) = Provider::<Ws>::connect(stream_provider_endpoint.clone()).await
        {
            let mut block_stream = stream_provider.subscribe_blocks().await?;
            let block_filter = events::initialize_block_filter(&configuration.dexes);
            //Get a mapping of event signature to event for quick lookup
            let event_sig_to_belt_event = events::get_event_signature_to_belt_event();

            tracing::info!("Listening for execution conditions...");
            //Listen for new blocks to be published. On every block, check for sync logs, update weights and run bellman ford
            while let Some(block) = block_stream.next().await {
                tracing::info!("Checking block {:?}", block.number.unwrap());

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
                state
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
                let markets_updated = state.handle_market_updates(&pool_events);

                // //Check orders for cancellation
                // if configuration.order_cancellation {
                //     order_cancellation::check_orders_for_cancellation(
                //         &configuration,
                //         &state,
                //         block.timestamp,
                //         pending_transactions_sender.clone(),
                //         middleware.clone(),
                //     )
                //     .await?;
                // }

                // //Check orders that are ready to be refreshed and send a refresh tx
                // if configuration.order_refresh {
                //     order_refresh::check_orders_for_refresh(
                //         &configuration,
                //         &state,
                //         block.timestamp,
                //         pending_transactions_sender.clone(),
                //         middleware.clone(),
                //     )
                //     .await?;
                // }

                //Evaluate orders for execution
                if !markets_updated.is_empty() {
                    execution::fill_orders_at_execution_price(
                        &configuration,
                        &state,
                        markets_updated,
                        pending_transactions_sender.clone(),
                        middleware.clone(),
                    )
                    .await?;
                }
            }
        }
    }
}
