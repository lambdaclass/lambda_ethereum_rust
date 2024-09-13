use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use crate::error::StoreError;

/// InMemory implementation for the TrieDB trait, with get and put operations.
pub struct InMemoryTrieDB {
    map: Arc<Mutex<HashMap<Vec<u8>, Vec<u8>>>>,
}

use super::TrieDB;

impl InMemoryTrieDB {
    pub fn new(map: Arc<Mutex<HashMap<Vec<u8>, Vec<u8>>>>) -> Self {
        Self { map }
    }
}

impl TrieDB for InMemoryTrieDB {
    fn get(&self, key: Vec<u8>) -> Result<Option<Vec<u8>>, StoreError> {
        Ok(self.map.lock().unwrap().get(&key).cloned())
    }

    fn put(&self, key: Vec<u8>, value: Vec<u8>) -> Result<(), StoreError> {
        self.map.lock().unwrap().insert(key, value);
        Ok(())
    }
}
