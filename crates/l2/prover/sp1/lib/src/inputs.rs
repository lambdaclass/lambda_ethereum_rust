use crate::db_memorydb::MemoryDB;
use ethereum_rust_core::{
    types::{Block, BlockHeader},
    Bytes,
};
use ethereum_rust_rlp::decode::RLPDecode;
use serde::{Deserialize, Serialize};

use std::io::{BufReader, Read};

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct ProverInput {
    pub db: MemoryDB,
    pub parent_block_header: BlockHeader,
    pub block: Block,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct ProverInputNoExecution {
    pub head_block: Block,
    pub parent_block_header: BlockHeader,
    pub block_is_valid: bool,
}

#[derive(Debug, Deserialize)]
pub struct RawProverInputNoExecution {
    pub head_block: Bytes,
    pub parent_block_header: Bytes,
    pub block_is_valid: bool,
}

pub fn read_chain_file(chain_rlp_path: &str) -> Vec<Block> {
    let chain_file = std::fs::File::open(chain_rlp_path).expect("Failed to open chain rlp file");
    let mut chain_rlp_reader = BufReader::new(chain_file);
    let mut buf = vec![];
    chain_rlp_reader.read_to_end(&mut buf).unwrap();
    let mut blocks = Vec::new();
    while !buf.is_empty() {
        let (item, rest) = Block::decode_unfinished(&buf).unwrap();
        blocks.push(item);
        buf = rest.to_vec();
    }
    blocks
}
