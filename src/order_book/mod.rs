use std::sync::Arc;

use async_trait::async_trait;
use ethers::{
    providers::{JsonRpcClient, Provider},
    types::{H160, H256},
};

use crate::{
    abi,
    error::ExecutorError,
    orders::order::{Order, OrderVariant},
};

pub struct SandboxLimitOrderBook(H160);
pub struct LimitOrderBook(H160);

#[async_trait]
pub trait OrderBook {
    async fn get_remote_order<P: JsonRpcClient>(
        &self,
        order_id: H256,
        provider: Arc<Provider<P>>,
    ) -> Result<Order, ExecutorError<P>>;

    //Get order
    //update order
    //ect
}

#[async_trait]
impl OrderBook for SandboxLimitOrderBook {
    async fn get_remote_order<P: JsonRpcClient>(
        &self,
        order_id: H256,
        provider: Arc<Provider<P>>,
    ) -> Result<Order, ExecutorError<P>> {
        let slob = abi::ISandboxLimitOrderBook::new(self.0, provider);
        let order_bytes = slob.get_order_by_id(order_id.into()).call().await?;

        Order::from_bytes(&order_bytes, OrderVariant::SandboxLimitOrder)
    }
}

#[async_trait]
impl OrderBook for LimitOrderBook {
    async fn get_remote_order<P: JsonRpcClient>(
        &self,
        order_id: H256,
        provider: Arc<Provider<P>>,
    ) -> Result<Order, ExecutorError<P>> {
        let lob = abi::ISandboxLimitOrderBook::new(self.0, provider);
        let order_bytes = lob.get_order_by_id(order_id.into()).call().await?;

        Order::from_bytes(&order_bytes, OrderVariant::LimitOrder)
    }
}
