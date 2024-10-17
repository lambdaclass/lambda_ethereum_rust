use thiserror::Error;

#[derive(Debug, Error)]
pub enum MemoryDBError {
    #[error("{0}")]
    Custom(String),
}
