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
    #[error("Decode Error: {0}")]
    DecodeError(#[from] RLPDecodeError),
    #[error("Encode Error: {0}")]
    EncodeError(#[from] RLPEncodeError),
    #[error("Store Error: {0}")]
    StoreError(#[from] StoreError),
    #[error("Not Found: {0}")]
    NotFound(String),
    #[error("Invalid peer id")]
    InvalidPeerId(),
    #[error("Cryptography Error: {0}")]
    CryptographyError(#[from] EcdsaError),
    #[error("Invalid recovery id")]
    InvalidRecoveryId(),
    #[error("Cannot handle message")]
    MessageNotHandled(),
}
