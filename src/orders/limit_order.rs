use ethers::types::{H160, H256};

#[derive(Debug)]
pub struct LimitOrder {
    pub buy: bool,
    pub taxed: bool,
    pub stop_loss: bool,
    pub last_refresh_timestamp: u32,
    pub expiration_timestamp: u32,
    pub fee_in: u32,
    pub fee_out: u32,
    pub tax_in: u16,
    pub price: u128,
    pub amount_out_min: u128,
    pub quantity: u128,
    pub execution_credit: u128,
    pub owner: H160,
    pub token_in: H160,
    pub token_out: H160,
    pub order_id: H256,
}
