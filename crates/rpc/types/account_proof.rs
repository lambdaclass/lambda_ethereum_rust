use ethereum_rust_core::{Address, H256, U256};

// TODO: serialize all to hex strings
pub struct AccountProof {
    pub account_proof: Vec<Vec<u8>>,
    pub address: Address,
    pub balance: U256,
    pub code_hash: H256,
    pub nonce: u64,
    pub storage_hash: H256,
    pub storage_proof: Vec<StorageProof>,
}

pub struct StorageProof {
    pub key: U256,
    pub proof: Vec<Vec<u8>>,
    pub value: U256,
}
