use std::collections::HashMap;

use ::ef_tests::types::TestUnit;
use ethrex_core::evm::{execute_tx, SpecId};

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
    .is_success());
}

fn parse_test_file(file: &str) -> HashMap<String, TestUnit> {
    let s: String = std::fs::read_to_string(file).expect("Unable to read file");
    let tests: HashMap<String, TestUnit> = serde_json::from_str(&s).expect("Unable to parse JSON");
    tests
}

fn parse_and_execute_test_file(file: &str) {
    let tests = parse_test_file(file);
    for (_k, test) in tests {
        execute_test(&test)
    }
}

#[cfg(test)]
mod ef_tests {
    use crate::parse_and_execute_test_file;

    #[test]
    fn beacon_root_contract_calls_test() {
        parse_and_execute_test_file("./vectors/cancun/eip4788_beacon_root/beacon_root_contract/beacon_root_contract_calls.json");
    }
}
