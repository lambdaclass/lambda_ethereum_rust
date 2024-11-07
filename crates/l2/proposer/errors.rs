use crate::utils::{config::errors::ConfigError, eth_client::errors::EthClientError};
use ethereum_rust_dev::utils::engine_client::errors::EngineClientError;
use ethereum_rust_vm::EvmError;

#[derive(Debug, thiserror::Error)]
pub enum L1WatcherError {
    #[error("L1Watcher error: {0}")]
    EthClientError(#[from] EthClientError),
    #[error("L1Watcher failed to deserialize log: {0}")]
    FailedToDeserializeLog(String),
    #[error("L1Watcher failed to parse private key: {0}")]
    FailedToDeserializePrivateKey(String),
    #[error("L1Watcher failed to retrieve depositor account info: {0}")]
    FailedToRetrieveDepositorAccountInfo(String),
    #[error("L1Watcher failed to retrieve chain config: {0}")]
    FailedToRetrieveChainConfig(String),
    #[error("L1Watcher failed to get config: {0}")]
    FailedToGetConfig(#[from] ConfigError),
}

#[derive(Debug, thiserror::Error)]
pub enum ProverServerError {
    #[error("ProverServer connection failed: {0}")]
    ConnectionError(#[from] std::io::Error),
    #[error("Proposer failed because of an EthClient error: {0}")]
    EthClientError(#[from] EthClientError),
}

#[derive(Debug, thiserror::Error)]
pub enum ProposerError {
    #[error("Proposer failed because of an EthClient error: {0}")]
    EthClientError(#[from] EthClientError),
    #[error("Proposer failed because of an EngineClient error: {0}")]
    EngineClientError(#[from] EngineClientError),
    #[error("Proposer failed to produce block: {0}")]
    FailedToProduceBlock(String),
    #[error("Proposer failed to prepare PayloadAttributes timestamp: {0}")]
    FailedToGetSystemTime(#[from] std::time::SystemTimeError),
    #[error("Proposer failed to serialize block: {0}")]
    FailedToRetrieveBlockFromStorage(String),
    #[error("Proposer failed to encode state diff: {0}")]
    FailedToEncodeStateDiff(#[from] StateDiffError),
    #[error("Proposer failed to open Points file: {0}")]
    FailedToOpenPointsFile(#[from] std::io::Error),
    #[error("Proposer failed to re-execute block: {0}")]
    FailedToReExecuteBlock(#[from] EvmError),
    #[error("Proposer failed to make KZG operations: {0}")]
    KZGError(#[from] c_kzg::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum StateDiffError {
    #[error("StateDiff failed to deserialize: {0}")]
    FailedToDeserializeStateDiff(String),
    #[error("StateDiff failed to serialize: {0}")]
    FailedToSerializeStateDiff(String),
    #[error("StateDiff failed to get config: {0}")]
    FailedToGetConfig(#[from] ConfigError),
    #[error("StateDiff invalid account state diff type: {0}")]
    InvalidAccountStateDiffType(u8),
    #[error("StateDiff unsupported version: {0}")]
    UnsupportedVersion(u8),
    #[error("Both bytecode and bytecode hash are set")]
    BytecodeAndBytecodeHashSet,
    #[error("Empty account diff")]
    EmptyAccountDiff,
}
