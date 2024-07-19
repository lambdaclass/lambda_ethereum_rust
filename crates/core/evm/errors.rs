use std::fmt::Display;

use revm::primitives::result::EVMError as RevmError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EvmError {
    #[error("Invalid Transaction: {0}")]
    Transaction(String),
    #[error("Invalid Header: {0}")]
    Header(String),
    #[error("DB error: {0}")]
    DB(String),
    #[error("{0}")]
    Custom(String),
    #[error("{0}")]
    Precompile(String),
}

impl<T: Display> From<RevmError<T>> for EvmError {
    fn from(value: RevmError<T>) -> Self {
        match value {
            RevmError::Transaction(err) => EvmError::Transaction(err.to_string()),
            RevmError::Header(err) => EvmError::Header(err.to_string()),
            RevmError::Database(err) => EvmError::DB(err.to_string()),
            RevmError::Custom(err) => EvmError::Custom(err),
            RevmError::Precompile(err) => EvmError::Precompile(err),
        }
    }
}
