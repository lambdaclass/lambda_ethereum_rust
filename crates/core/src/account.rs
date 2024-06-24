use std::collections::HashMap;

use ethereum_types::{H256, U256};
use serde::Deserialize;

#[allow(unused)]
#[derive(Debug, Deserialize)]
pub struct Account {
    #[serde(default)]
    pub code: Vec<u8>,
    #[serde(default)]
    pub storage: HashMap<H256, H256>,
    pub balance: U256,
    #[serde(default)]
    pub nonce: u64,
}
