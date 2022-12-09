use ethers::{
    abi::ethabi::Bytes,
    types::{H160, H256},
};

pub trait ExecutionCalldata {
    fn to_bytes() -> Bytes;
}

pub struct LimitOrderExecutionCalldata {
    pub order_ids: Vec<H256>, // bytes32[] calldata orderIds
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
