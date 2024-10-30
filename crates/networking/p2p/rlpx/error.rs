use crate::rlpx::message::Message;
use ethereum_rust_rlp::error::{RLPDecodeError, RLPEncodeError};
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
    #[error("Invalid peer id")]
    InvalidPeerId(),
    #[error("Unexpected message: {0}")]
    UnexpectedMessage(Message),
}
