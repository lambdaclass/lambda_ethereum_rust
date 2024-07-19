use std::{collections::HashMap, path::Path};

use ethereum_rust_core::evm::{execute_tx, SpecId};

use crate::types::TestUnit;

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

pub fn parse_and_execute_test_file(path: &Path) {
    let tests = parse_test_file(path);

    for (_k, test) in tests {
        execute_test(&test)
    }
}
