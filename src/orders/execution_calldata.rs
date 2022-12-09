use ethers::{
    abi::{ethabi::Bytes, Token},
    providers::JsonRpcClient,
    types::{H160, H256},
};

use crate::error::BeltError;

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
    pub order_ids: Vec<H256>, // bytes32[] calldata orderIds
}

impl LimitOrderExecutionOrderIds {
    pub fn new() -> LimitOrderExecutionOrderIds {
        LimitOrderExecutionOrderIds::default()
    }

    pub fn add_order_id(&mut self, order_id: H256) {
        self.order_ids.push(order_id);
    }
}

impl ExecutionCalldata for LimitOrderExecutionOrderIds {
    fn to_bytes(&self) -> Bytes {
        ethers::abi::encode(
            &self
                .order_ids
                .iter()
                .map(|order_id| Token::FixedBytes(order_id.as_fixed_bytes().to_vec()))
                .collect::<Vec<Token>>(),
        )
    }
}
