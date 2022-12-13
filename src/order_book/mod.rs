pub struct SandboxLimitOrderBook(H160);
pub struct LimitOrderBook(H160);

pub trait OrderBook {
    //Get order
    //update order
    //ect
}

impl OrderBook for SandboxLimitOrderBook {}

impl OrderBook for LimitOrderBook {}
