use ethereum_rust_storage::error::StoreError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MempoolError {
    #[error("DB error: {0}")]
    StoreError(#[from] StoreError),
}
