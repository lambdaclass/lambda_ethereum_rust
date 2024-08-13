use std::{collections::HashMap, path::Path};

use crate::types::TestUnit;
use ethereum_rust_core::{
    rlp::decode::RLPDecode,
    rlp::encode::RLPEncode,
    types::{
        validate_block_header, validate_cancun_header_fields, Account as CoreAccount,
        Block as CoreBlock, BlockHeader as CoreBlockHeader,
    },
};
use ethereum_rust_evm::{evm_state, execute_block, validate_block, EvmState, SpecId};
use ethereum_rust_storage::{EngineType, Store};

/// Tests the execute_block function, only run on validated tests, check [validate_test] function.
pub fn execute_test(test_key: &str, test: &TestUnit) {
    // Build pre state
    let mut evm_state = build_evm_state_for_test(test);
    let blocks = test.blocks.clone();

    // Check world_state
    check_prestate_against_db(test_key, test, evm_state.database());

    // Execute all blocks in test
    for block_fixture in blocks.iter() {
        let block: &CoreBlock = &block_fixture.block().unwrap().clone().into();
        let spec = block_fork_spec(test, &block.header);

        let execution_result = execute_block(block, &mut evm_state, spec);

        match execution_result {
            Err(_) => {
                assert!(
                    block_fixture.expect_exception.is_some(),
                    "Expected transaction execution to fail on test: {}",
                    test_key
                );
                return;
            }
            Ok(()) => {
                assert!(
                    block_fixture.expect_exception.is_none(),
                    "Transaction execution failed on test: {} with error: {}",
                    test_key,
                    execution_result.unwrap_err()
                )
            }
        }
    }
    check_poststate_against_db(test_key, test, evm_state.database())
}

pub fn parse_test_file(path: &Path) -> HashMap<String, TestUnit> {
    let s: String = std::fs::read_to_string(path).expect("Unable to read file");
    let tests: HashMap<String, TestUnit> = serde_json::from_str(&s).expect("Unable to parse JSON");
    tests
}

pub fn validate_test(test: &TestUnit) -> bool {
    // check that the decoded genesis block header matches the deserialized one
    let genesis_rlp = test.genesis_rlp.clone();
    let decoded_block = CoreBlock::decode(&genesis_rlp).unwrap();
    let genesis_block_header = CoreBlockHeader::from(test.genesis_block_header.clone());
    assert_eq!(decoded_block.header, genesis_block_header);

    // check that all blocks are valid, tests validate_block function
    let mut parent_block_header = genesis_block_header;
    for block in &test.blocks {
        if let Some(inner_block) = block.block() {
            let core_block = CoreBlock::from(inner_block.clone());
            let spec = block_fork_spec(test, &core_block.header);
            let valid_block = validate_block(&core_block, &parent_block_header, spec);
            if !valid_block {
                assert!(block.expect_exception.is_some());
                return false;
            }
            parent_block_header = core_block.header;
        }

        // check that blocks can be decoded, only when the block has already been validated
        match CoreBlock::decode(block.rlp.as_ref()) {
            Ok(decoded_block) => {
                // check that the decoded block matches the deserialized one
                let inner_block = block.block().unwrap();
                assert_eq!(decoded_block, (inner_block.clone()).into());
                let mut rlp_block = Vec::new();
                // check that encoding the decoded block matches the rlp field
                decoded_block.encode(&mut rlp_block);
                assert_eq!(rlp_block, block.rlp.to_vec());
            }
            Err(_) => {
                assert!(block.expect_exception.is_some() && test.is_rlp_only_test());
                return false;
            }
        }
    }
    true
}

pub fn block_fork_spec(test: &TestUnit, header: &CoreBlockHeader) -> SpecId {
    let spec = match &*test.network {
        "Shanghai" => SpecId::SHANGHAI,
        "Cancun" => SpecId::CANCUN,
        "Paris" => SpecId::MERGE,
        "ShanghaiToCancunAtTime15k" => {
            if header.timestamp >= 15_000 {
                SpecId::CANCUN
            } else {
                SpecId::SHANGHAI
            }
        }
        _ => panic!("Unsupported network: {}", test.network),
    };
    spec
}
/// Creates an in-memory DB for evm execution and loads the prestate accounts
pub fn build_evm_state_for_test(test: &TestUnit) -> EvmState {
    let mut store =
        Store::new("store.db", EngineType::InMemory).expect("Failed to build DB for testing");
    store
        .add_block_header(
            test.genesis_block_header.number.low_u64(),
            test.genesis_block_header.clone().into(),
        )
        .unwrap();
    for (address, account) in &test.pre {
        let account: CoreAccount = account.clone().into();
        store
            .add_account(*address, account)
            .expect("Failed to write to test DB")
    }
    evm_state(store)
}

/// Checks db is correct after setting up initial state
/// Panics if any comparison fails
fn check_prestate_against_db(test_key: &str, test: &TestUnit, db: &Store) {
    let block_number = test.genesis_block_header.number.low_u64();
    let db_block_header = db.get_block_header(block_number).unwrap().unwrap();
    let test_state_root = test.genesis_block_header.state_root;
    assert_eq!(
        test_state_root, db_block_header.state_root,
        "Mismatched genesis state root for database, test: {test_key}"
    );
    assert_eq!(
        test_state_root,
        db.clone().world_state_root(),
        "Mismatched genesis state root for world state trie, test: {test_key}"
    );
}

/// Checks that all accounts in the post-state are present and have the correct values in the DB
/// Panics if any comparison fails
/// Tests that previously failed the validation stage shouldn't be executed with this function.
fn check_poststate_against_db(test_key: &str, test: &TestUnit, db: &Store) {
    for (addr, account) in &test.post_state {
        let expected_account: CoreAccount = account.clone().into();
        // Check info
        let db_account_info = db
            .get_account_info(*addr)
            .expect("Failed to read from DB")
            .unwrap_or_else(|| {
                panic!("Account info for address {addr} not found in DB, test:{test_key}")
            });
        assert_eq!(
            db_account_info, expected_account.info,
            "Mismatched account info for address {addr} test:{test_key}"
        );
        // Check code
        let code_hash = expected_account.info.code_hash;
        let db_account_code = db
            .get_account_code(code_hash)
            .expect("Failed to read from DB")
            .unwrap_or_else(|| {
                panic!("Account code for code hash {code_hash} not found in DB test:{test_key}")
            });
        assert_eq!(
            db_account_code, expected_account.code,
            "Mismatched account code for code hash {code_hash} test:{test_key}"
        );
        // Check storage
        for (key, value) in expected_account.storage {
            let db_storage_value = db
                .get_storage_at(*addr, key)
                .expect("Failed to read from DB")
                .unwrap_or_else(|| {
                    panic!("Storage missing for address {addr} key {key} in DB test:{test_key}")
                });
            assert_eq!(
                db_storage_value, value,
                "Mismatched storage value for address {addr}, key {key} test:{test_key}"
            );
        }
    }
    // Check world state
    // get last valid block
    let last_block = match test.genesis_block_header.hash == test.lastblockhash {
        // lastblockhash matches genesis block
        true => &test.genesis_block_header,
        // lastblockhash matches a block in blocks list
        false => test
            .blocks
            .iter()
            .map(|b| &b.block().unwrap().block_header)
            .find(|h| h.hash == test.lastblockhash)
            .unwrap(),
    };
    let test_state_root = last_block.state_root;
    // TODO: these checks should be enabled once we start storing the Blocks in the DB
    // let db_block_header = db
    //     .get_block_header(test_block.number.low_u64())
    //     .unwrap()
    //     .unwrap();
    // assert_eq!(
    //     test_state_root,
    //     db_block_header.state_root,
    //     "Mismatched state root for database, test: {test_key}");
    assert_eq!(
        test_state_root,
        db.clone().world_state_root(),
        "Mismatched state root for world state trie, test: {test_key}"
    );
}
