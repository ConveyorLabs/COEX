use std::sync::Arc;

use cfmms::pool::Pool;
use ethers::abi::ethabi::Bytes;
use ethers::abi::{AbiEncode, Token};
use ethers::providers::Middleware;
use ethers::types::{H160, H256, I256, U256};

use crate::error::ExecutorError;
use crate::order::sandbox_limit_order::SandboxLimitOrder;
use crate::{abi, config, transactions};

#[derive(Debug, Default)]

//TODO: rename this to SandboxMulticall but be mindful of abi::SandboxMulticall
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
        execution_bundle.order_id_bundles.push(vec![]);

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

    pub fn add_route_to_calls(
        &mut self,
        route: Vec<Pool>,
        amounts_in: &[U256],
        amounts_out: &[U256],
        order: &SandboxLimitOrder,
        sandbox_limit_order_router: H160,
    ) {
        //Add calls for each swap throughout the route
        let mut token_in = order.token_in;
        for (i, pool) in route.iter().enumerate() {
            match pool {
                Pool::UniswapV2(uniswap_v2_pool) => {
                    let to = if i == route.len() - 1 {
                        sandbox_limit_order_router
                    } else {
                        route[i + 1].address()
                    };

                    self.add_uniswap_v2_swap_to_calls(
                        token_in,
                        amounts_out[i],
                        to,
                        uniswap_v2_pool,
                    );
                }

                Pool::UniswapV3(uniswap_v3_pool) => self.add_uniswap_v3_swap_to_calls(
                    token_in,
                    amounts_in[i],
                    sandbox_limit_order_router,
                    sandbox_limit_order_router,
                    uniswap_v3_pool,
                ),
            }
            //Update the token in
            token_in = self.get_next_token_in(token_in, pool);
        }
    }

    fn get_next_token_in(&self, prev_token_in: H160, pool: &Pool) -> H160 {
        match pool {
            Pool::UniswapV2(uniswap_v2_pool) => {
                if prev_token_in == uniswap_v2_pool.token_a {
                    uniswap_v2_pool.token_b
                } else {
                    uniswap_v2_pool.token_a
                }
            }

            Pool::UniswapV3(uniswap_v3_pool) => {
                if prev_token_in == uniswap_v3_pool.token_a {
                    uniswap_v3_pool.token_b
                } else {
                    uniswap_v3_pool.token_a
                }
            }
        }
    }

    pub fn add_swap_to_calls(
        &mut self,
        token_in: H160,
        amount_in: U256,
        amount_out: U256,
        to: H160,
        from: H160,
        pool: &cfmms::pool::Pool,
    ) {
        match pool {
            Pool::UniswapV2(uniswap_v2_pool) => {
                self.add_uniswap_v2_swap_to_calls(token_in, amount_out, to, uniswap_v2_pool);
            }

            Pool::UniswapV3(uniswap_v3_pool) => {
                self.add_uniswap_v3_swap_to_calls(token_in, amount_in, to, from, uniswap_v3_pool);
            }
        }
    }

    pub fn add_uniswap_v2_swap_to_calls(
        &mut self,
        token_in: H160,
        amount_out: U256,
        to: H160,
        pool: &cfmms::pool::UniswapV2Pool,
    ) {
        let (amount_0_out, amount_1_out) = if pool.token_a == token_in {
            (U256::zero(), amount_out)
        } else {
            (amount_out, U256::zero())
        };

        self.add_call(Call::new(
            pool.address,
            pool.swap_calldata(amount_0_out, amount_1_out, to, vec![]),
        ));
    }

    pub fn add_uniswap_v3_swap_to_calls(
        &mut self,
        token_in: H160,
        amount_in: U256,
        to: H160,
        from: H160,
        pool: &cfmms::pool::UniswapV3Pool,
    ) {
        let zero_for_one = if pool.token_a == token_in {
            true
        } else {
            false
        };

        let sqrt_price_limit_x_96 = if zero_for_one {
            uniswap_v3_math::tick_math::MIN_SQRT_RATIO + 1
        } else {
            uniswap_v3_math::tick_math::MAX_SQRT_RATIO - 1
        };

        let calldata = ethers::abi::encode(&vec![
            Token::Bool(zero_for_one),
            Token::Address(token_in),
            Token::Address(from),
        ]);

        self.add_call(Call::new(
            pool.address,
            pool.swap_calldata(
                to,
                zero_for_one,
                I256::from_raw(amount_in),
                sqrt_price_limit_x_96,
                calldata,
            ),
        ));
    }
}

impl Call {
    pub fn new(target: H160, call_data: Bytes) -> Call {
        Call { target, call_data }
    }
}

pub async fn execute_sandbox_limit_order_bundles<M: Middleware>(
    slo_bundles: Vec<SandboxLimitOrderExecutionBundle>,
    configuration: &config::Config,
    pending_transactions_sender: Arc<tokio::sync::mpsc::Sender<(H256, Vec<H256>)>>,
    middleware: Arc<M>,
) -> Result<(), ExecutorError<M>> {
    for bundle in slo_bundles {
        let order_id_bundles = bundle.order_id_bundles.clone();

        match transactions::construct_and_simulate_slo_execution_transaction(
            configuration,
            bundle,
            middleware.clone(),
        )
        .await
        {
            Ok(tx) => {
                let pending_tx_hash = transactions::sign_and_send_transaction(
                    tx,
                    &configuration.wallet_key,
                    &configuration.chain,
                    middleware.clone(),
                )
                .await?;

                tracing::info!(
                    "Pending sandbox limit order execution tx: {:?}",
                    pending_tx_hash
                );

                for order_ids in order_id_bundles {
                    pending_transactions_sender
                        .send((pending_tx_hash, order_ids))
                        .await?;
                }
            }
            Err(err) => {
                tracing::error!(
                    "Error when simulating sandbox limit order execution: {:?}",
                    err
                );
            }
        }
    }

    Ok(())
}
