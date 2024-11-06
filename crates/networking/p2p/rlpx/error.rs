use ethereum_rust_rlp::error::{RLPDecodeError, RLPEncodeError};
use ethereum_rust_storage::error::StoreError;
use thiserror::Error;

// TODO improve errors
#[derive(Debug, Error)]
pub(crate) enum RLPxError {
    #[error("{0}")]
    HandshakeError(String),
    #[error("{0}")]
    ConnectionError(String),
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
    #[error("Error in cryptographic library: {0}")]
    CryptographyError(String),
}

// Grouping all cryptographic related errors in a single CryptographicError variant
// We can improve this to individual errors if required
impl From<k256::ecdsa::Error> for RLPxError {
    fn from(e: k256::ecdsa::Error) -> Self {
        RLPxError::CryptographyError(e.to_string())
    }
}

impl From<k256::elliptic_curve::Error> for RLPxError {
    fn from(e: k256::elliptic_curve::Error) -> Self {
        RLPxError::CryptographyError(e.to_string())
    }
}

impl From<sha3::digest::InvalidLength> for RLPxError {
    fn from(e: sha3::digest::InvalidLength) -> Self {
        RLPxError::CryptographyError(e.to_string())
    }
}

impl From<aes::cipher::StreamCipherError> for RLPxError {
    fn from(e: aes::cipher::StreamCipherError) -> Self {
        RLPxError::CryptographyError(e.to_string())
    }
}
