use std::{collections::HashMap, path::Path};

use levm::{
    block::BlockEnv,
    primitives::{Bytes, H160, U256},
    transaction::{TransactTo, TxEnv},
    vm::{Account, Db, StorageSlot, VM},
    vm_result::{ExecutionResult, StateAccount},
};

use crate::ef_tests_executor::models::AccountInfo;

// use levm::{
//     block::BlockEnv,
//     primitives::U256,
//     transaction::Transaction,
//     vm::{Account, Environment, Message, StorageSlot, TransactTo, WorldState, VM},
//     vm_result::ExecutionResult,
// };

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

// unit.transaction ->
// pub struct TransactionParts {
//     pub data: Vec<Bytes>,
//     pub gas_limit: Vec<U256>,
//     pub gas_price: Option<U256>,
//     pub nonce: U256,
//     pub secret_key: H256,
//     /// if sender is not present we need to derive it from secret key.
//     #[serde(default)]
//     pub sender: Option<Address>,
//     #[serde(deserialize_with = "deserialize_maybe_empty")]
//     pub to: Option<Address>,
//     pub value: Vec<U256>,
//     pub max_fee_per_gas: Option<U256>,
//     pub max_priority_fee_per_gas: Option<U256>,
//     #[serde(default)]
//     pub access_lists: Vec<Option<AccessList>>,
//     #[serde(default)]
//     pub blob_versioned_hashes: Vec<H256>,
//     pub max_fee_per_blob_gas: Option<U256>,
// }

// TX ENUM ->
// pub enum Transaction {
//     Legacy {
//         chain_id: u64,
//         nonce: U256,
//         gas_limit: u64,
//         msg_sender: Address,
//         to: Option<Address>,
//         value: U256,
//         gas_price: u64,
//         data: Bytes,
//     },
//     AccessList {
//         chain_id: u64,
//         nonce: U256,
//         gas_limit: u64,
//         msg_sender: Address,
//         to: Option<Address>,
//         value: U256,
//         gas_price: u64,
//         access_list: AccessList,
//         y_parity: U256,
//         data: Bytes,
//     },
//     FeeMarket {
//         chain_id: u64,
//         nonce: U256,
//         gas_limit: u64,
//         msg_sender: Address,
//         to: Option<Address>,
//         value: U256,
//         max_fee_per_gas: u64,
//         max_priority_fee_per_gas: u64,
//         access_list: AccessList,
//         y_parity: U256,
//         data: Bytes,
//     },
//     Blob {
//         chain_id: u64,
//         nonce: U256,
//         gas_limit: u64,
//         msg_sender: Address,
//         to: Address, // must not be null
//         value: U256,
//         max_fee_per_gas: u64,
//         max_priority_fee_per_gas: u64,
//         access_list: AccessList,
//         y_parity: U256,
//         max_fee_per_blob_gas: U256,
//         blob_versioned_hashes: Vec<VersionedHash>,
//         data: Bytes,
//     },
// }
// fn setup_transaction(transaction: &TransactionParts) -> Transaction {
//     let msg_sender = transaction.sender.unwrap_or_default(); // if not present we derive it from secret key

//     // si tiene max prio fee es fee market o blob
//     // legacy y access list tienen gas price
//     // access list es legacy pero con access list
//     // blob es fee market pero con blob versioned hashes

//     if let Some(gas_price) = transaction.gas_price {
//         if let Some(access_list) = transaction.access_lists.get(0).cloned().flatten() {
//             // access list data ( also for Transaction::FeeMarket and Transaction::Blob )
//             // let access_list_vector = unit
//             //     .transaction
//             //     .access_lists
//             //     .get(test.indexes.data)
//             //     .cloned()
//             //     .flatten()
//             //     .unwrap_or_default();
//             // let mut access_list = AccessList::default();
//             // for access_list_item in access_list_vector {
//             //     let storage_keys = access_list_item
//             //         .storage_keys
//             //         .iter()
//             //         .map(|key| ethereum_types::U256::from(key.0))
//             //         .collect();

//             //     access_list.push((access_list_item.address, storage_keys));
//             // }
//             // access_list.push((env.block.coinbase, Vec::new())); // after Shanghai, coinbase address is added to access list
//             // access_list.push((env.tx.caller, Vec::new())); // after Berlin, tx.sender is added to access list
//             // access_list.append(&mut precompiled_addresses()); // precompiled address are always warm
//             Transaction::AccessList {
//                 chain_id: 0,
//                 nonce: transaction.nonce,
//                 gas_limit: transaction.gas_limit[test.indexes.gas].as_u64();
//                 msg_sender,
//                 to: transaction.to,
//                 value: transaction.value[0],
//                 gas_price,
//                 access_list,
//                 y_parity: U256::zero(),
//                 data: decode_hex(transaction.data[0].clone()).unwrap(),
//             }
//         } else {
//             Transaction::Legacy {
//                 chain_id: 0,
//                 nonce: transaction.nonce,
//                 gas_limit: transaction.gas_limit[0].as_u64(),
//                 msg_sender,
//                 to: transaction.to,
//                 value: transaction.value[0],
//                 gas_price,
//                 data: decode_hex(transaction.data[0].clone()).unwrap(),
//             }
//         }
//     } else if let Some(max_fee_per_blob_gas) = transaction.max_fee_per_blob_gas {
//         Transaction::Blob {
//             chain_id: 0,
//             nonce: transaction.nonce,
//             gas_limit: transaction.gas_limit[0].as_u64(),
//             msg_sender,
//             to: transaction.to,
//             value: transaction.value[0],
//             max_fee_per_gas: transaction.max_fee_per_gas.unwrap(),
//             max_priority_fee_per_gas: transaction.max_priority_fee_per_gas.unwrap(),
//             access_list: transaction.access_lists.get(0).cloned().flatten(),
//             y_parity: U256::zero(),
//             max_fee_per_blob_gas,
//             blob_versioned_hashes: transaction.blob_versioned_hashes.clone(),
//             data: decode_hex(transaction.data[0].clone()).unwrap(),
//         }
//     } else {
//         Transaction::FeeMarket {
//             chain_id: 0,
//             nonce: transaction.nonce,
//             gas_limit: transaction.gas_limit[0].as_u64(),
//             msg_sender,
//             to: transaction.to,
//             value: transaction.value[0],
//             max_fee_per_gas: transaction.max_fee_per_gas.unwrap(),
//             max_priority_fee_per_gas: transaction.max_priority_fee_per_gas.unwrap(),
//             access_list: transaction.access_lists.get(0).cloned().flatten(),
//             y_parity: U256::zero(),
//             data: decode_hex(transaction.data[0].clone()).unwrap(),
//         }
//     }
// }

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
        max_fee_per_blob_gas: transaction.max_fee_per_gas,
    }
}

fn setup_block_env(env: &Env) -> BlockEnv {
    let mut block_env = BlockEnv::default();
    block_env.number = env.current_number;
    block_env.coinbase = env.current_coinbase;
    block_env.timestamp = env.current_timestamp;
    block_env.base_fee_per_gas = env.current_base_fee.unwrap_or_default();
    block_env.gas_limit = env.current_gas_limit.as_u64() as usize;
    block_env.chain_id = 0;
    block_env.prev_randao = env.current_random;
    block_env.excess_blob_gas = if let Some(excess_blob_gas) = env.current_excess_blob_gas {
        Some(excess_blob_gas.as_u64())
    } else {
        None
    };
    block_env
}

fn setup_vm(test: &Test, unit: &TestUnit) -> VM {
    let tx_env = setup_txenv(&unit.transaction, test);
    let block_env = setup_block_env(&unit.env);

    let mut db = Db::default();

    // Load pre storage into db
    for (address, account_info) in unit.pre.iter() {
        let opcodes = decode_hex(account_info.code.clone()).unwrap();

        // pub struct Account {
        //     pub address: Address,
        //     pub balance: U256,
        //     pub bytecode: Bytes,
        //     pub storage: HashMap<U256, StorageSlot>,
        //     pub nonce: U256,
        // }

        let storage = account_info
            .storage
            .iter()
            .map(|(key, value)| {
                (
                    U256::from(key.clone()),
                    StorageSlot {
                        original_value: value.clone(),
                        current_value: value.clone(),
                        is_cold: false,
                    },
                )
            })
            .collect();

        let account = Account::new(
            address.clone(),
            account_info.balance,
            opcodes,
            account_info.nonce,
            storage,
        );

        db.accounts.insert(address.clone(), account.clone());
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
