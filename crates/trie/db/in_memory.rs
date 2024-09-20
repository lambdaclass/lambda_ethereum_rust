use super::TrieDB;
use crate::error::TrieError;
use std::{
    cell::RefCell,
    collections::HashMap,
    sync::{Arc, Mutex},
};

/// InMemory implementation for the TrieDB trait, with get and put operations.
pub struct InMemoryTrieDB {
    inner: Arc<Mutex<HashMap<Vec<u8>, Vec<u8>>>>,
}

impl InMemoryTrieDB {
    pub fn new(map: Arc<Mutex<HashMap<Vec<u8>, Vec<u8>>>>) -> Self {
        Self { inner: map }
    }
}

impl TrieDB for InMemoryTrieDB {
    fn get(&self, key: Vec<u8>) -> Result<Option<Vec<u8>>, TrieError> {
        Ok(self.inner.lock().unwrap().get(&key).cloned())
    }

    fn put(&self, key: Vec<u8>, value: Vec<u8>) -> Result<(), TrieError> {
        self.inner.lock().unwrap().insert(key, value);
        Ok(())
    }
}

#[derive(Default)]
pub struct SimplifiedInMemoryTrieDB {
    inner: RefCell<HashMap<Vec<u8>, Vec<u8>>>,
}

impl SimplifiedInMemoryTrieDB {
    pub fn new(map: RefCell<HashMap<Vec<u8>, Vec<u8>>>) -> Self {
        Self { inner: map }
    }
}

impl TrieDB for SimplifiedInMemoryTrieDB {
    fn get(&self, key: Vec<u8>) -> Result<Option<Vec<u8>>, TrieError> {
        Ok(self.inner.borrow().get(&key).cloned())
    }

    fn put(&self, key: Vec<u8>, value: Vec<u8>) -> Result<(), TrieError> {
        self.inner.borrow_mut().insert(key, value);
        Ok(())
    }
}
