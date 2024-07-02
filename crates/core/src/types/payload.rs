use bytes::Bytes;
use ethereum_types::{Address, Bloom};
use keccak_hash::H256;
use serde::{Deserialize, Serialize};

use crate::serde_utils;

use super::Withdrawal;

#[allow(unused)]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionPayloadV3 {
    parent_hash: H256,
    fee_recipient: Address,
    state_root: H256,
    receipts_root: H256,
    logs_bloom: Bloom,
    prev_randao: H256,
    #[serde(deserialize_with = "crate::serde_utils::u64::deser_hex_str")]
    block_number: u64,
    #[serde(deserialize_with = "crate::serde_utils::u64::deser_hex_str")]
    gas_limit: u64,
    #[serde(deserialize_with = "crate::serde_utils::u64::deser_hex_str")]
    gas_used: u64,
    #[serde(deserialize_with = "crate::serde_utils::u64::deser_hex_str")]
    timestamp: u64,
    #[serde(deserialize_with = "crate::serde_utils::bytes::deser_hex_str")]
    extra_data: Bytes,
    #[serde(deserialize_with = "crate::serde_utils::u64::deser_hex_str")]
    base_fee_per_gas: u64,
    block_hash: H256,
    transactions: Vec<EncodedTransaction>,
    withdrawals: Vec<Withdrawal>,
    #[serde(deserialize_with = "crate::serde_utils::u64::deser_hex_str")]
    blob_gas_used: u64,
    #[serde(deserialize_with = "crate::serde_utils::u64::deser_hex_str")]
    excess_blob_gas: u64,
}

#[allow(unused)]
#[derive(Debug)]
pub struct EncodedTransaction(pub Bytes);

impl<'de> Deserialize<'de> for EncodedTransaction {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(EncodedTransaction(serde_utils::bytes::deser_hex_str(
            deserializer,
        )?))
    }
}

#[allow(unused)]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PayloadStatus {
    status: PayloadValidationStatus,
    latest_valid_hash: H256,
    validation_error: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum PayloadValidationStatus {
    Valid,
    Invalid,
    Syncing,
    Accepted,
}
