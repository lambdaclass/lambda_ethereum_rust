use core::panic;
use std::{collections::HashMap, path::Path};

use ::ef_tests::types::TestUnit;
use ef_tests::types::{Block, Header};
use ethereum_rust_core::{
    evm::{execute_tx, SpecId},
    rlp::decode::RLPDecode,
};
use std::num::ParseIntError;

pub fn decode_hex(s: &str) -> Result<Vec<u8>, ParseIntError> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
        .collect()
}

fn execute_test(test: &TestUnit) {
    // TODO: Add support for multiple blocks and multiple transactions per block.
    let transaction = test
        .blocks
        .first()
        .unwrap()
        .transactions
        .as_ref()
        .unwrap()
        .first()
        .unwrap();

    let pre = test
        .pre
        .clone()
        .into_iter()
        .map(|(k, v)| (k, v.into()))
        .collect();

    let genesis_rlp_as_string = test.genesis_rlp.clone();
    let genesis_rlp_bytes = decode_hex(&genesis_rlp_as_string.clone()[2..]).unwrap();

    match Block::decode(&genesis_rlp_bytes) {
        Ok(block) => {
            assert_eq!(test.genesis_block_header, block.block_header.unwrap());
        }
        Err(_) => panic!(),
    }

    assert!(execute_tx(
        &transaction.clone().into(),
        &test
            .blocks
            .first()
            .as_ref()
            .unwrap()
            .block_header
            .clone()
            .unwrap()
            .into(),
        &pre,
        SpecId::CANCUN,
    )
    .unwrap()
    .is_success());
}

pub fn parse_test_file(path: &Path) -> HashMap<String, TestUnit> {
    let s: String = std::fs::read_to_string(path).expect("Unable to read file");
    let tests: HashMap<String, TestUnit> = serde_json::from_str(&s).expect("Unable to parse JSON");
    tests
}

pub fn parse_and_execute_test_file(path: &Path) {
    let tests = parse_test_file(path);

    for (_k, test) in tests {
        execute_test(&test)
    }
}
