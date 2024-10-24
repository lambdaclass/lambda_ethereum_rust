use ethereum_rust_core::types::{Block, Genesis};
use ethereum_rust_rlp::decode::RLPDecode;

use std::{
    fs::File,
    io::{BufReader, Read as _},
};

// From cmd/ethereum_rust
pub fn read_chain_file(chain_rlp_path: &str) -> Vec<Block> {
    let chain_file = File::open(chain_rlp_path).expect("Failed to open chain rlp file");
    _chain_file(chain_file).expect("Failed to decode chain rlp file")
}

// From cmd/ethereum_rust
pub fn read_genesis_file(genesis_file_path: &str) -> Genesis {
    let genesis_file = std::fs::File::open(genesis_file_path).expect("Failed to open genesis file");
    _genesis_file(genesis_file).expect("Failed to decode genesis file")
}

// From cmd/ethereum_rust/decode.rs
fn _chain_file(file: File) -> Result<Vec<Block>, Box<dyn std::error::Error>> {
    let mut chain_rlp_reader = BufReader::new(file);
    let mut buf = vec![];
    chain_rlp_reader.read_to_end(&mut buf)?;
    let mut blocks = Vec::new();
    while !buf.is_empty() {
        let (item, rest) = Block::decode_unfinished(&buf)?;
        blocks.push(item);
        buf = rest.to_vec();
    }
    Ok(blocks)
}

// From cmd/ethereum_rust/decode.rs
fn _genesis_file(file: File) -> Result<Genesis, serde_json::Error> {
    let genesis_reader = BufReader::new(file);
    serde_json::from_reader(genesis_reader)
}
