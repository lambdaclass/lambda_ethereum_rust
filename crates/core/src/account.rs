use std::collections::HashMap;

use bytes::Bytes;
use ethereum_types::{H256, U256};
use serde::Deserialize;

#[allow(unused)]
#[derive(Debug, Deserialize)]
pub struct Account {
    #[serde(default)]
    pub code: Bytes,
    #[serde(default)]
    pub storage: HashMap<H256, H256>,
    #[serde(deserialize_with = "crate::serde_utils::u256::deser_dec_str")]
    pub balance: U256,
    #[serde(default, deserialize_with = "crate::serde_utils::u64::deser_dec_str")]
    pub nonce: u64,
}
