use std::sync::Arc;

use crate::error::ExecutorError;
use crate::{config, transaction_utils};

use super::ExecutionCalldata;
use ethers::abi::ethabi::Bytes;
use ethers::abi::Token;
use ethers::providers::Middleware;
use ethers::types::H256;

#[derive(Default, Debug)]
pub struct LimitOrderExecutionBundle {
    pub order_groups: Vec<LimitOrderExecutionOrderIds>, // bytes32[] calldata orderIds
}

impl LimitOrderExecutionBundle {
    pub fn new() -> LimitOrderExecutionBundle {
        LimitOrderExecutionBundle::default()
    }

    pub fn add_order_group(&mut self, order_group: LimitOrderExecutionOrderIds) {
        self.order_groups.push(order_group);
    }

    pub fn add_empty_order_group(&mut self) {
        if let Some(order_group) = self.order_groups.last() {
            if !order_group.order_ids.is_empty() {
                self.order_groups
                    .push(LimitOrderExecutionOrderIds::default());
            }
        } else {
            self.order_groups
                .push(LimitOrderExecutionOrderIds::default());
        };
    }

    pub fn append_order_id_to_latest_order_group(&mut self, order_id: H256) {
        if let Some(order_group) = self.order_groups.last_mut() {
            order_group.add_order_id(order_id);
        } else {
            self.add_empty_order_group();
            self.append_order_id_to_latest_order_group(order_id);
        }
    }
}

impl ExecutionCalldata for LimitOrderExecutionBundle {
    fn to_bytes(&self) -> Bytes {
        self.order_groups
            .iter()
            .flat_map(|order_group| order_group.to_bytes())
            .collect::<Vec<u8>>()
    }
}

#[derive(Default, Debug)]
pub struct LimitOrderExecutionOrderIds {
    pub order_ids: Vec<[u8; 32]>, // bytes32[] calldata orderIds
}

impl LimitOrderExecutionOrderIds {
    pub fn new() -> LimitOrderExecutionOrderIds {
        LimitOrderExecutionOrderIds::default()
    }

    pub fn add_order_id(&mut self, order_id: H256) {
        self.order_ids.push(order_id.to_fixed_bytes());
    }
}

impl ExecutionCalldata for LimitOrderExecutionOrderIds {
    fn to_bytes(&self) -> Bytes {
        ethers::abi::encode(
            &self
                .order_ids
                .iter()
                .map(|order_id| Token::FixedBytes(order_id.to_vec()))
                .collect::<Vec<Token>>(),
        )
    }
}

pub async fn execute_limit_order_groups<M: Middleware>(
    limit_order_execution_bundle: LimitOrderExecutionBundle,
    configuration: &config::Config,
    pending_transactions_sender: Arc<tokio::sync::mpsc::Sender<(H256, Vec<H256>)>>,
    middleware: Arc<M>,
) -> Result<(), ExecutorError<M>> {
    // execute limit orders
    for order_group in limit_order_execution_bundle.order_groups {
        if !order_group.order_ids.is_empty() {
            if let Ok(tx) = transaction_utils::construct_and_simulate_lo_execution_transaction(
                configuration,
                order_group.order_ids.clone(),
                middleware.clone(),
            )
            .await
            {
                let pending_tx_hash = transaction_utils::sign_and_send_transaction(
                    tx,
                    &configuration.wallet_key,
                    &configuration.chain,
                    middleware.clone(),
                )
                .await?;

                tracing::info!("Pending limit order execution tx: {:?}", pending_tx_hash);

                let order_ids = order_group
                    .order_ids
                    .iter()
                    .map(|f| H256::from_slice(f.as_slice()))
                    .collect::<Vec<H256>>();

                pending_transactions_sender
                    .send((pending_tx_hash, order_ids))
                    .await?;
            }
        }
    }

    Ok(())
}
