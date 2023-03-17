use std::sync::Arc;

use ethers::{
    providers::Middleware,
    types::{H160, H256, U256},
};

use crate::{
    abi::{self, ISandboxLimitOrderBook},
    config::Config,
    error::ExecutorError,
    order::{Order, OrderVariant},
    state::State,
    transactions,
};

pub async fn check_orders_for_cancellation<M: Middleware>(
    configuration: &Config,
    state: &State,
    block_timestamp: U256,
    pending_transactions_sender: Arc<tokio::sync::mpsc::Sender<(H256, Vec<H256>)>>,
    middleware: Arc<M>,
) -> Result<(), ExecutorError<M>> {
    //TODO: We can make this process much faster and lightweight by using a batch contract to get the token in balance for the order in large batches
    //TODO: Then we can handle cancellation as one single group or as async singular transactions to make cancellation profits more distributed across COEXs
    //TODO: Right now this implementation was used to get things on its feet and functional at the very least but this is very slow

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

            let tx = transactions::construct_and_simulate_cancel_order_transaction(
                configuration,
                *order_id,
                order_variant,
                middleware.clone(),
            )
            .await?;

            let pending_tx_hash = transactions::sign_and_send_transaction(
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
