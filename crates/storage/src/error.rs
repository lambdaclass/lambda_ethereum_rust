#[cfg(feature = "rocksdb")]
use std::sync::mpsc::{RecvError, SendError};
use thiserror::Error;

// TODO improve errors
#[derive(Debug, Error)]
pub enum StoreError {
    #[error("DecodeError")]
    DecodeError,
    #[cfg(feature = "libmdbx")]
    #[error("Libmdbx error: {0}")]
    LibmdbxError(#[from] anyhow::Error),
    #[cfg(feature = "rocksdb")]
    #[error("Rocksdb error: {0}")]
    RocksDbError(#[from] rocksdb::Error),
    #[cfg(feature = "rocksdb")]
    #[error("Recv error: {0}")]
    RecvError(#[from] RecvError),
    #[cfg(feature = "rocksdb")]
    #[error("Send error: {0}")]
    SendError(String),
    #[cfg(feature = "sled")]
    #[error("Sled error: {0}")]
    SledError(#[from] sled::Error),
    #[error("{0}")]
    Custom(String),
}

#[cfg(feature = "rocksdb")]
impl<T> From<SendError<T>> for StoreError {
    fn from(err: SendError<T>) -> Self {
        Self::SendError(err.to_string())
    }
}
