use std::{collections::HashMap, path::Path};

use crate::types::{Account, TestUnit};
use ethereum_rust_core::{
    rlp::decode::RLPDecode,
    rlp::encode::RLPEncode,
    types::{Account as CoreAccount, Block as CoreBlock, Transaction as CoreTransaction},
    Address,
};
use ethereum_rust_evm::{
    apply_state_transitions, evm_state, execute_tx, process_withdrawals, EvmState, SpecId,
};
use ethereum_rust_storage::{EngineType, Store};

pub fn execute_test(test_key: &str, test: &TestUnit, check_post_state: bool) {
    // Build pre state
    let mut evm_state = build_evm_state_from_prestate(&test.pre);
    let blocks = test.blocks.clone();
    // Execute all txs in the test unit
    for block in blocks.iter() {
        let block_header = block.header().clone();
        for (tx_index, transaction) in block.transactions().iter().enumerate() {
            assert_eq!(
                transaction.clone().sender,
                CoreTransaction::from(transaction.clone()).sender(),
                "Expected sender address differs from derived sender address on test: {}",
                test_key
            );
            let execution_result = execute_tx(
                &transaction.clone().into(),
                &block_header.clone().into(),
                &mut evm_state,
                SpecId::CANCUN,
            );
            // If this is the last tx in a block that is expecting an exception then we must make sure it fails
            // TODO: Check that the exception is the one in the test unit
            let is_last_tx = block.transactions().len() == tx_index + 1;
            if block.expect_exception.is_some() && is_last_tx {
                assert!(
                    execution_result.is_err(),
                    "Expected transaction execution to fail on test: {}",
                    test_key
                )
            } else {
                assert!(
                    execution_result.is_ok(),
                    "Transaction execution failed on test: {} with error: {}",
                    test_key,
                    execution_result.unwrap_err()
                )
            }
        }
        // Apply state transitions
        apply_state_transitions(&mut evm_state).expect("Failed to update DB state");
        // Process withdrawals (if present)
        if let Some(withdrawals) = block.withdrawals() {
            process_withdrawals(evm_state.database(), withdrawals)
                .expect("DB error when processing withdrawals")
        }
    }
    // Check post state
    if check_post_state {
        check_poststate_against_db(&test.post_state, evm_state.database())
    }
}

pub fn parse_test_file(path: &Path) -> HashMap<String, TestUnit> {
    let s: String = std::fs::read_to_string(path).expect("Unable to read file");
    let tests: HashMap<String, TestUnit> = serde_json::from_str(&s).expect("Unable to parse JSON");
    tests
}

pub fn validate_test(test: &TestUnit) {
    // check that the decoded genesis block header matches the deserialized one
    let genesis_rlp = test.genesis_rlp.clone();
    let decoded_block = CoreBlock::decode(&genesis_rlp).unwrap();
    assert_eq!(
        decoded_block.header,
        test.genesis_block_header.clone().into()
    );

    // check that blocks can be decoded
    for block in &test.blocks {
        match CoreBlock::decode(block.rlp.as_ref()) {
            Ok(decoded_block) => {
                // check that the decoded block matches the deserialized one
                assert_eq!(decoded_block, (block.block().clone()).into());
                let mut rlp_block = Vec::new();
                // check that encoding the decoded block matches the rlp field
                decoded_block.encode(&mut rlp_block);
                assert_eq!(rlp_block, block.rlp.to_vec());
            }
            Err(_) => assert!(block.expect_exception.is_some()),
        }
    }
}

/// Creates an in-memory DB for evm execution and loads the prestate accounts
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

/// Checks that all accounts in the post-state are present and have the correct values in the DB
/// Panics if any comparison fails
fn check_poststate_against_db(post: &HashMap<Address, Account>, db: &Store) {
    for (addr, account) in post {
        let expected_account: CoreAccount = account.clone().into();
        // Check info
        let db_account_info = db
            .get_account_info(*addr)
            .expect("Failed to read from DB")
            .unwrap_or_else(|| panic!("Account info for address {addr} not found in DB"));
        assert_eq!(
            db_account_info, expected_account.info,
            "Mismatched account info for address {addr}"
        );
        // Check code
        let code_hash = expected_account.info.code_hash;
        let db_account_code = db
            .get_account_code(code_hash)
            .expect("Failed to read from DB")
            .unwrap_or_else(|| panic!("Account code for code hash {code_hash} not found in DB"));
        assert_eq!(
            db_account_code, expected_account.code,
            "Mismatched account code for code hash {code_hash}"
        );
        // Check storage
        for (key, value) in expected_account.storage {
            let db_storage_value = db
                .get_storage_at(*addr, key)
                .expect("Failed to read from DB")
                .unwrap_or_else(|| panic!("Storage missing for address {addr} key {key} in DB"));
            assert_eq!(
                db_storage_value, value,
                "Mismatched storage value for address {addr}, key {key}"
            );
        }
    }
}
