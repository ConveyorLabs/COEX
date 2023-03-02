use std::sync::Arc;

use ethers::{providers::Middleware, signers::LocalWallet, types::H160};

use crate::{abi, config::Chain, error::ExecutorError, transaction_utils};

pub async fn start_check_in_service<M: Middleware>(
    check_in_address: H160,
    wallet_address: H160,
    wallet_key: &LocalWallet,
    chain: &Chain,
    middleware: Arc<M>,
) -> Result<(), ExecutorError<M>> {
    //Check when the last check in was

    let check_in_contract = abi::IConveyorExecutor::new(check_in_address, middleware.clone());

    let last_check_in = check_in_contract
        .last_check_in(wallet_address)
        .call()
        .await?;

    let block_timestamp = get_block_timestamp(middleware.clone()).await?;

    //If the last check in was past the threshold, check in

    transaction_utils::sign_and_send_transaction(tx, wallet_key, chain, middleware).await?;

    loop {}

    //Calc the sleep time

    //Sleep and await

    Ok(())
}

pub async fn get_block_timestamp<M: Middleware>(
    middleware: Arc<M>,
) -> Result<u64, ExecutorError<M>> {
    loop {
        if let Some(block) = middleware
            .get_block(
                middleware
                    .get_block_number()
                    .await
                    .map_err(ExecutorError::MiddlewareError)?,
            )
            .await
            .map_err(ExecutorError::MiddlewareError)?
        {
            return Ok(block.timestamp.as_u64());
        }
    }
}
