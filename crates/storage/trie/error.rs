use ethrex_rlp::error::RLPDecodeError;
use redb::{CommitError, StorageError, TableError, TransactionError};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TrieError {
    #[error("Libmdbx error: {0}")]
    LibmdbxError(anyhow::Error),
    #[error("Redb Storage error: {0}")]
    RedbStorageError(#[from] StorageError),
    #[error("Redb Table error: {0}")]
    RedbTableError(#[from] TableError),
    #[error("Redb Commit error: {0}")]
    RedbCommitError(#[from] CommitError),
    #[error("Redb Transaction error: {0}")]
    RedbTransactionError(#[from] TransactionError),
    #[error(transparent)]
    RLPDecode(#[from] RLPDecodeError),
    #[error("Verification Error: {0}")]
    Verify(String),
}
