use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
    time::Duration,
};

use ethers::{
    providers::{JsonRpcClient, Middleware, Provider},
    types::H256,
};

//TODO: pass in sleep time for checking transactions
//TODO: pass in pending order ids
pub async fn handle_pending_transactions<P: 'static + JsonRpcClient>(
    pending_order_ids: Arc<Mutex<HashSet<H256>>>,
    pending_tx_interval: Duration,
    provider: Arc<Provider<P>>,
) -> tokio::sync::mpsc::Sender<(H256, Vec<H256>)> {
    let (tx, mut rx): (
        tokio::sync::mpsc::Sender<(H256, Vec<H256>)>,
        tokio::sync::mpsc::Receiver<(H256, Vec<H256>)>,
    ) = tokio::sync::mpsc::channel(32);

    let pending_transactions: Arc<Mutex<HashMap<H256, Vec<H256>>>> =
        Arc::new(Mutex::new(HashMap::new()));

    //Make a clone of the pending transactions Arc for both threads that will be spun up below
    let pending_transactions_0 = pending_transactions.clone();
    let pending_transactions_1 = pending_transactions.clone();

    //Spin up a thread that receives new pending transactions
    tokio::spawn(async move {
        while let Some(pending_transaction) = rx.recv().await {
            pending_transactions_0
                .lock()
                .expect("Could not acquire lock on pending transactions")
                .insert(pending_transaction.0, pending_transaction.1);
        }
    });

    let provider = provider.clone();
    //Spin up a thread that checks each pending transaction to see if it has been completed
    tokio::spawn(async move {
        loop {
            let pending_transactions = pending_transactions_1
                .lock()
                .expect("Could not acquire lock on pending transactions")
                .clone();

            for pending_transaction in pending_transactions {
                match provider
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
                    Err(err) => {
                        //TODO: handle the provider error
                    }
                }
            }

            tokio::time::sleep(Duration::new(0, 1000)).await;
        }
    });

    tx
}
