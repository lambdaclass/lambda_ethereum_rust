use std::{
    collections::HashMap,
    fs::{self, read_dir},
    path::{Path, PathBuf},
    str::FromStr,
};

use bytes::Bytes;
use ethereum_rust_levm::{
    errors::VMError,
    vm::{Account, Db, StorageSlot, VM},
};
use ethereum_types::{Address, H256, U256};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Env {
    current_base_fee: Option<U256>,
    current_coinbase: Address,
    current_difficulty: U256,
    current_excess_blob_gas: Option<U256>,
    current_gas_limit: U256,
    current_number: U256,
    current_random: Option<H256>,
    current_timestamp: U256,
    previous_hash: Option<H256>,
}

// Taken from cmd/ef_tests/types.rs
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub struct ReadedAccount {
    pub balance: U256,
    #[serde(rename = "code", with = "ethereum_rust_core::serde_utils::bytes")]
    pub code: Bytes,
    pub nonce: U256,
    pub storage: HashMap<U256, U256>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Transaction {
    #[serde(rename = "data", with = "ethereum_rust_core::serde_utils::bytes::vec")]
    data: Vec<Bytes>,
    gas_limit: Vec<U256>,
    gas_price: Option<U256>,
    nonce: U256,
    secret_key: H256,
    sender: Address,
    to: TxDestination,
    value: Vec<U256>, // Using serde_json::Value would not rise an error, but, works?
    access_lists: Option<Vec<Option<Vec<AccesList>>>>,
    blob_versioned_hashes: Option<Vec<H256>>,
    max_fee_per_blob_gas: Option<U256>,
    max_fee_per_gas: Option<U256>,
    max_priority_fee_per_gas: Option<U256>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
    #[serde(rename = "txbytes", with = "ethereum_rust_core::serde_utils::bytes")]
    txbytes: Bytes,
    /// For a transaction that is supposed to fail, the exception
    expect_exception: Option<String>,
}

/// Contains the necessary elements to run a test
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TestArgs {
    #[serde(default, rename = "_info")]
    pub info: Option<serde_json::Value>,
    /// Contains the environment, the block just before the one that runs the VM or executes the transaction
    env: Env,
    /// Contains the state of the accounts before the transaction execution
    pre: HashMap<Address, ReadedAccount>,
    /// Contains the state of the environment and db after the transaction execution
    post: HashMap<String, Vec<TransactionResults>>,
    /// Contains the transaction to execute
    transaction: Transaction,
}

fn file_extension_is_json(path: &Path) -> bool {
    path.extension().map(|ext| ext == "json").unwrap_or(false)
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
        } else if file_extension_is_json(sub_path) {
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

/// Note: This behaviour is not the complex that should be to have it's own function
fn parse_contents(json_contents: Vec<String>) -> Vec<HashMap<String, TestArgs>> {
    json_contents
        .into_iter()
        .map(|json_content| {
            //println!("{}", &json_content[..55]);
            serde_json::from_str(&json_content).expect("Unable to parse JSON")
        })
        .collect()
}

fn init_environment(test_args: &TestArgs) -> Result<VM, VMError> {
    // Be careful with clone (performance)
    let accounts = test_args
        .pre
        .clone()
        .into_iter()
        .map(|(add, acc)| {
            let storage: HashMap<U256, StorageSlot> = acc
                .storage
                .into_iter()
                .map(|(key, val)| {
                    let slot = StorageSlot {
                        original_value: val,
                        current_value: val,
                        is_cold: true,
                    };
                    (key, slot)
                })
                .collect();
            let new_acc = Account {
                address: add,
                balance: acc.balance,
                bytecode: acc.code,
                storage,
                nonce: acc.nonce.as_u64(),
            };
            (add, new_acc)
        })
        .collect();

    let mut db = Db {
        accounts,
        block_hashes: Default::default(), // Dont know where is set
    };

    let destination: Option<Address> = match test_args.transaction.to {
        TxDestination::Some(address) => Some(address),
        TxDestination::None => None,
    };
    // Note: here TxDestination should be matched to an Address Option (when creating is Null)
    // but Vm does not support an option in address, so create is not supported currently

    VM::new(
        destination,
        test_args.transaction.sender,
        test_args.transaction.value[0], // Maybe there's more than one value, see GeneralStateTests/stTransactionTest/NoSrcAccount.json
        test_args.transaction.data[0].clone(), // There's more than one values in vector, see GeneralStateTests/Cancun/stEIP1153-transientStorage/transStorageOK.json
        test_args.transaction.gas_limit[0],    // Same as above comments
        test_args.env.current_number,
        test_args.env.current_coinbase,
        test_args.env.current_timestamp,
        Some(H256::zero()),
        U256::one(),
        test_args.env.current_base_fee.unwrap(),
        test_args
            .transaction
            .gas_price
            .unwrap_or(test_args.transaction.max_fee_per_gas.unwrap_or_default()),
        &mut db,
        Default::default(), // Dont know where is set
        test_args.env.current_excess_blob_gas,
        Default::default(),               // Dont know where is set
        test_args.transaction.secret_key, // Dont know where is set
        None,
    )
}

#[test]
fn ethereum_foundation_general_state_tests() {
    // At this point Ethereum foundation tests should be already downloaded.
    // The ones from https://github.com/ethereum/tests/tree/develop/GeneralStateTests

    let json_contents = parse_files();

    let tests_cases: Vec<HashMap<String, TestArgs>> = parse_contents(json_contents);

    let tests_cases = &tests_cases[0..1];

    for test_case in tests_cases {
        //Maybe there are more than one test per hashmap, so should iterate each hashmap too

        for (test_name, test_args) in test_case {
            // Initialize
            println!("Parseando el test {:?}", test_name);
            let mut vm = init_environment(test_args).expect("An error happened at init of test.");

            // Execute
            //println!("Executing testcase {test_name}");
            //let _result = vm.transact();

            // Verify
            /*
            Possible tests:
            - Verify the result of the execution
            - Verify the hash state
            - Verify the result of the accounts individually (possibly require equal )
            - Check if gas costs have been applied to the sender
             */

            /*
                       if let Err(e) = result {
                           if e.into() == test_args.post.get("Cancun").unwrap().get(0).unwrap().expect_exception.unwrap() {
                               println!("El error es el esperado: {:?}", e);
                           }
                       }
            */
            /*
               // See if vm.call_frames.len() is equivalent to the amount of logs
               for i in 0..vm.call_frames.len() {
                   assert_eq!(test_args.post.get("Cancun").unwrap().get(i).unwrap().log, from(vm.call_frames[0].logs[i].data));
               }
            */

            println!("Test name: {}", test_name);
        }
    }

    // unimplemented!();
}
