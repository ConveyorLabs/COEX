use std::sync::Arc;

use ethers::{
    providers::Middleware,
    types::{H160, H256, U256},
};

use crate::{
    abi::{self, ISandboxLimitOrderBook},
    config::Config,
    error::ExecutorError,
    orders::{
        self,
        order::{Order, OrderVariant},
    },
    state::State,
    transaction_utils,
};

pub const THIRTY_DAYS_IN_SECONDS: U256 = U256([39395328, 0, 0, 0]);

pub async fn check_orders_for_refresh<M: Middleware>(
    configuration: &Config,
    state: &State,
    block_timestamp: U256,
    pending_transactions_sender: Arc<tokio::sync::mpsc::Sender<(H256, Vec<H256>)>>,
    middleware: Arc<M>,
) -> Result<(), ExecutorError<M>> {
    //TODO: make this async
    for (order_id, order) in state.active_orders.iter() {
        if block_timestamp - U256::from(order.last_refresh_timestamp()) >= THIRTY_DAYS_IN_SECONDS {
            let order_variant = match order {
                Order::LimitOrder(_) => OrderVariant::LimitOrder,
                Order::SandboxLimitOrder(_) => OrderVariant::SandboxLimitOrder,
            };

            //The order id is inserted into a vec to be passed into the refreshOrder function as well as passed into the pending transactions
            let order_ids = vec![*order_id];

            let tx = transaction_utils::construct_and_simulate_refresh_order_transaction(
                configuration,
                &order_ids,
                order_variant,
                middleware.clone(),
            )
            .await?;

            let pending_tx_hash = transaction_utils::sign_and_send_transaction(
                tx,
                &configuration.wallet_key,
                &configuration.chain,
                middleware.clone(),
            )
            .await?;

            pending_transactions_sender
                .send((pending_tx_hash, order_ids))
                .await?;
        }
    }

    Ok(())
}
