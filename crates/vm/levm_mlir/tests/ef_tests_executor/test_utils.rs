use std::{collections::HashMap, path::Path};

use bytes::Bytes;
use ethereum_rust_levm_mlir::{
    db::Db,
    env::AccessList,
    report::{InvalidTransaction, TransactionReport, TxResult},
    state,
    utils::precompiled_addresses,
    Environment, Evm,
};

use super::models::{AccountInfo, Test, TestSuite, TestUnit};

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

fn setup_evm(test: &Test, unit: &TestUnit) -> Evm<Db> {
    // let to = unit.transaction.to;
    let sender = unit.transaction.sender.unwrap_or_default();
    let gas_price = unit.transaction.gas_price.unwrap_or_default();
    let mut env = Environment::default();
    env.tx_to = unit.transaction.to;
    env.tx_gas_price = gas_price;
    env.tx_caller = sender;
    env.tx_gas_limit = unit.transaction.gas_limit[test.indexes.gas].as_u64();
    env.tx_value = unit.transaction.value[test.indexes.value];
    env.tx_calldata = decode_hex(unit.transaction.data[test.indexes.data].clone()).unwrap();
    let access_list_vector = unit
        .transaction
        .access_lists
        .get(test.indexes.data)
        .cloned()
        .flatten()
        .unwrap_or_default();
    let mut access_list = AccessList::default();
    for access_list_item in access_list_vector {
        let storage_keys = access_list_item
            .storage_keys
            .iter()
            .map(|key| ethereum_types::U256::from(key.0))
            .collect();

        access_list.push((access_list_item.address, storage_keys));
    }
    access_list.push((env.block_coinbase_address, Vec::new())); // after Shanghai, coinbase address is added to access list
    access_list.push((env.tx_caller, Vec::new())); // after Berlin, tx.sender is added to access list
    access_list.append(&mut precompiled_addresses()); // precompiled address are always warm

    env.block_number = unit.env.current_number;
    env.block_coinbase_address = unit.env.current_coinbase;
    env.block_timestamp = unit.env.current_timestamp;
    let excess_blob_gas = unit
        .env
        .current_excess_blob_gas
        .unwrap_or_default()
        .as_u64();
    env.set_block_blob_base_fee(excess_blob_gas);

    if let Some(basefee) = unit.env.current_base_fee {
        env.block_basefee = basefee;
    };
    let mut db = Db::new();

    // Load pre storage into db
    for (address, account_info) in unit.pre.iter() {
        let opcodes = decode_hex(account_info.code.clone()).unwrap();
        db = db.with_contract(address.to_owned(), opcodes);
        db.set_account(
            address.to_owned(),
            account_info.nonce,
            account_info.balance,
            account_info.storage.clone(),
        );
    }

    Evm::new(env, db)
}

fn verify_result(
    test: &Test,
    expected_result: Option<&Bytes>,
    execution_result: &Result<TransactionReport, InvalidTransaction>,
) -> Result<(), String> {
    match (&test.expect_exception, execution_result) {
        (None, Ok(execution_result)) => {
            // We need to do the .zip as some tests of the ef returns "None" as expected when the results are big
            if let Some((expected_output, output)) =
                expected_result.zip(Some(execution_result.output.clone()))
            {
                if expected_output != &output {
                    return Err("Wrong output".into());
                }
            }
            Ok(())
        }
        (Some(_), Err(_)) => {
            Ok(()) //Got error and expected one
        }
        (Some(err), Ok(execution_result)) => {
            match execution_result.result {
                TxResult::ExceptionalHalt { .. } | TxResult::Revert => {
                    Ok(()) // Got error and got expected halt/revert
                }
                _ => Err(format!("Expected error: {}, but got success", err)),
            }
        }
        (None, Err(err)) => Err(format!("Expected success, but got error: {}", err)),
    }
}

/// Test the resulting storage is the same as the expected storage
fn verify_storage(
    post_state: &HashMap<ethereum_types::H160, AccountInfo>,
    res_state: HashMap<ethereum_types::H160, state::Account>,
) {
    let mut result_state = HashMap::new();
    for address in post_state.keys() {
        let account = res_state.get(address).unwrap();
        let opcodes = decode_hex(account.info.code.clone().unwrap()).unwrap();
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
                    .map(|(addr, slot)| (addr, slot.present_value))
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
            let mut evm = setup_evm(test, &unit);
            let res = evm.transact();
            verify_result(test, unit.out.as_ref(), &res)?;
            // TODO: use rlp and hash to check logs

            if let Ok(res) = res {
                verify_storage(&test.post_state, res.new_state);
            }
        }
    }
    Ok(())
}
