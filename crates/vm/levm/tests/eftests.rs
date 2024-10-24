use std::{
    collections::HashMap,
    fs::{self, read_dir},
    path::PathBuf,
    str::FromStr,
};

use bytes::Bytes;
use ethereum_types::{Address, H256, U256, U512};
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
    pub code: Bytes,
    pub nonce: U256,
    pub storage: HashMap<U256, U256>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Transaction {
    data: Vec<Bytes>,
    gas_limit: Vec<U256>,
    gas_price: Option<U256>,
    nonce: U256,
    secret_key: H256,
    sender: Address,
    to: TxDestination,
    value: Vec<U512>,
    access_lists: Option<Vec<Option<Vec<AccesList>>>>,
    blob_versioned_hashes: Option<Vec<H256>>,
    max_fee_per_blob_gas: Option<U256>,
    max_fee_per_gas: Option<U256>,
    max_priority_fee_per_gas: Option<U256>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AccesList {
    address: Address,
    storage_keys: Vec<U256>, // U256 or Address?
}

/// To cover the case when 'to' field is an empty string
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum TxDestination {
    Some(Address),
    #[default]
    None,
}

impl Serialize for TxDestination {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            TxDestination::Some(address) => serializer.serialize_str(&format!("{:#x}", address)),
            TxDestination::None => serializer.serialize_none(),
        }
    }
}

impl<'de> Deserialize<'de> for TxDestination {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let str_option = Option::<String>::deserialize(deserializer)?;
        match str_option {
            Some(str) if !str.is_empty() => Ok(TxDestination::Some(
                Address::from_str(str.trim_start_matches("0x")).map_err(|_| {
                    serde::de::Error::custom(format!("Failed to deserialize hex value {str}"))
                })?,
            )),
            _ => Ok(TxDestination::None),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Index {
    data: u16, // Maybe could be u64, but biggest value i've seen is 452
    gas: u16,
    value: u16,
}

#[derive(Debug, Serialize, Deserialize)]
struct TransactionResults {
    /// define an index of the transaction in txs vector that has been used for this result
    indexes: Index,
    /// hash of the post state after transaction execution
    hash: H256,
    /// log hash of the transaction logs
    logs: H256,
    /// the transaction bytes of the generated transaction
    txbytes: Bytes,
    //expect // Exception	for a transaction that is supposed to fail, the exception
}

/// Contains the necessary elements to run a test
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TestArgs {
    #[serde(default, rename = "_info")]
    pub info: Option<serde_json::Value>,
    /// Contains the environment, the block just before the one that runs the VM or executes the transaction
    env: Env,
    /// Contains the state of the accounts before the transaction execution
    pre: HashMap<Address, Account>,
    /// Contains the state of the environment and db after the transaction execution
    post: HashMap<String, Vec<TransactionResults>>,
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
            if sub_path
                .extension()
                .map(|ext| ext == "json")
                .unwrap_or(false)
            {
                let file_content = fs::read_to_string(sub_path).unwrap();
                contents.push(file_content);
            }
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

/// Note: This behaviour is not the complex that should be to have it's own function
fn parse_contents(json_contents: Vec<String>) -> Vec<HashMap<String, TestArgs>> {
    json_contents
        .into_iter()
        .map(|json_content| {
            println!("{}", &json_content[..55]);
            let res = serde_json::from_str(&json_content).expect("Unable to parse JSON");
            res
        })
        .collect()
}

#[test]
fn ethereum_foundation_general_state_tests() {
    // At this point Ethereum foundation tests should be already downloaded.
    // The ones from https://github.com/ethereum/tests/tree/develop/GeneralStateTests

    let json_contents = parse_files();

    let tests_cases = parse_contents(json_contents);

    for test_case in tests_cases {
        //Maybe there are more than one test per hashmap, so should iterate each hashmap too
        // Initialize

        // Execute

        // Verify

        println!("{:?}", test_case);
    }

    unimplemented!();
}
