use std::{collections::HashMap, path::Path};

use ::ef_tests::types::TestUnit;
use ef_tests::types::Account;
use ethereum_rust_core::{types::Account as CoreAccount, Address};
use ethereum_rust_evm::{evm_state, execute_tx, EvmState, SpecId};
use ethereum_rust_storage::{EngineType, Store};

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
        &mut build_evm_state_from_prestate(&pre),
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

// Creates an in-memory DB for evm execution and loads the prestate accounts
pub fn build_evm_state_from_prestate(pre: &HashMap<Address, Account>) -> EvmState {
    let mut store =
        Store::new("store.db", EngineType::InMemory).expect("Failed to build DB for testing");
    for (address, account) in pre {
        let account: CoreAccount = account.clone().into();
        store
            .add_account(*address, account)
            .expect("Failed to write to test DB")
    }
    evm_state(store)
}
