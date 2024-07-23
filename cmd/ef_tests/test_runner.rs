use std::{collections::HashMap, path::Path};

use crate::types::TestUnit;
use ethereum_rust_core::{
    evm::{execute_tx, SpecId},
    rlp::{decode::RLPDecode, encode::RLPEncode},
    types::Block,
};
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
    let genesis_rlp = test.genesis_rlp.clone();
    let decoded_block = Block::decode(&genesis_rlp).unwrap();
    assert_eq!(
        decoded_block.header,
        test.genesis_block_header.clone().into()
    );

    // check that blocks can be decoded
    for block in &test.blocks {
        // skip the blocks with exceptions expected
        if block.expect_exception.is_some() {
            continue;
        }

        match Block::decode(block.rlp.as_ref()) {
            Ok(decoded_block) => {
                let mut rlp_block = Vec::new();
                decoded_block.encode(&mut rlp_block);
                assert_eq!(decoded_block, (block.clone()).into());
                assert_eq!(rlp_block, block.rlp.to_vec());
            }
            Err(_) => assert!(block.expect_exception.is_some()),
        }
    }
}

pub fn parse_and_execute_test_file(path: &Path) {
    let tests = parse_test_file(path);

    for (_k, test) in tests {
        validate_test(&test);
        //execute_test(&test)
    }
}
