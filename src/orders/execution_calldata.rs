use ethers::{
    abi::ethabi::Bytes,
    types::{H160, H256},
};

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

pub struct LimitOrderExecutionCalldata {
    pub order_ids: Vec<H256>, // bytes32[] calldata orderIds
}

impl LimitOrderExecutionCalldata {
    pub fn add_order_id(&mut self, order_id: H256) {
        self.order_ids.push(order_id);
    }
}

impl ExecutionCalldata for LimitOrderExecutionCalldata {
    fn to_bytes(&self) -> Bytes {
        let mut calldata = Bytes::new();
        for order_id in &self.order_ids {
            calldata.append(&mut Bytes::from(order_id.as_bytes()))
        }

        calldata
    }
}
