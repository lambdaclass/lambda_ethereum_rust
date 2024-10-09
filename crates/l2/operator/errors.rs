use crate::utils::{config::errors::ConfigError, eth_client::errors::EthClientError};

#[derive(Debug, thiserror::Error)]
pub enum L1WatcherError {
    #[error("L1Watcher error: {0}")]
    EthClientError(#[from] EthClientError),
    #[error("L1Watcher failed to deserialize log: {0}")]
    LogTopicDeserializationError(String),
    #[error("L1Watcher failed to parse private key: {0}")]
    SignerPrivateKeyDeserializationError(String),
    #[error("L1Watcher failed to retrieve depositor account info: {0}")]
    DepositorAccountInfoRetrievalError(String),
    #[error("L1Watcher failed to retrieve chain config: {0}")]
    ChainConfigRetrievalError(String),
    #[error("L1Watcher failed to get config: {0}")]
    ConfigError(#[from] ConfigError),
}

#[derive(Debug, thiserror::Error)]
pub enum ProofDataProviderError {}

#[derive(Debug, thiserror::Error)]
pub enum OperatorError {}
