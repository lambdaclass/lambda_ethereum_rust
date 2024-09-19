use ethereum_rust_core::{Address, H256, U256};

// TODO: serialize all to hex strings
pub struct AccountProof {
    account_proof: Vec<Vec<u8>>,
    address: Address,
    balance: U256,
    code_hash: H256,
    nonce: u64,
    storage_hash: H256,
    storage_proof: Vec<StorageProof>,
}

pub struct StorageProof {
    key: U256,
    proof: Vec<Vec<u8>>,
    value: U256,
}
