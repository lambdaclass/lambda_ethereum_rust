use crate::db_memorydb::MemoryDB;
use ethereum_rust_core::types::{Block, BlockHeader};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Input {
    pub db: MemoryDB,
    pub parent_block_header: BlockHeader,
    pub block: Block,
}
