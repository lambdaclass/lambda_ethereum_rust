use std::collections::HashMap;

use ethereum_types::{H256, U256};

#[derive(Clone, Debug, Default)]
pub struct Db {
    block_hashes: HashMap<U256, H256>,
}

impl Db {
    pub fn insert_block_hash(&mut self, number: U256, hash: H256) {
        self.block_hashes.insert(number, hash);
    }

    pub fn get_block_hash(&mut self, number: U256) -> Option<H256> {
        self.block_hashes.get(&number).cloned()
    }
}
