use super::primitives::{Address, Bytes, H256, U256};
use serde::{de, Deserialize};
use std::collections::{BTreeMap, HashMap};

/*
Models and deserialization functions extracted from REVM revme
https://github.com/bluealloy/revm/blob/main/bins/revme
*/

pub fn deserialize_str_as_u64<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: de::Deserializer<'de>,
{
    let string = String::deserialize(deserializer)?;

    if let Some(stripped) = string.strip_prefix("0x") {
        u64::from_str_radix(stripped, 16)
    } else {
        string.parse()
    }
    .map_err(serde::de::Error::custom)
}

pub fn deserialize_maybe_empty<'de, D>(deserializer: D) -> Result<Option<Address>, D::Error>
where
    D: de::Deserializer<'de>,
{
    let string = String::deserialize(deserializer)?;
    if string.is_empty() {
        Ok(None)
    } else {
        string.parse().map_err(de::Error::custom).map(Some)
    }
}

pub type SpecName = String; //TODO: maybe we should check if the SpecName is valid

#[derive(Debug, PartialEq, Eq, Deserialize)]
pub struct TestSuite(pub BTreeMap<String, TestUnit>);

#[derive(Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TestUnit {
    /// Test info is optional
    #[serde(default, rename = "_info")]
    pub info: Option<serde_json::Value>,
    pub env: Env,
    pub pre: HashMap<Address, AccountInfo>,
    pub post: BTreeMap<SpecName, Vec<Test>>,
    pub transaction: TransactionParts,
    #[serde(default)]
    pub out: Option<Bytes>,
}

/// State test indexed state result deserialization.
#[derive(Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Test {
    pub expect_exception: Option<String>,
    pub indexes: TxPartIndices,
    pub hash: H256,
    #[serde(default)]
    pub post_state: HashMap<Address, AccountInfo>,
    pub logs: H256,
    pub txbytes: Option<Bytes>,
}

#[derive(Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TxPartIndices {
    pub data: usize,
    pub gas: usize,
    pub value: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AccountInfo {
    pub balance: U256,
    pub code: Bytes,
    #[serde(deserialize_with = "deserialize_str_as_u64")]
    pub nonce: u64,
    pub storage: HashMap<U256, U256>,
}

#[derive(Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Env {
    pub current_coinbase: Address,
    pub current_difficulty: U256,
    pub current_gas_limit: U256,
    pub current_number: U256,
    pub current_timestamp: U256,
    pub current_base_fee: Option<U256>,
    pub previous_hash: Option<H256>,
    pub current_random: Option<H256>,
    pub current_beacon_root: Option<H256>,
    pub current_withdrawals_root: Option<H256>,
    pub parent_blob_gas_used: Option<U256>,
    pub parent_excess_blob_gas: Option<U256>,
    pub current_excess_blob_gas: Option<U256>,
}

#[derive(Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TransactionParts {
    pub data: Vec<Bytes>,
    pub gas_limit: Vec<U256>,
    pub gas_price: Option<U256>,
    pub nonce: U256,
    pub secret_key: H256,
    /// if sender is not present we need to derive it from secret key.
    #[serde(default)]
    pub sender: Option<Address>,
    #[serde(deserialize_with = "deserialize_maybe_empty")]
    pub to: Option<Address>,
    pub value: Vec<U256>,
    pub max_fee_per_gas: Option<U256>,
    pub max_priority_fee_per_gas: Option<U256>,
    #[serde(default)]
    pub access_lists: Vec<Option<AccessList>>,
    #[serde(default)]
    pub blob_versioned_hashes: Vec<H256>,
    pub max_fee_per_blob_gas: Option<U256>,
}

#[derive(Debug, PartialEq, Eq, Deserialize, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AccessListItem {
    pub address: Address,
    pub storage_keys: Vec<H256>,
}

pub type AccessList = Vec<AccessListItem>;
