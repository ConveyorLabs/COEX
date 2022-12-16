use cfmms::error::CFFMError;
use ethers::{
    prelude::{gas_oracle::MiddlewareError, AbiError, ContractError},
    providers::{JsonRpcClient, Middleware, Provider, ProviderError},
    types::H256,
};
use thiserror::Error;
use tokio::task::JoinError;

#[derive(Error, Debug)]
pub enum ExecutorError<P, M>
where
    P: JsonRpcClient,
    M: Middleware,
{
    #[error("Provider error")]
    ProviderError(#[from] ProviderError),
    #[error("Middlewear error")]
    MiddlewareError(#[from] MiddlewareError<M>),
    #[error("Contract error")]
    ContractError(#[from] ContractError<Provider<P>>),
    #[error("ABI error")]
    ABIError(#[from] AbiError),
    #[error("Join error")]
    JoinError(#[from] JoinError),
    #[error("CFFM error")]
    CFFMError(#[from] CFFMError<P>),
    #[error("Invalid order group index")]
    InvalidOrderGroupIndex(),
    #[error("tokio::sync::mpsc error")]
    PendingTransactionSendError(#[from] tokio::sync::mpsc::error::SendError<(H256, Vec<H256>)>),
}
