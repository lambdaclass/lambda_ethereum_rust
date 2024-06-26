use std::collections::HashMap;

use bytes::Bytes;
use ethereum_types::{H256, U256};
use serde::Deserialize;

use crate::rlp::encode::RLPEncode;

#[allow(unused)]
#[derive(Debug, Deserialize, PartialEq)]
pub struct Account {
    #[serde(flatten)]
    pub info: AccountInfo,
    #[serde(default)]
    pub storage: AccountStorage,
}

// We use two separate structs for easier DB management
#[derive(Debug, Deserialize, PartialEq)]
pub struct AccountInfo {
    #[serde(default)]
    pub code: Bytes,
    #[serde(deserialize_with = "crate::serde_utils::u256::deser_dec_str")]
    pub balance: U256,
    #[serde(default, deserialize_with = "crate::serde_utils::u64::deser_dec_str")]
    pub nonce: u64,
}

#[derive(Debug, Deserialize, PartialEq, Default)]
pub struct AccountStorage(pub HashMap<H256, H256>);

impl RLPEncode for AccountInfo {
    fn encode(&self, buf: &mut dyn bytes::BufMut) {
        self.code.encode(buf);
        self.balance.encode(buf);
        self.nonce.encode(buf);
    }
}
