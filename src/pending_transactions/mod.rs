use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};

use ethers::{
    providers::{JsonRpcClient, Middleware, PendingTransaction, Provider},
    types::H256,
};

//TODO: pass in sleep time for checking transactions
//TODO: pass in pending order ids
pub async fn handle_pending_transactions<P: 'static + JsonRpcClient>(
    provider: Arc<Provider<P>>,
) -> tokio::sync::mpsc::Sender<H256> {
    let (tx, mut rx) = tokio::sync::mpsc::channel(32);

    let pending_transactions: Arc<Mutex<HashSet<H256>>> = Arc::new(Mutex::new(HashSet::new()));

    //Make a clone of the pending transactions Arc for both threads that will be spun up below
    let pending_transactions_0 = pending_transactions.clone();
    let pending_transactions_1 = pending_transactions.clone();

    //Spin up a thread that receives new pending transactions
    tokio::spawn(async move {
        while let Some(pending_tx_hash) = rx.recv().await {
            pending_transactions_0
                .lock()
                .expect("Could not aquire lock on pending transactions")
                .insert(pending_tx_hash);
        }
    });

    let provider = provider.clone();
    //Spin up a thread that checks each pending transaction to see if it has been completed
    tokio::spawn(async move {
        loop {
            let mut pending_tx_hashes = vec![];

            for tx_hash in pending_transactions_1
                .lock()
                .expect("Could not aquire lock on pending transactions")
                .iter()
            {
                pending_tx_hashes.push(tx_hash.to_owned());
            }

            for tx_hash in pending_tx_hashes {
                match provider.get_transaction_receipt(tx_hash).await {
                    Ok(tx_receipt) => {
                        if tx_receipt.is_some() {
                            pending_transactions_1
                                .lock()
                                .expect("Could not aquire lock on pending transactions")
                                .remove(&tx_hash);

                            //TODO: remove pending order Id
                        }
                    }
                    Err(err) => {
                        //TODO: handle the provider error
                    }
                }
            }

            //TODO: sleep if any
        }
    });

    tx
}
