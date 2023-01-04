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

pub async fn check_orders_for_cancellation<M: Middleware>(
    configuration: &Config,
    state: &State,
    block_timestamp: U256,
    pending_transactions_sender: Arc<tokio::sync::mpsc::Sender<(H256, Vec<H256>)>>,
    middleware: Arc<M>,
) -> Result<(), ExecutorError<M>> {
    //TODO: make this async
    for (order_id, order) in state.active_orders.iter() {
        let owner_balance = abi::IErc20::new(order.token_in(), middleware.clone())
            .balance_of(order.owner())
            .call()
            .await?;

        if order.amount_in() > owner_balance.as_u128()
            || U256::from(order.expiration_timestamp()) <= block_timestamp
        {
            let order_variant = match order {
                Order::LimitOrder(_) => OrderVariant::LimitOrder,
                Order::SandboxLimitOrder(_) => OrderVariant::SandboxLimitOrder,
            };

            let tx = transaction_utils::construct_and_simulate_cancel_order_transaction(
                configuration,
                *order_id,
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
                .send((pending_tx_hash, vec![*order_id]))
                .await?;
        }
    }

    Ok(())
}
