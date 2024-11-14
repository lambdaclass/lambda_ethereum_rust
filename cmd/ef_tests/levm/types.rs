use crate::{
    deserialize::{
        deserialize_ef_post_value_indexes, deserialize_hex_bytes, deserialize_hex_bytes_vec,
        deserialize_u256_optional_safe, deserialize_u256_safe,
        deserialize_u256_valued_hashmap_safe, deserialize_u256_vec_safe,
    },
    utils,
};
use bytes::Bytes;
use ethereum_rust_core::{
    types::{Genesis, GenesisAccount, TxKind},
    Address, H256, U256,
};
use ethereum_rust_levm::{db::Cache, errors::VMError, vm::VM, Environment};
use ethereum_rust_vm::db::StoreWrapper;
use serde::Deserialize;
use std::{collections::HashMap, sync::Arc};

#[derive(Debug)]
pub struct EFTest {
    pub name: String,
    pub _info: EFTestInfo,
    pub env: EFTestEnv,
    pub post: EFTestPost,
    pub pre: EFTestPre,
    pub transaction: EFTestTransaction,
}

impl TryFrom<&EFTest> for VM {
    type Error = VMError;

    fn try_from(test: &EFTest) -> Result<Self, Self::Error> {
        let (initial_state, parent_hash) = utils::load_initial_state(test);
        let db = Arc::new(StoreWrapper {
            store: initial_state.database().unwrap().clone(),
            block_hash: parent_hash,
        });
        Self::new(
            test.transaction.to.clone(),
            Environment {
                origin: test.transaction.sender,
                consumed_gas: U256::default(),
                refunded_gas: U256::default(),
                gas_limit: test.env.current_gas_limit,
                block_number: test.env.current_number,
                coinbase: test.env.current_coinbase,
                timestamp: test.env.current_timestamp,
                prev_randao: Some(test.env.current_random),
                chain_id: U256::from(1729),
                base_fee_per_gas: test.env.current_base_fee,
                gas_price: test.transaction.gas_price.unwrap_or_default(), // or max_fee_per_gas?
                block_excess_blob_gas: Some(test.env.current_excess_blob_gas),
                block_blob_gas_used: None,
                tx_blob_hashes: None,
            },
            *test.transaction.value.first().unwrap(),
            test.transaction.data.first().unwrap().clone(),
            db,
            Cache::default(),
        )
    }
}

impl From<&EFTest> for Genesis {
    fn from(test: &EFTest) -> Self {
        Genesis {
            alloc: {
                let mut alloc = HashMap::new();
                for (account, ef_test_pre_value) in test.pre.0.iter() {
                    alloc.insert(*account, ef_test_pre_value.into());
                }
                alloc
            },
            coinbase: test.env.current_coinbase,
            difficulty: test.env.current_difficulty,
            gas_limit: test.env.current_gas_limit.as_u64(),
            mix_hash: test.env.current_random,
            timestamp: test.env.current_timestamp.as_u64(),
            base_fee_per_gas: Some(test.env.current_base_fee.as_u64()),
            excess_blob_gas: Some(test.env.current_excess_blob_gas.as_u64()),
            ..Default::default()
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct EFTestInfo {
    pub comment: String,
    #[serde(rename = "filling-rpc-server")]
    pub filling_rpc_server: String,
    #[serde(rename = "filling-tool-version")]
    pub filling_tool_version: String,
    #[serde(rename = "generatedTestHash")]
    pub generated_test_hash: H256,
    #[serde(default)]
    pub labels: Option<HashMap<u64, String>>,
    pub lllcversion: String,
    pub solidity: String,
    pub source: String,
    #[serde(rename = "sourceHash")]
    pub source_hash: H256,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EFTestEnv {
    #[serde(deserialize_with = "deserialize_u256_safe")]
    pub current_base_fee: U256,
    pub current_coinbase: Address,
    #[serde(deserialize_with = "deserialize_u256_safe")]
    pub current_difficulty: U256,
    #[serde(deserialize_with = "deserialize_u256_safe")]
    pub current_excess_blob_gas: U256,
    #[serde(deserialize_with = "deserialize_u256_safe")]
    pub current_gas_limit: U256,
    #[serde(deserialize_with = "deserialize_u256_safe")]
    pub current_number: U256,
    pub current_random: H256,
    #[serde(deserialize_with = "deserialize_u256_safe")]
    pub current_timestamp: U256,
}

#[derive(Debug, Deserialize, Clone)]
pub enum EFTestPost {
    Cancun(Vec<EFTestPostValue>),
}

impl EFTestPost {
    pub fn values(self) -> Vec<EFTestPostValue> {
        match self {
            EFTestPost::Cancun(v) => v,
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &EFTestPostValue> {
        match self {
            EFTestPost::Cancun(v) => v.iter(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct EFTestPostValue {
    #[serde(rename = "expectException")]
    pub expect_exception: Option<String>,
    pub hash: H256,
    #[serde(deserialize_with = "deserialize_ef_post_value_indexes")]
    pub indexes: HashMap<String, U256>,
    pub logs: H256,
    #[serde(deserialize_with = "deserialize_hex_bytes")]
    pub txbytes: Bytes,
}

#[derive(Debug, Deserialize)]
pub struct EFTestPre(pub HashMap<Address, EFTestPreValue>);

#[derive(Debug, Deserialize)]
pub struct EFTestPreValue {
    #[serde(deserialize_with = "deserialize_u256_safe")]
    pub balance: U256,
    #[serde(deserialize_with = "deserialize_hex_bytes")]
    pub code: Bytes,
    #[serde(deserialize_with = "deserialize_u256_safe")]
    pub nonce: U256,
    #[serde(deserialize_with = "deserialize_u256_valued_hashmap_safe")]
    pub storage: HashMap<U256, U256>,
}

impl From<&EFTestPreValue> for GenesisAccount {
    fn from(value: &EFTestPreValue) -> Self {
        Self {
            code: value.code.clone(),
            storage: value
                .storage
                .iter()
                .map(|(k, v)| {
                    let mut key_bytes = [0u8; 32];
                    k.to_big_endian(&mut key_bytes);
                    (H256::from_slice(&key_bytes), *v)
                })
                .collect(),
            balance: value.balance,
            nonce: value.nonce.as_u64(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EFTestTransaction {
    #[serde(deserialize_with = "deserialize_hex_bytes_vec")]
    pub data: Vec<Bytes>,
    #[serde(deserialize_with = "deserialize_u256_vec_safe")]
    pub gas_limit: Vec<U256>,
    #[serde(default, deserialize_with = "deserialize_u256_optional_safe")]
    pub gas_price: Option<U256>,
    #[serde(deserialize_with = "deserialize_u256_safe")]
    pub nonce: U256,
    pub secret_key: H256,
    pub sender: Address,
    pub to: TxKind,
    #[serde(deserialize_with = "deserialize_u256_vec_safe")]
    pub value: Vec<U256>,
}
