use std::{collections::HashMap, path::Path};

use bytes::Bytes;
use levm::{
    block::BlockEnv,
    primitives::U256,
    transaction::Transaction,
    vm::{Environment, Message, TransactTo, WorldState, VM},
    vm_result::ExecutionResult,
};

use super::models::{AccountInfo, Env, Test, TestSuite, TestUnit, TransactionParts};

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
fn setup_transaction(transaction: &TransactionParts) -> Transaction {
    let msg_sender = transaction.sender.unwrap_or_default(); // if not present we derive it from secret key

    // si tiene max prio fee es fee market o blob
    // legacy y access list tienen gas price
    // access list es legacy pero con access list
    // blob es fee market pero con blob versioned hashes

    if let Some(gas_price) = transaction.gas_price {
        if let Some(access_list) = transaction.access_lists.get(0).cloned().flatten() {
            // access list data ( also for Transaction::FeeMarket and Transaction::Blob )
            // let access_list_vector = unit
            //     .transaction
            //     .access_lists
            //     .get(test.indexes.data)
            //     .cloned()
            //     .flatten()
            //     .unwrap_or_default();
            // let mut access_list = AccessList::default();
            // for access_list_item in access_list_vector {
            //     let storage_keys = access_list_item
            //         .storage_keys
            //         .iter()
            //         .map(|key| ethereum_types::U256::from(key.0))
            //         .collect();

            //     access_list.push((access_list_item.address, storage_keys));
            // }
            // access_list.push((env.block.coinbase, Vec::new())); // after Shanghai, coinbase address is added to access list
            // access_list.push((env.tx.caller, Vec::new())); // after Berlin, tx.sender is added to access list
            // access_list.append(&mut precompiled_addresses()); // precompiled address are always warm
            Transaction::AccessList {
                chain_id: 0,
                nonce: transaction.nonce,
                gas_limit: transaction.gas_limit[0].as_u64(),
                msg_sender,
                to: transaction.to,
                value: transaction.value[0],
                gas_price,
                access_list,
                y_parity: U256::zero(),
                data: decode_hex(transaction.data[0].clone()).unwrap(),
            }
        } else {
            Transaction::Legacy {
                chain_id: 0,
                nonce: transaction.nonce,
                gas_limit: transaction.gas_limit[0].as_u64(),
                msg_sender,
                to: transaction.to,
                value: transaction.value[0],
                gas_price,
                data: decode_hex(transaction.data[0].clone()).unwrap(),
            }
        }
    } else if let Some(max_fee_per_blob_gas) = transaction.max_fee_per_blob_gas {
        Transaction::Blob {
            chain_id: 0,
            nonce: transaction.nonce,
            gas_limit: transaction.gas_limit[0].as_u64(),
            msg_sender,
            to: transaction.to,
            value: transaction.value[0],
            max_fee_per_gas: transaction.max_fee_per_gas.unwrap(),
            max_priority_fee_per_gas: transaction.max_priority_fee_per_gas.unwrap(),
            access_list: transaction.access_lists.get(0).cloned().flatten(),
            y_parity: U256::zero(),
            max_fee_per_blob_gas,
            blob_versioned_hashes: transaction.blob_versioned_hashes.clone(),
            data: decode_hex(transaction.data[0].clone()).unwrap(),
        }
    } else {
        Transaction::FeeMarket {
            chain_id: 0,
            nonce: transaction.nonce,
            gas_limit: transaction.gas_limit[0].as_u64(),
            msg_sender,
            to: transaction.to,
            value: transaction.value[0],
            max_fee_per_gas: transaction.max_fee_per_gas.unwrap(),
            max_priority_fee_per_gas: transaction.max_priority_fee_per_gas.unwrap(),
            access_list: transaction.access_lists.get(0).cloned().flatten(),
            y_parity: U256::zero(),
            data: decode_hex(transaction.data[0].clone()).unwrap(),
        }
    }
}

// unit.env ->
// pub struct Env {
//     pub current_coinbase: Address,
//     pub current_difficulty: U256,
//     pub current_gas_limit: U256,
//     pub current_number: U256,
//     pub current_timestamp: U256,
//     pub current_base_fee: Option<U256>,
//     pub previous_hash: Option<H256>,
//     pub current_random: Option<H256>,
//     pub current_beacon_root: Option<H256>,
//     pub current_withdrawals_root: Option<H256>,
//     pub parent_blob_gas_used: Option<U256>,
//     pub parent_excess_blob_gas: Option<U256>,
//     pub current_excess_blob_gas: Option<U256>,
// }

// pub struct BlockEnv {
//     pub number: U256,
//     pub coinbase: Address,
//     pub timestamp: U256,
//     pub base_fee_per_gas: U256,
//     pub gas_limit: usize,
//     pub chain_id: usize,
//     pub prev_randao: Option<H256>,
//     pub excess_blob_gas: Option<u64>,
//     pub blob_gas_used: Option<u64>,
// }

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
    let transaction = setup_transaction(&unit.transaction);
    let block_env = setup_block_env(&unit.env);

    let world_state = WorldState::default();

    // Load pre storage into world state
    for (address, account_info) in unit.pre.iter() {
        let opcodes = decode_hex(account_info.code.clone()).unwrap();
        let account = Account {
            address: address,
            balance: account_info.balance,
            bytecode: opcodes,
            storage: account_info.storage.clone(),
            nonce: account_info.nonce,
        };
        world_state
            .accounts
            .insert(address, account.clone());
    }

    VM::new(transaction, block_env, world_state)
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
fn verify_storage(
    post_state: &HashMap<ethereum_types::H160, AccountInfo>,
    res_state: HashMap<ethereum_types::H160, levm::state::Account>,
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
            let mut vm = setup_vm(test, &unit);
            let res = vm.execute();

            verify_result(test, unit.out.as_ref(), &res.result)?;
            // TODO: use rlp and hash to check logs
            verify_storage(&test.post_state, res.state);
        }
    }
    Ok(())
}
