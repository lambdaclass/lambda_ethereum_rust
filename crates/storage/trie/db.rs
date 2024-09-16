pub mod in_memory;
pub mod libmdbx;

use crate::error::StoreError;

use super::{InMemoryTrieDB, Libmdbx};

pub trait TrieDB {
    fn get(&self, key: Vec<u8>) -> Result<Option<Vec<u8>>, StoreError>;
    fn put(&self, key: Vec<u8>, value: Vec<u8>) -> Result<(), StoreError>;
}
