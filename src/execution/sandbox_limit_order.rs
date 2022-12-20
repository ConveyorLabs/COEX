use ethers::abi::ethabi::Bytes;
use ethers::types::{H160, H256};

use crate::abi;

#[derive(Debug, Default)]
pub struct SandboxLimitOrderExecutionBundle {
    order_id_bundle_idx: usize,
    pub order_id_bundles: Vec<Vec<H256>>, //bytes32[][] orderIdBundles
    pub fill_amounts: Vec<u128>,          // uint128[] fillAmounts
    pub transfer_addresses: Vec<H160>,    // address[] transferAddresses
    pub calls: Vec<Call>,                 // Call[] calls
}

impl SandboxLimitOrderExecutionBundle {
    pub fn to_sandbox_multicall(self) -> abi::SandboxMulticall {
        let order_id_bundles: Vec<Vec<[u8; 32]>> = self
            .order_id_bundles
            .iter()
            .map(|bundle| {
                bundle
                    .iter()
                    .map(|order_id| order_id.as_fixed_bytes().to_owned())
                    .collect()
            })
            .collect();

        let calls: Vec<abi::Call> = self
            .calls
            .iter()
            .map(|call| abi::Call {
                target: call.target,
                call_data: ethers::types::Bytes::from(call.call_data.to_owned()),
            })
            .collect();

        abi::SandboxMulticall {
            order_id_bundles,
            fill_amounts: self.fill_amounts,
            transfer_addresses: self.transfer_addresses,
            calls: calls,
        }
    }
}

#[derive(Debug, Default)]
pub struct Call {
    pub target: H160,       // address target
    pub call_data: Vec<u8>, // bytes callData
}

impl SandboxLimitOrderExecutionBundle {
    pub fn new() -> SandboxLimitOrderExecutionBundle {
        let mut execution_bundle = SandboxLimitOrderExecutionBundle::default();
        execution_bundle.add_new_order_id_bundle();

        execution_bundle
    }

    pub fn add_order_id_to_current_bundle(&mut self, order_id: H256) {
        self.order_id_bundles[self.order_id_bundle_idx].push(order_id);
    }

    pub fn add_new_order_id_bundle(&mut self) {
        self.order_id_bundles.push(vec![]);
        self.order_id_bundle_idx += 1;
    }

    pub fn add_fill_amount(&mut self, fill_amount: u128) {
        self.fill_amounts.push(fill_amount);
    }

    pub fn add_transfer_address(&mut self, transfer_address: H160) {
        self.transfer_addresses.push(transfer_address);
    }

    pub fn add_call(&mut self, call: Call) {
        self.calls.push(call);
    }
}

impl Call {
    pub fn new(target: H160, call_data: Bytes) -> Call {
        Call { target, call_data }
    }
}
