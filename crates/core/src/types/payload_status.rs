use keccak_hash::H256;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PayloadStatus {
    status: PayloadValidationStatus,
    latest_valid_hash: H256,
    validation_error: Option<String>
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum PayloadValidationStatus {
    Valid,
    Invalid,
    Syncing,
    Accepted,
}
