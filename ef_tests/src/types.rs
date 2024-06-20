use revm::primitives::{
    alloy_primitives::{Bloom, B64},
    Address, Bytes, B256, U256,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TestUnit {
    #[serde(default, rename = "_info")]
    pub info: Option<serde_json::Value>,
    pub blocks: Vec<Block>,
    pub genesis_block_header: Header,
    #[serde(rename = "genesisRLP")]
    pub genesis_rlp: serde_json::Value,
    pub lastblockhash: serde_json::Value,
    pub network: serde_json::Value,
    pub post_state: serde_json::Value,
    pub pre: HashMap<Address, Account>,
    pub seal_engine: serde_json::Value,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub struct Account {
    pub balance: U256,
    pub code: Bytes,
    pub nonce: U256,
    pub storage: HashMap<U256, U256>,
}

#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TransactionParts {
    pub data: Vec<Bytes>,
    pub gas_limit: Vec<U256>,
    pub gas_price: Option<U256>,
    pub nonce: U256,
    pub secret_key: B256,
    pub sender: Address,
    pub to: Address,
    pub value: Vec<U256>,
    pub max_fee_per_gas: Option<U256>,
    pub max_priority_fee_per_gas: Option<U256>,
    #[serde(default)]
    pub access_lists: Vec<Option<AccessList>>,
    #[serde(default)]
    pub blob_versioned_hashes: Vec<B256>,
    pub max_fee_per_blob_gas: Option<U256>,
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
    pub previous_hash: Option<B256>,
    pub current_random: Option<B256>,
    pub current_beacon_root: Option<B256>,
    pub current_withdrawals_root: Option<B256>,
    pub parent_blob_gas_used: Option<U256>,
    pub parent_excess_blob_gas: Option<U256>,
    pub current_excess_blob_gas: Option<U256>,
}

#[derive(Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TxPartIndices {
    pub data: usize,
    pub gas: usize,
    pub value: usize,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AccessListItem {
    pub address: Address,
    pub storage_keys: Vec<B256>,
}

pub type AccessList = Vec<AccessListItem>;

#[derive(Debug, PartialEq, Eq, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Header {
    pub bloom: Bloom,
    pub coinbase: Address,
    pub difficulty: U256,
    pub extra_data: Bytes,
    pub gas_limit: U256,
    pub gas_used: U256,
    pub hash: B256,
    pub mix_hash: B256,
    pub nonce: B64,
    pub number: U256,
    pub parent_hash: B256,
    pub receipt_trie: B256,
    pub state_root: B256,
    pub timestamp: U256,
    pub transactions_trie: B256,
    pub uncle_hash: B256,
    pub base_fee_per_gas: Option<U256>,
    pub withdrawals_root: Option<B256>,
    pub blob_gas_used: Option<U256>,
    pub excess_blob_gas: Option<U256>,
    pub parent_beacon_block_root: Option<B256>,
    pub requests_root: Option<B256>,
}

#[derive(Debug, PartialEq, Eq, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Block {
    pub block_header: Option<Header>,
    pub rlp: Bytes,
    pub transactions: Option<Vec<Transaction>>,
    pub uncle_headers: Option<Vec<Header>>,
}

#[derive(Debug, PartialEq, Eq, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Transaction {
    #[serde(rename = "type")]
    pub transaction_type: Option<U256>,
    pub data: Bytes,
    pub gas_limit: U256,
    pub gas_price: Option<U256>,
    pub nonce: U256,
    pub r: U256,
    pub s: U256,
    pub v: U256,
    pub value: U256,
    pub chain_id: Option<U256>,
    pub access_list: Option<AccessList>,
    pub max_fee_per_gas: Option<U256>,
    pub max_priority_fee_per_gas: Option<U256>,
    pub hash: Option<B256>,
    pub sender: Address,
    pub to: Address,
}
