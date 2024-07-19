use std::{collections::HashMap, path::Path};

use crate::types::TestUnit;
use ethereum_rust_core::{
    evm::{execute_tx, SpecId},
    rlp::{error::RLPDecodeError, structs::Decoder},
    types::{BlockHeader, Transaction, Withdrawal},
};
use std::num::ParseIntError;

pub fn decode_hex(s: &str) -> Result<Vec<u8>, ParseIntError> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
        .collect()
}

/// Decodes a block and returns its header
fn decode_block(rlp: &[u8]) -> Result<BlockHeader, RLPDecodeError> {
    let decoder = Decoder::new(rlp)?;
    let (block_header, decoder) = decoder.decode_field("block_header")?;
    let (_transactions, decoder): (Vec<Transaction>, Decoder) =
        decoder.decode_field("transactions")?;
    let (_ommers, decoder): (Vec<BlockHeader>, Decoder) = decoder.decode_field("ommers")?;
    let (_withdrawals, decoder): (Option<Vec<Withdrawal>>, Decoder) =
        decoder.decode_optional_field();
    let _remaining = decoder.finish()?;
    Ok(block_header)
}

#[allow(unused)]
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

fn validate_test(test: &TestUnit) {
    // check that the decoded genesis block header matches the deserialized one
    let genesis_rlp_as_string = test.genesis_rlp.clone();
    let genesis_rlp_bytes = decode_hex(&genesis_rlp_as_string.clone()[2..]).unwrap();
    let block_header = decode_block(&genesis_rlp_bytes).unwrap();
    assert_eq!(block_header, test.genesis_block_header.clone().into());

    // check that blocks can be decoded
    for block in &test.blocks {
        assert!(decode_block(block.rlp.as_ref()).is_ok() || block.expect_exception.is_some())
    }
}

pub fn parse_and_execute_test_file(path: &Path) {
    let tests = parse_test_file(path);

    for (_k, test) in tests {
        validate_test(&test);
        //execute_test(&test)
    }
}
