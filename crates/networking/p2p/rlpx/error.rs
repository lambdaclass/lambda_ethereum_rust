use ethereum_rust_rlp::error::{RLPDecodeError, RLPEncodeError};
use ethereum_rust_storage::error::StoreError;
use k256::ecdsa::Error as EcdsaError;
use thiserror::Error;

// TODO improve errors
#[derive(Debug, Error)]
pub(crate) enum RLPxError {
    #[error("{0}")]
    HandshakeError(String),
    #[error("Invalid connection state")]
    InvalidState(),
    #[error("Not Found: {0}")]
    NotFound(String),
    #[error("Invalid peer id")]
    InvalidPeerId(),
    #[error("Invalid recovery id")]
    InvalidRecoveryId(),
    #[error("Cannot handle message")]
    MessageNotHandled(),
    #[error(transparent)]
    RLPDecodeError(#[from] RLPDecodeError),
    #[error(transparent)]
    RLPEncodeError(#[from] RLPEncodeError),
    #[error(transparent)]
    StoreError(#[from] StoreError),
    #[error(transparent)]
    EcdsaError(#[from] EcdsaError),
}
