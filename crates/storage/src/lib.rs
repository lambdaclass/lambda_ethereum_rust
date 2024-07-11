//! Abstraction for persistent data storage.
//!    Supporting InMemory and Libmdbx storage.
//!    There is also a template for Sled and RocksDb implementation in case we
//!    want to test or benchmark against those engines (Currently disabled behind feature flags
//!    to avoid requiring the implementation of the full API).

use self::error::StoreError;
#[cfg(feature = "in_memory")]
use self::in_memory::Store as InMemoryStore;
#[cfg(feature = "libmdbx")]
use self::libmdbx::Store as LibmdbxStore;
#[cfg(feature = "rocksdb")]
use self::rocksdb::Store as RocksDbStore;
#[cfg(feature = "sled")]
use self::sled::Store as SledStore;
use ethereum_rust_core::types::AccountInfo;
use ethereum_types::Address;
use std::fmt::Debug;
use std::sync::{Arc, Mutex};

mod error;
mod rlp;

#[cfg(feature = "in_memory")]
mod in_memory;
#[cfg(feature = "libmdbx")]
mod libmdbx;
#[cfg(feature = "rocksdb")]
mod rocksdb;
#[cfg(feature = "sled")]
mod sled;

pub(crate) type Key = Vec<u8>;
pub(crate) type Value = Vec<u8>;

pub trait StoreEngine: Debug + Send {
    /// Add account info
    fn add_account_info(
        &mut self,
        address: Address,
        account_info: AccountInfo,
    ) -> Result<(), StoreError>;

    /// Obtain account info
    fn get_account_info(&self, address: Address) -> Result<Option<AccountInfo>, StoreError>;

    /// Set an arbitrary value (used for eventual persistent values: eg. current_block_height)
    fn set_value(&mut self, key: Key, value: Value) -> Result<(), StoreError>;

    /// Retrieve a stored value under Key
    fn get_value(&self, key: Key) -> Result<Option<Value>, StoreError>;
}

#[derive(Debug, Clone)]
pub struct Store {
    engine: Arc<Mutex<dyn StoreEngine>>,
}

#[allow(dead_code)]
pub enum EngineType {
    #[cfg(feature = "in_memory")]
    InMemory,
    #[cfg(feature = "libmdbx")]
    Libmdbx,
    #[cfg(feature = "sled")]
    Sled,
    #[cfg(feature = "rocksdb")]
    RocksDb,
}

impl Store {
    pub fn new(path: &str, engine_type: EngineType) -> Result<Self, StoreError> {
        let store = match engine_type {
            #[cfg(feature = "libmdbx")]
            EngineType::Libmdbx => Self {
                engine: Arc::new(Mutex::new(LibmdbxStore::new(path)?)),
            },
            #[cfg(feature = "in_memory")]
            EngineType::InMemory => Self {
                engine: Arc::new(Mutex::new(InMemoryStore::new()?)),
            },
            #[cfg(feature = "sled")]
            EngineType::Sled => Self {
                engine: Arc::new(Mutex::new(SledStore::new(path)?)),
            },
            #[cfg(feature = "rocksdb")]
            EngineType::RocksDb => Self {
                engine: Arc::new(Mutex::new(RocksDbStore::new(path)?)),
            },
        };
        Ok(store)
    }

    pub fn add_account_info(
        &mut self,
        address: Address,
        account_info: AccountInfo,
    ) -> Result<(), StoreError> {
        self.engine
            .clone()
            .lock()
            .unwrap()
            .add_account_info(address, account_info)
    }

    pub fn get_account_info(&self, address: Address) -> Result<Option<AccountInfo>, StoreError> {
        self.engine
            .clone()
            .lock()
            .unwrap()
            .get_account_info(address)
    }
}

#[cfg(test)]
mod tests {
    use std::{env, fs};

    use bytes::Bytes;
    use ethereum_rust_core::types;
    use ethereum_types::U256;

    use super::*;

    #[cfg(feature = "in_memory")]
    #[test]
    fn test_in_memory_store() {
        let store = Store::new("test", EngineType::InMemory).unwrap();
        test_store_account(store.clone());
    }

    #[cfg(feature = "libmdbx")]
    #[test]
    fn test_libmdbx_store() {
        // Removing preexistent DBs in case of a failed previous test
        remove_test_dbs("test.mdbx");
        let store = Store::new("test.mdbx", EngineType::Libmdbx).unwrap();
        test_store_account(store.clone());

        remove_test_dbs("test.mdbx");
    }

    #[cfg(feature = "sled")]
    #[test]
    fn test_sled_store() {
        // Removing preexistent DBs in case of a failed previous test
        remove_test_dbs("test.sled");
        let store = Store::new("test.sled", EngineType::Sled).unwrap();
        test_store_account(store.clone());

        remove_test_dbs("test.sled");
    }

    #[cfg(feature = "rocksdb")]
    #[test]
    fn test_rocksdb_store() {
        // Removing preexistent DBs in case of a failed previous test
        remove_test_dbs("test.rocksdb");
        let store = Store::new("test.rocksdb", EngineType::Sled).unwrap();
        test_store_account(store.clone());

        remove_test_dbs("test.rocksdb");
    }

    fn test_store_account(mut store: Store) {
        let address = Address::random();
        let code = Bytes::new();
        let balance = U256::from_dec_str("50").unwrap();
        let nonce = 5;
        let code_hash = types::code_hash(&code);

        let account_info = new_account_info(code.clone(), balance, nonce);
        let _ = store.add_account_info(address, account_info);

        let stored_account_info = store.get_account_info(address).unwrap().unwrap();

        assert_eq!(code_hash, stored_account_info.code_hash);
        assert_eq!(balance, stored_account_info.balance);
        assert_eq!(nonce, stored_account_info.nonce);
    }

    fn new_account_info(code: Bytes, balance: U256, nonce: u64) -> AccountInfo {
        AccountInfo {
            code_hash: types::code_hash(&code),
            balance,
            nonce,
        }
    }

    fn remove_test_dbs(prefix: &str) {
        // Removes all test databases from filesystem
        for entry in fs::read_dir(env::current_dir().unwrap()).unwrap() {
            if entry
                .as_ref()
                .unwrap()
                .file_name()
                .to_str()
                .unwrap()
                .starts_with(prefix)
            {
                fs::remove_dir_all(entry.unwrap().path()).unwrap();
            }
        }
    }
}
