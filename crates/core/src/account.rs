use std::collections::HashMap;

use ethereum_types::{H256, U256};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Account {
    #[serde(default)]
    code: Vec<u8>,
    #[serde(default)]
    storage: HashMap<H256, H256>,
    balance: U256,
    #[serde(default)]
    nonce: u64,
}
