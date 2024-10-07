use thiserror::Error;

use ethereum_rust_core::types::InvalidBlockHeaderError;
use ethereum_rust_evm::EvmError;
use ethereum_rust_storage::error::StoreError;

#[derive(Debug, Error)]
pub enum ChainError {
    #[error("Invalid Block: {0}")]
    InvalidBlock(#[from] InvalidBlockError),
    #[error("Parent block not found")]
    ParentNotFound,
    //TODO: If a block with block_number greater than latest plus one is received
    //maybe we are missing data and should wait for syncing
    #[error("Block number is not child of a canonical block.")]
    NonCanonicalParent,
    #[error("DB error: {0}")]
    StoreError(#[from] StoreError),
    #[error("EVM error: {0}")]
    EvmError(#[from] EvmError),
}

#[derive(Debug, Error)]
pub enum InvalidBlockError {
    #[error("World State Root does not match the one in the header after executing")]
    StateRootMismatch,
    #[error("Invalid Header, validation failed pre-execution: {0}")]
    InvalidHeader(#[from] InvalidBlockHeaderError),
    #[error("Exceeded MAX_BLOB_GAS_PER_BLOCK")]
    ExceededMaxBlobGasPerBlock,
    #[error("Exceeded MAX_BLOB_NUMBER_PER_BLOCK")]
    ExceededMaxBlobNumberPerBlock,
    #[error("Gas used doesn't match value in header")]
    GasUsedMismatch,
    #[error("Blob gas used doesn't match value in header")]
    BlobGasUsedMismatch,
}

#[derive(Debug, Error)]
pub enum MempoolError {
    #[error("No block header")]
    NoBlockHeaderError,
    #[error("DB error: {0}")]
    StoreError(#[from] StoreError),
    #[error("Transaction max init code size exceeded")]
    TxMaxInitCodeSizeError,
    #[error("Transaction gas limit exceeded")]
    TxGasLimitExceededError,
    #[error("Transaction priority fee above gas fee")]
    TxGasOverflowError,
    #[error("Transaction intrinsic gas overflow")]
    TxTipAboveFeeCapError,
    #[error("Transaction intrinsic gas cost above gas limit")]
    TxIntrinsicGasCostAboveLimitError,
    #[error("Transaction blob base fee too low")]
    TxBlobBaseFeeTooLowError,
}
