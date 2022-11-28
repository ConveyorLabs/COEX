use ethers::types::{H160, H256};

#[derive(Debug)]
pub struct SandboxLimitOrder {
    pub last_refresh_timestamp: u32,
    pub expiration_timestamp: u32,
    pub fill_percent: u128,
    pub fee_remaining: u128,
    pub amount_in_remaining: u128,
    pub amount_out_remaining: u128,
    pub execution_credit_remaining: u128,
    pub owner: H160,
    pub token_in: H160,
    pub token_out: H160,
    pub order_id: H256,
}
