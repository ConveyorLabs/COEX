use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
    time::Duration,
};

use ethers::{
    providers::{Middleware, PendingTransaction},
    signers::LocalWallet,
    types::{
        transaction::eip2718::TypedTransaction, Bytes, Eip1559TransactionRequest, TraceType,
        TransactionRequest, H160, H256,
    },
};
use sha3::digest::KeyInit;
use tokio::time::sleep;

use crate::{
    abi::{self, SandboxMulticall},
    config::{self, Chain},
    error::ExecutorError,
    execution,
};

//TODO: pass in sleep time for checking transactions
//TODO: pass in pending order ids
pub async fn initialize_pending_transaction_handler<M: 'static + Middleware>(
    pending_order_ids: Arc<Mutex<HashSet<H256>>>,
    pending_tx_interval: Duration,
    middleware: Arc<M>,
) -> tokio::sync::mpsc::Sender<(H256, Vec<H256>)> {
    let (tx, mut rx): (
        tokio::sync::mpsc::Sender<(H256, Vec<H256>)>,
        tokio::sync::mpsc::Receiver<(H256, Vec<H256>)>,
    ) = tokio::sync::mpsc::channel(32);

    let pending_transactions: Arc<Mutex<HashMap<H256, Vec<H256>>>> =
        Arc::new(Mutex::new(HashMap::new()));

    //Make a clone of the pending transactions Arc for both threads that will be spun up below
    let pending_transactions_0 = pending_transactions.clone();
    let pending_transactions_1 = pending_transactions;

    //Spin up a thread that receives new pending transactions
    tokio::spawn(async move {
        while let Some(pending_transaction) = rx.recv().await {
            pending_transactions_0
                .lock()
                .expect("Could not acquire lock on pending transactions")
                .insert(pending_transaction.0, pending_transaction.1);
        }
    });

    let middleware = middleware;
    //Spin up a thread that checks each pending transaction to see if it has been completed
    tokio::spawn(async move {
        loop {
            let pending_transactions = pending_transactions_1
                .lock()
                .expect("Could not acquire lock on pending transactions")
                .clone();

            for pending_transaction in pending_transactions {
                match middleware
                    .get_transaction_receipt(pending_transaction.0.to_owned())
                    .await
                {
                    Ok(tx_receipt) => {
                        if tx_receipt.is_some() {
                            pending_transactions_1
                                .lock()
                                .expect("Could not acquire lock on pending transactions")
                                .remove(&pending_transaction.0);

                            for order_id in pending_transaction.1 {
                                pending_order_ids
                                    .lock()
                                    .expect("Could not acquire lock on pending_order_ids")
                                    .remove(&order_id);
                            }
                        }
                    }
                    Err(_err) => {
                        //TODO: handle the middleware error
                    }
                }
            }

            tokio::time::sleep(pending_tx_interval).await;
        }
    });

    tx
}

//TODO: change this to construct execution transaction, pass in calldata and execution address,
//TODO: this way we can simulate the tx with the same contract instance that made the calldata
//Construct a limit order execution transaction
pub async fn construct_and_simulate_lo_execution_transaction<M: Middleware>(
    configuration: &config::Config,
    order_ids: Vec<[u8; 32]>,
    middleware: Arc<M>,
) -> Result<TypedTransaction, ExecutorError<M>> {
    let calldata = abi::ILimitOrderRouter::new(configuration.limit_order_book, middleware.clone())
        .execute_limit_orders(order_ids)
        .calldata()
        .unwrap();

    if configuration.chain.is_eip1559() {
        //:: EIP 1559 transaction

        let tx = fill_and_simulate_transaction(
            calldata,
            configuration.limit_order_book,
            configuration.wallet_address,
            configuration.chain.chain_id(),
            middleware.clone(),
        )
        .await?;

        Ok(tx)
    } else {
        //TODO: legacy transaction
        let tx = fill_and_simulate_transaction(
            calldata,
            configuration.limit_order_book,
            configuration.wallet_address,
            configuration.chain.chain_id(),
            middleware.clone(),
        )
        .await?;

        // println!("tx: {:#?}", tx);

        Ok(tx)
    }
}

//TODO: change this to construct execution transaction, pass in calldata and execution address,
//TODO: this way we can simulate the tx with the same contract instance that made the calldata
//Construct a limit order execution transaction
pub async fn construct_and_simulate_slo_execution_transaction<M: Middleware>(
    configuration: &config::Config,
    slo_bundle: execution::sandbox_limit_order::SandboxLimitOrderExecutionBundle,
    middleware: Arc<M>,
) -> Result<TypedTransaction, ExecutorError<M>> {
    let sandbox_limit_order_router = abi::ISandboxLimitOrderRouter::new(
        configuration.sandbox_limit_order_router,
        middleware.clone(),
    );
    let calldata = sandbox_limit_order_router
        .execute_sandbox_multicall(slo_bundle.to_sandbox_multicall())
        .calldata()
        .unwrap();

    if configuration.chain.is_eip1559() {
        let tx = fill_and_simulate_transaction(
            calldata,
            configuration.sandbox_limit_order_router,
            configuration.wallet_address,
            configuration.chain.chain_id(),
            middleware.clone(),
        )
        .await?;

        Ok(tx)
    } else {
        //TODO: legacy transactions
        let tx = fill_and_simulate_transaction(
            calldata,
            configuration.limit_order_book,
            configuration.wallet_address,
            configuration.chain.chain_id(),
            middleware.clone(),
        )
        .await?;

        // println!("tx: {:#?}", tx);

        Ok(tx)
    }
}

//Signs and sends transaction, bumps gas if necessary
pub async fn sign_and_send_transaction<M: Middleware>(
    mut tx: TypedTransaction,
    wallet_key: &LocalWallet,
    chain: &Chain,
    middleware: Arc<M>,
) -> Result<H256, ExecutorError<M>> {
    let mut signed_tx = raw_signed_transaction(tx.clone(), wallet_key);
    loop {
        match middleware.send_raw_transaction(signed_tx.clone()).await {
            Ok(pending_tx) => {
                return Ok(pending_tx.tx_hash());
            }
            Err(err) => {
                let error_string = err.to_string();
                if error_string.contains("transaction underpriced") {
                    println!("Bumping gas");
                    if chain.is_eip1559() {
                        let eip1559_tx = tx.as_eip1559_mut().unwrap();
                        eip1559_tx.max_priority_fee_per_gas =
                            Some(eip1559_tx.max_priority_fee_per_gas.unwrap() * 150 / 100);
                        eip1559_tx.max_fee_per_gas =
                            Some(eip1559_tx.max_fee_per_gas.unwrap() * 150 / 100);

                        //TODO: remove this, just for throttling
                        sleep(Duration::new(1, 0)).await;

                        tx = eip1559_tx.to_owned().into();

                        signed_tx = raw_signed_transaction(tx.clone(), wallet_key);
                    }
                } else if error_string.contains("insufficient funds") {
                    return Err(ExecutorError::InsufficientWalletFunds());
                } else {
                    tracing::error!("{:?}", error_string);
                    return Err(err).map_err(ExecutorError::MiddlewareError);
                }
            }
        }
    }
}

async fn fill_and_simulate_transaction<M: Middleware>(
    calldata: Bytes,
    to: H160,
    from: H160,
    chain_id: usize,
    middleware: Arc<M>,
) -> Result<TypedTransaction, ExecutorError<M>> {
    let (max_fee_per_gas, max_priority_fee_per_gas) = middleware
        .estimate_eip1559_fees(None)
        .await
        .map_err(ExecutorError::MiddlewareError)?;

    let mut tx: TypedTransaction = Eip1559TransactionRequest::new()
        .data(calldata.clone())
        .to(to)
        .from(from)
        .chain_id(chain_id)
        .max_priority_fee_per_gas(max_priority_fee_per_gas)
        .max_fee_per_gas(max_fee_per_gas)
        .into();

    println!("Getting right here");

    //   Simulate the tx
    //TODO: handle legacy transactions

    //match fill transaction, it will fail if the calldata fails
    middleware
        .fill_transaction(&mut tx, None)
        .await
        .map_err(ExecutorError::MiddlewareError)?;
    println!("");
    println!("Getting right here2");

    tx.set_gas(tx.gas().unwrap() * 150 / 100);

    Ok(tx)
}

//Construct a sandbox limit order execution transaction
pub async fn construct_slo_execution_transaction<M: 'static + Middleware>(
    execution_address: H160,
    data: Bytes,
    middleware: Arc<M>,
    chain: &Chain,
) -> Result<TypedTransaction, ExecutorError<M>> {
    //TODO: For the love of god, refactor the transaction composition

    match chain {
        //:: EIP 1559 transaction
        Chain::Ethereum | Chain::Polygon | Chain::Optimism | Chain::Arbitrum => {
            let tx = Eip1559TransactionRequest::new()
                .to(execution_address)
                .data(data)
                .into();

            Ok(tx)
        }

        //:: Legacy transaction
        Chain::BSC | Chain::Cronos => {
            let tx = TransactionRequest::new().to(execution_address).data(data);

            let gas_price = middleware
                .get_gas_price()
                .await
                .map_err(ExecutorError::MiddlewareError)?;

            let tx = tx.gas_price(gas_price);

            let mut tx: TypedTransaction = tx.into();
            let gas_limit = middleware
                .estimate_gas(&tx)
                .await
                .map_err(ExecutorError::MiddlewareError)?;

            tx.set_gas(gas_limit);

            Ok(tx)
        }
    }
}

pub fn raw_signed_transaction(tx: TypedTransaction, wallet_key: &LocalWallet) -> Bytes {
    tx.rlp_signed(&wallet_key.sign_transaction_sync(&tx))
}
