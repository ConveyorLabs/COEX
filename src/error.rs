use cfmms::error::PairSyncError;
use ethers::{
    prelude::{AbiError, ContractError},
    providers::{JsonRpcClient, Provider, ProviderError},
};
use thiserror::Error;
use tokio::task::JoinError;

#[derive(Error, Debug)]
pub enum BeltError<P>
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
    #[error("Pair sync error")]
    PairSyncError(#[from] PairSyncError<P>),
}
