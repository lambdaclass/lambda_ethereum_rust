use ethrex_rlp::error::RLPDecodeError;
use ethrex_trie::TrieError;
use redb::{CommitError, DatabaseError, StorageError, TableError, TransactionError};
use thiserror::Error;

// TODO improve errors
#[derive(Debug, Error)]
pub enum StoreError {
    #[error("DecodeError")]
    DecodeError,
    #[cfg(feature = "libmdbx")]
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
    #[error("Redb Database error: {0}")]
    RedbDatabaseError(#[from] DatabaseError),
    #[error("{0}")]
    Custom(String),
    #[error(transparent)]
    RLPDecode(#[from] RLPDecodeError),
    #[error(transparent)]
    Trie(#[from] TrieError),
    #[error("missing store: is an execution DB being used instead?")]
    MissingStore,
}
