use std::{
    collections::HashMap,
    fs::{self, read_dir},
    path::PathBuf,
};

use bytes::Bytes;
use ethereum_types::{Address, U256};
use keccak_hash::H256;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Env {
    current_base_fee: Option<U256>,
    current_coinbase: Address,
    current_difficulty: U256,
    current_excess_blob_gas: Option<U256>,
    current_gas_limit: U256,
    current_number: U256,
    current_random: Option<H256>,
    current_timestamp: U256,
}

// Taken from cmd/ef_tests/types.rs
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub struct Account {
    pub balance: U256,
    //#[serde(with = "ethereum_rust_core::serde_utils::bytes")]
    //pub code: Bytes,
    pub nonce: U256,
    pub storage: HashMap<U256, U256>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Transaction {
    data: Bytes,
    gas_limit: u64,
    gas_price: u64,
    nonce: u64,
    secret_key: u64,
    sender: Address,
    to: Address,
    value: Vec<u64>,
}


/// Contains the necessary elements to run a test
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TestArgs {
    #[serde(default, rename = "_info")]
    pub info: Option<serde_json::Value>,
    /// Contains the environment, the block just before the one that runs the VM or executes the transaction
    env: Env,
    /// Contains the state of the environment and db before the transaction execution
    pre: HashMap<Address, Account>,
    /// Contains the state of the environment and db after the transaction execution
    post: HashMap<Address, Account>,
    /// Contains the transaction to execute
    transaction: Transaction,
}

fn directory_contents(path: &PathBuf, contents: &mut Vec<String>) {
    let sub_paths: Vec<PathBuf> = read_dir(path)
        .unwrap()
        .filter_map(|entry| match entry {
            Ok(direntry) => Some(direntry.path()),
            Err(err) => {
                eprintln!("Error reading directory entry: {}", err);
                None
            }
        })
        .collect();

    for sub_path in &sub_paths {
        if sub_path.is_dir() {
            directory_contents(sub_path, contents);
        } else {
            let file_content = fs::read_to_string(sub_path).unwrap();
            contents.push(file_content);
        }
    }
}

/// Parses the content of the files into the TestCase struct
fn parse_files() -> Vec<String> {
    let paths: Vec<PathBuf> = read_dir("tests/ef_testcases/GeneralStateTests")
        .unwrap()
        .filter_map(|entry| match entry {
            Ok(direntry) => Some(direntry.path()),
            Err(err) => {
                eprintln!("Error reading directory entry: {}", err);
                None
            }
        })
        .collect();

    let mut contents = Vec::new();

    for path in paths {
        if path.is_dir() {
            directory_contents(&path, &mut contents);
        } else {
            let file_content = fs::read_to_string(path).unwrap();
            contents.push(file_content);
        }
    }

    contents
}

#[test]
fn ethereum_foundation_general_state_tests() {
    // At this point Ethereum foundation tests should be already downloaded.
    // The ones from https://github.com/ethereum/tests/tree/develop/GeneralStateTests

    let json_contents = parse_files();

    let tests_cases: Vec<HashMap<String, TestArgs>> = json_contents
        .into_iter()
        .map(|json_content| {
            println!("{}",&json_content[..55]);
            serde_json::from_str(&json_content).expect("Unable to parse JSON")
        })
            .collect();

    for test_case in tests_cases {
        //Maybe there are more than one test per hashmap, so should iterate each hashmap too
        // Initialize

        // Execute

        // Verify

        println!("{:?}", test_case);

    }

    unimplemented!();
}
