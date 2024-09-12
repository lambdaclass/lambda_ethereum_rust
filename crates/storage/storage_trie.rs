use std::sync::{Arc, Mutex};

use ethereum_types::Address;

use crate::error::StoreError;
use crate::trie::{Trie, TrieDB};

use crate::Store;

/// The DB-backedn for a storage trie
/// All storage tries read from the same dupsort DB table
/// The trie has an assigned trie in order to write into the correct key
pub struct StorageTrieDB {
    backend: Arc<Mutex<dyn StorageTrieBackend>>,
    address: Address,
}

pub trait StorageTrieBackend {
    fn get(&self, address: Address, key: Vec<u8>) -> Result<Option<Vec<u8>>, StoreError>;
    fn put(&self, address: Address, key: Vec<u8>, value: Vec<u8>) -> Result<(), StoreError>;
}

impl TrieDB for StorageTrieDB {
    fn get(&self, key: Vec<u8>) -> Result<Option<Vec<u8>>, StoreError> {
        self.backend.lock().unwrap().get(self.address, key)
    }

    fn put(&self, key: Vec<u8>, value: Vec<u8>) -> Result<(), StoreError> {
        self.backend.lock().unwrap().put(self.address, key, value)
    }
}

impl Store {
    pub fn storage_trie_db(&self, address: Address) -> Trie<StorageTrieDB> {
       let db = StorageTrieDB {backend: self.engine.clone(), address };
       Trie::new(db).unwrap()
    }
}
