use std::{collections::HashMap, path::Path};

use ethereum_rust_levm::{
    block::BlockEnv,
    primitives::{Bytes, H160},
    transaction::{TransactTo, TxEnv},
    vm::{Account, Db, StorageSlot, VM},
    vm_result::{ExecutionResult, StateAccount},
};

use crate::ef_tests_executor::models::AccountInfo;

use super::models::{Env, Test, TestSuite, TestUnit, TransactionParts};

/// Receives a Bytes object with the hex representation
/// And returns a Bytes object with the decimal representation
/// Taking the hex numbers by pairs
fn decode_hex(bytes_in_hex: Bytes) -> Option<Bytes> {
    let hex_header = &bytes_in_hex[0..2];
    if hex_header != b"0x" {
        return None;
    }
    let hex_string = std::str::from_utf8(&bytes_in_hex[2..]).unwrap(); // we don't need the 0x
    let mut opcodes = Vec::new();
    for i in (0..hex_string.len()).step_by(2) {
        let pair = &hex_string[i..i + 2];
        let value = u8::from_str_radix(pair, 16).unwrap();
        opcodes.push(value);
    }
    Some(Bytes::from(opcodes))
}

fn setup_txenv(transaction: &TransactionParts, test: &Test) -> TxEnv {
    let msg_sender = transaction.sender.unwrap_or_default(); // if not present we derive it from secret key
    let transact_to: TransactTo = match transaction.to {
        Some(to) => TransactTo::Call(to),
        None => TransactTo::Create,
    };

    TxEnv {
        msg_sender,
        gas_limit: transaction.gas_limit[test.indexes.gas].as_u64(),
        gas_price: transaction.gas_price,
        transact_to,
        value: transaction.value[test.indexes.value],
        chain_id: Some(0),
        data: decode_hex(transaction.data[test.indexes.data].clone()).unwrap(),
        nonce: Some(transaction.nonce.as_u64()),
        // access_list: transaction.access_lists.get(0).cloned().flatten(),
        access_list: None,
        max_priority_fee_per_gas: transaction.max_priority_fee_per_gas,
        blob_hashes: transaction.blob_versioned_hashes.clone(),
        max_fee_per_blob_gas: transaction.max_fee_per_blob_gas,
        max_fee_per_gas: transaction.max_fee_per_gas,
    }
}

fn setup_block_env(env: &Env) -> BlockEnv {
    BlockEnv {
        number: env.current_number,
        coinbase: env.current_coinbase,
        timestamp: env.current_timestamp,
        base_fee_per_gas: env.current_base_fee.unwrap_or_default(),
        gas_limit: env.current_gas_limit.as_u64() as usize,
        chain_id: 0,
        prev_randao: env.current_random,
        excess_blob_gas: env
            .current_excess_blob_gas
            .map(|excess_blob_gas| excess_blob_gas.as_u64()),
        ..Default::default()
    }
}

fn setup_vm(test: &Test, unit: &TestUnit) -> VM {
    let tx_env = setup_txenv(&unit.transaction, test);
    let block_env = setup_block_env(&unit.env);

    let mut db = Db::default();

    // Load pre storage into db
    for (address, account_info) in unit.pre.iter() {
        let opcodes = decode_hex(account_info.code.clone()).unwrap();

        let storage = account_info
            .storage
            .iter()
            .map(|(key, value)| {
                (
                    *key,
                    StorageSlot {
                        original_value: *value,
                        current_value: *value,
                        is_cold: false,
                    },
                )
            })
            .collect();

        let account = Account::new(
            *address,
            account_info.balance,
            opcodes,
            account_info.nonce,
            storage,
        );

        db.accounts.insert(*address, account.clone());
    }

    VM::new(tx_env, block_env, db)
}

fn verify_result(
    test: &Test,
    expected_result: Option<&Bytes>,
    execution_result: &ExecutionResult,
) -> Result<(), String> {
    match (&test.expect_exception, execution_result) {
        (None, _) => {
            // We need to do the .zip as some tests of the ef returns "None" as expected when the results are big
            if let Some((expected_output, output)) = expected_result.zip(execution_result.output())
            {
                if expected_output != output {
                    return Err("Wrong output".into());
                }
            }
            Ok(())
        }
        (Some(_), ExecutionResult::Halt { .. } | ExecutionResult::Revert { .. }) => {
            Ok(()) //Halt/Revert and want an error
        }
        _ => Err("Expected exception but got none".into()),
    }
}

/// Test the resulting storage is the same as the expected storage
fn verify_storage(post_state: &HashMap<H160, AccountInfo>, res_state: HashMap<H160, StateAccount>) {
    let mut result_state = HashMap::new();
    for address in post_state.keys() {
        let account = res_state.get(address).unwrap();
        let opcodes = decode_hex(account.info.code.clone()).unwrap();
        result_state.insert(
            address.to_owned(),
            AccountInfo {
                balance: account.info.balance,
                code: opcodes,
                nonce: account.info.nonce,
                storage: account
                    .storage
                    .clone()
                    .into_iter()
                    .map(|(addr, slot)| (addr, slot.current_value))
                    .collect(),
            },
        );
    }
    assert_eq!(*post_state, result_state);
}

pub fn run_test(path: &Path, contents: String) -> datatest_stable::Result<()> {
    let test_suite: TestSuite = serde_json::from_reader(contents.as_bytes())
        .unwrap_or_else(|_| panic!("Failed to parse JSON test {}", path.display()));

    for (_name, unit) in test_suite.0 {
        // NOTE: currently we only support Cancun spec
        let Some(tests) = unit.post.get("Cancun") else {
            continue;
        };

        for test in tests {
            let mut vm = setup_vm(test, &unit);
            let res = vm.transact().unwrap();

            verify_result(test, unit.out.as_ref(), &res.result)?;
            // TODO: use rlp and hash to check logs
            verify_storage(&test.post_state, res.state);
        }
    }
    Ok(())
}
