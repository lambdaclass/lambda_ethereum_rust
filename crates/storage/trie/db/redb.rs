use std::sync::Arc;

use redb::{Database, Error, ReadableTable, TableDefinition};

use super::TrieDB;

const TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("Trie");

pub struct RedBTrie {
    db: Arc<Database>,
}

impl RedBTrie {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }
}

impl TrieDB for RedBTrie {
    fn get(&self, key: Vec<u8>) -> Result<Option<Vec<u8>>, crate::TrieError> {
        let read_txn = self.db.begin_read().unwrap();
        let table = read_txn.open_table(TABLE).unwrap();
        let result = match table.get(&*key).unwrap() {
            Some(value) => Some(value.value().to_vec()),
            None => None,
        };

        Ok(result)
    }

    fn put(&self, key: Vec<u8>, value: Vec<u8>) -> Result<(), crate::TrieError> {
        let write_txn = self.db.begin_write().unwrap();
        {
            let mut table = write_txn.open_table(TABLE).unwrap();
            table.insert(&*key, &*value).unwrap();
        }
        write_txn.commit().unwrap();

        Ok(())
    }
}
