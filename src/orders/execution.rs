use std::sync::Arc;

use ethers::{
    abi::{ethabi::Bytes, Token},
    providers::{JsonRpcClient, Middleware, Provider},
    signers::LocalWallet,
    types::{
        transaction::eip2718::TypedTransaction, Eip1559TransactionRequest, TransactionRequest,
        H160, H256,
    },
};

use crate::{abi, config::Chain, error::BeltError};

use super::order;

pub trait ExecutionCalldata {
    fn to_bytes(&self) -> Bytes;
}

pub struct SandboxLimitOrderExecutionCalldata {
    pub order_id_bundles: Vec<Vec<H256>>, //bytes32[][] orderIdBundles
    pub fill_amounts: Vec<u128>,          // uint128[] fillAmounts
    pub transfer_addresses: Vec<H160>,    // address[] transferAddresses
    pub calls: Vec<Call>,                 // Call[] calls
}

pub struct Call {
    pub target: H160,       // address target
    pub call_data: Vec<u8>, // bytes callData
}

#[derive(Default)]
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
        self.order_groups
            .push(LimitOrderExecutionOrderIds::default());
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

#[derive(Default)]
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

//Construct a sandbox limit order execution transaction
pub async fn construct_signed_slo_execution_transaction<P: 'static + JsonRpcClient>(
    execution_address: H160,
    data: Bytes,
    provider: Arc<Provider<P>>,
    wallet: LocalWallet,
    chain: &Chain,
) -> Result<TransactionRequest, BeltError<P>> {
    //TODO: For the love of god, refactor the transaction composition

    match chain {
        //:: EIP 1559 transaction
        Chain::Ethereum | Chain::Polygon | Chain::Optimism | Chain::Arbitrum => {
            let tx = Eip1559TransactionRequest::new()
                .to(execution_address)
                .data(data);

            //Update transaction gas fees
            let (max_priority_fee_per_gas, max_fee_per_gas) =
                provider.estimate_eip1559_fees(None).await?;
            let tx = tx.max_priority_fee_per_gas(max_priority_fee_per_gas);
            let tx = tx.max_fee_per_gas(max_fee_per_gas);

            let mut tx: TypedTransaction = tx.into();
            let gas_limit = provider.estimate_gas(&tx).await?;
            tx.set_gas(gas_limit);

            wallet.sign_transaction_sync(&tx);

            Ok(tx.into())
        }

        //:: Legacy transaction
        Chain::BSC | Chain::Cronos => {
            let tx = TransactionRequest::new().to(execution_address).data(data);

            let gas_price = provider.get_gas_price().await?;
            let tx = tx.gas_price(gas_price);

            let mut tx: TypedTransaction = tx.into();
            let gas_limit = provider.estimate_gas(&tx).await?;
            tx.set_gas(gas_limit);

            wallet.sign_transaction_sync(&tx);

            Ok(tx.into())
        }
    }
}

//Construct a limit order execution transaction
pub async fn construct_signed_lo_execution_transaction<P: 'static + JsonRpcClient>(
    execution_address: H160,
    order_ids: Vec<[u8; 32]>,
    wallet: Arc<LocalWallet>,
    provider: Arc<Provider<P>>,
    chain: &Chain,
) -> Result<TransactionRequest, BeltError<P>> {
    //TODO: For the love of god, refactor the transaction composition

    let calldata = abi::ILimitOrderRouter::new(execution_address, provider.clone())
        .execute_orders(order_ids)
        .calldata()
        .unwrap();

    match chain {
        //:: EIP 1559 transaction
        Chain::Ethereum | Chain::Polygon | Chain::Optimism | Chain::Arbitrum => {
            let tx = Eip1559TransactionRequest::new()
                .to(execution_address)
                .data(calldata);

            //Update transaction gas fees
            let (max_priority_fee_per_gas, max_fee_per_gas) =
                provider.estimate_eip1559_fees(None).await?;
            let tx = tx.max_priority_fee_per_gas(max_priority_fee_per_gas);
            let tx = tx.max_fee_per_gas(max_fee_per_gas);

            let mut tx: TypedTransaction = tx.into();
            let gas_limit = provider.estimate_gas(&tx).await?;
            tx.set_gas(gas_limit);

            wallet.sign_transaction_sync(&tx);

            Ok(tx.into())
        }

        //:: Legacy transaction
        Chain::BSC | Chain::Cronos => {
            let tx = TransactionRequest::new()
                .to(execution_address)
                .data(calldata);

            let gas_price = provider.get_gas_price().await?;
            let tx = tx.gas_price(gas_price);

            let mut tx: TypedTransaction = tx.into();
            let gas_limit = provider.estimate_gas(&tx).await?;
            tx.set_gas(gas_limit);

            wallet.sign_transaction_sync(&tx);

            Ok(tx.into())
        }
    }
}
