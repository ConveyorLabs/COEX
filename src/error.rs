use cfmms::error::CFMMError;
use ethers::{
    prelude::{
        nonce_manager::NonceManagerError, AbiError, ContractError,
    },
    providers::{Middleware, ProviderError},
    types::H256,
};
use thiserror::Error;
use tokio::task::JoinError;

#[derive(Error, Debug)]
pub enum ExecutorError<M>
where
    M: Middleware,
{
    #[error("Provider error")]
    ProviderError(#[from] ProviderError),
    #[error("Middleware error")]
    MiddlewareError(<M as Middleware>::Error),
    #[error("Nonce manager error")]
    NonceManagerError(#[from] NonceManagerError<M>),
    #[error("Contract error")]
    ContractError(#[from] ContractError<M>),
    #[error("ABI error")]
    ABIError(#[from] AbiError),
    #[error("Join error")]
    JoinError(#[from] JoinError),
    #[error("CFFM error")]
    CFFMError(#[from] CFMMError<M>),
    #[error("Invalid order group index")]
    InvalidOrderGroupIndex(),
    #[error("tokio::sync::mpsc error")]
    PendingTransactionSendError(#[from] tokio::sync::mpsc::error::SendError<(H256, Vec<H256>)>),
}
