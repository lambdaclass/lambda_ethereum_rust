use serde::{Deserialize, Serialize};

use ethereum_rust_core::{types::BlockHash, H256};

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PayloadStatus {
    pub status: PayloadValidationStatus,
    pub latest_valid_hash: Option<H256>,
    pub validation_error: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum PayloadValidationStatus {
    Valid,
    Invalid,
    Syncing,
    Accepted,
}

impl PayloadStatus {
    // Convenience methods to create payload status

    /// Creates a PayloadStatus with invalid status and error message
    pub fn invalid_with_err(error: &str) -> Self {
        PayloadStatus {
            status: PayloadValidationStatus::Invalid,
            latest_valid_hash: None,
            validation_error: Some(error.to_string()),
        }
    }

    /// Creates a PayloadStatus with invalid status and latest valid hash
    pub fn invalid_with_hash(hash: BlockHash) -> Self {
        PayloadStatus {
            status: PayloadValidationStatus::Invalid,
            latest_valid_hash: Some(hash),
            validation_error: None,
        }
    }

    /// Creates a PayloadStatus with syncing status and no other info
    pub fn syncing() -> Self {
        PayloadStatus {
            status: PayloadValidationStatus::Syncing,
            latest_valid_hash: None,
            validation_error: None,
        }
    }

    /// Creates a PayloadStatus with valid status and latest valid hash
    pub fn valid_with_hash(hash: BlockHash) -> Self {
        PayloadStatus {
            status: PayloadValidationStatus::Valid,
            latest_valid_hash: Some(hash),
            validation_error: None,
        }
    }
    /// Creates a PayloadStatus with valid status and latest valid hash
    pub fn valid() -> Self {
        PayloadStatus {
            status: PayloadValidationStatus::Valid,
            latest_valid_hash: None,
            validation_error: None,
        }
    }
}
