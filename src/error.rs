use cfmms::error::CFFMError;
use ethers::{
    prelude::{AbiError, ContractError},
    providers::{JsonRpcClient, Provider, ProviderError},
    types::H256,
};
use thiserror::Error;
use tokio::task::JoinError;

#[derive(Error, Debug)]
pub enum ExecutorError<P>
where
    P: JsonRpcClient,
{
    #[error("Provider error")]
    ProviderError(#[from] ProviderError),
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
