use thiserror::Error;

// TODO improve errors
#[derive(Debug, Error)]
pub enum StoreError {
    #[error("DecodeError")]
    DecodeError,
    #[cfg(feature = "libmdbx")]
    #[error("Libmdbx error: {0}")]
    LibmdbxError(anyhow::Error),
    #[error("{0}")]
    Custom(String),
}
