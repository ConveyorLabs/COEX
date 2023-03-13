use std::{sync::Arc, time::Duration};

use cfmms::pool::Pool;
use ethers::{
    abi::ethabi::Bytes,
    providers::Middleware,
    signers::LocalWallet,
    types::{H160, U256},
};

use crate::{abi, config::Chain, error::ExecutorError, transaction_utils};

pub const CHECK_IN_WAIT_TIME: u64 = 43200;

pub async fn spawn_check_in_service<M: 'static + Middleware>(
    check_in_address: H160,
    wallet_address: H160,
    wallet_key: LocalWallet,
    chain: Chain,
    middleware: Arc<M>,
) -> Result<(), ExecutorError<M>> {
    //Handle the initial check in
    initial_check_in(
        check_in_address,
        wallet_address,
        wallet_key.clone(),
        chain,
        false,
        middleware.clone(),
    )
    .await?;

    tokio::spawn(async move {
        match start_check_in_service(
            check_in_address,
            wallet_address,
            wallet_key.clone(),
            chain.clone(),
            middleware.clone(),
        )
        .await
        {
            Ok(_) => {}
            Err(e) => {
                tracing::error!("Error in check in service: {:?}", e);
            }
        }
    });

    Ok(())
}

pub async fn initial_check_in<M: Middleware>(
    check_in_address: H160,
    wallet_address: H160,
    wallet_key: LocalWallet,
    chain: Chain,
    middleware: Arc<M>,
) -> Result<(), ExecutorError<M>> {
    let check_in_contract = abi::IConveyorExecutor::new(check_in_address, middleware.clone());

    let last_check_in: U256 = check_in_contract
        .last_check_in(wallet_address)
        .call()
        .await?;

    let block_timestamp = get_block_timestamp(middleware.clone()).await?;

    let time_elapsed = block_timestamp - last_check_in.as_u64();

    if time_elapsed >= CHECK_IN_WAIT_TIME {
        tracing::info!("Check in time elapsed, checking in");

        //submit a check in tx with retries
        let tx = transaction_utils::fill_and_simulate_transaction(
            abi::ICONVEYOREXECUTOR_ABI
                .function("checkIn")
                .unwrap()
                .encode_input(&[])
                .expect("Failed to encode checkIn input")
                .into(),
            check_in_address,
            wallet_address,
            chain.chain_id(),
            middleware.clone(),
        )
        .await?;

        let tx_hash = transaction_utils::sign_and_send_transaction(
            tx,
            &wallet_key,
            &chain,
            middleware.clone(),
        )
        .await?;

        tracing::info!("Pending check in tx: {:?}", tx_hash);

        loop {
            if let Ok(tx_receipt) = middleware.get_transaction_receipt(tx_hash).await {
                if tx_receipt.is_some() {
                    tracing::info!("Successfully checked in");
                    break;
                }
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }

    Ok(())
}

pub async fn check_in<M: Middleware>(
    check_in_address: H160,
    wallet_address: H160,
    wallet_key: LocalWallet,
    chain: Chain,
    middleware: Arc<M>,
) -> Result<(), ExecutorError<M>> {
    let check_in_contract = abi::IConveyorExecutor::new(check_in_address, middleware.clone());

    let last_check_in: U256 = check_in_contract
        .last_check_in(wallet_address)
        .call()
        .await?;
    tracing::info!("Last check in at {:?}", last_check_in);

    let block_timestamp = get_block_timestamp(middleware.clone()).await?;

    let time_elapsed = block_timestamp - last_check_in.as_u64();

    if time_elapsed >= CHECK_IN_WAIT_TIME {
        tracing::info!("Check in time elapsed, checking in");

        //submit a check in tx with retries
        let tx = transaction_utils::fill_and_simulate_transaction(
            abi::ICONVEYOREXECUTOR_ABI
                .function("checkIn")
                .unwrap()
                .encode_input(&[])
                .expect("Failed to encode checkIn input")
                .into(),
            check_in_address,
            wallet_address,
            chain.chain_id(),
            middleware.clone(),
        )
        .await?;

        let tx_hash = transaction_utils::sign_and_send_transaction(
            tx,
            &wallet_key,
            &chain,
            middleware.clone(),
        )
        .await?;

        tracing::info!("Pending check in tx: {:?}", tx_hash);

        loop {
            if let Ok(tx_receipt) = middleware.get_transaction_receipt(tx_hash).await {
                if tx_receipt.is_some() {
                    tracing::info!("Successfully checked in");
                    break;
                }
            }

            tokio::time::sleep(Duration::from_secs(1)).await;
        }

        tracing::info!("Waiting {:?} until next check in", CHECK_IN_WAIT_TIME);
        tokio::time::sleep(Duration::from_secs(CHECK_IN_WAIT_TIME)).await;
    } else {
        tracing::info!(
            "Waiting {:?} until next check in",
            CHECK_IN_WAIT_TIME - time_elapsed
        );

        tokio::time::sleep(Duration::from_secs(CHECK_IN_WAIT_TIME - time_elapsed)).await;
    }

    Ok(())
}

pub async fn start_check_in_service<M: Middleware>(
    check_in_address: H160,
    wallet_address: H160,
    wallet_key: LocalWallet,
    chain: Chain,
    middleware: Arc<M>,
) -> Result<(), ExecutorError<M>> {
    //Check when the last check in was

    loop {
        check_in(
            check_in_address,
            wallet_address,
            wallet_key.clone(),
            chain,
            middleware.clone(),
        )
        .await?;
    }
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
