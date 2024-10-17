use ethereum_rust_core::{
    types::{Block, BlockHeader},
    Bytes,
};
use ethereum_rust_rlp::decode::RLPDecode;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct ProverInput {
    pub db: MemoryDB,
    pub parent_block_header: BlockHeader,
    pub block: Block,
}

pub struct MemoryDB {
    data: u64,
}
