use crate::error::StoreError;
use libmdbx::{
    orm::{table, Database},
    table_info,
};

pub struct LibmdbxTrieDb(Database);

use super::TrieDB;

table!(
    /// NodeHash to Node table
    ( Nodes ) Vec<u8> => Vec<u8>
);

impl LibmdbxTrieDb {
    /// Opens a DB created by a previous execution or creates a new one if it doesn't exist

    pub fn init(trie_dir: &str) -> Result<LibmdbxTrieDb, StoreError> {
        LibmdbxTrieDb::open(trie_dir).or_else(|_| LibmdbxTrieDb::create(trie_dir))
    }

    /// Creates a new clean DB
    pub fn create(trie_dir: &str) -> Result<LibmdbxTrieDb, StoreError> {
        let tables = [table_info!(Nodes)].into_iter().collect();
        let path = Some(trie_dir.into());
        Ok(LibmdbxTrieDb(
            Database::create(path, &tables).map_err(StoreError::LibmdbxError)?,
        ))
    }

    /// Opens a DB created by a previous execution
    pub fn open(trie_dir: &str) -> Result<LibmdbxTrieDb, StoreError> {
        // Open DB
        let tables = [table_info!(Nodes)].into_iter().collect();
        let db = Database::open(trie_dir, &tables).map_err(StoreError::LibmdbxError)?;
        Ok(LibmdbxTrieDb(db))
    }

    #[cfg(test)]
    /// Creates a temporary DB, for testing purposes only
    pub fn init_temp() -> Self {
        use tempdir::TempDir;
        let tables = [table_info!(Nodes)].into_iter().collect();
        LibmdbxTrieDb(Database::create(None, &tables).expect("Failed to create temp DB"))
    }
}

impl TrieDB for LibmdbxTrieDb {
    fn get(&self, key: Vec<u8>) -> Result<Option<Vec<u8>>, StoreError> {
        let txn = self.0.begin_read().map_err(StoreError::LibmdbxError)?;
        txn.get::<Nodes>(key).map_err(StoreError::LibmdbxError)
    }

    fn put(&self, key: Vec<u8>, value: Vec<u8>) -> Result<(), StoreError> {
        let txn = self.0.begin_readwrite().map_err(StoreError::LibmdbxError)?;
        txn.upsert::<Nodes>(key, value)
            .map_err(StoreError::LibmdbxError)?;
        txn.commit().map_err(StoreError::LibmdbxError)
    }
}

#[test]
fn simple_addition() {
    let db = LibmdbxTrieDb::init_temp();
    assert_eq!(db.get("hello".into()).unwrap(), None);
    db.put("hello".into(), "value".into());
    assert_eq!(db.get("hello".into()).unwrap(), Some("value".into()));
}
