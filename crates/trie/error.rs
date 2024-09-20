use ethereum_rust_rlp::error::RLPDecodeError;
use thiserror::Error;

// TODO improve errors
#[derive(Debug, Error)]
pub enum TrieError {
    #[error("DecodeError")]
    DecodeError,
    #[error("Libmdbx error: {0}")]
    LibmdbxError(anyhow::Error),
    #[error("{0}")]
    Custom(String),
    #[error(transparent)]
    RLPDecode(#[from] RLPDecodeError),
}
