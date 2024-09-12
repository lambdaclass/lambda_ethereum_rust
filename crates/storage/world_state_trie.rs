use std::sync::{Arc, Mutex};

use crate::error::StoreError;
use crate::trie::{Trie, TrieDB};

use crate::Store;

pub struct WorldStateTrieDB {
    db: Arc<Mutex<dyn TrieDB>>,

}

impl TrieDB for WorldStateTrieDB {
    fn get(&self, key: Vec<u8>) -> Result<Option<Vec<u8>>, StoreError> {
        self.db.lock().unwrap().get(key)
    }

    fn put(&self, key: Vec<u8>, value: Vec<u8>) -> Result<(), StoreError> {
        self.db.lock().unwrap().put(key, value)
    }
}

impl Store {
    pub fn world_state_trie(&self) -> Trie<WorldStateTrieDB> {
       let db = WorldStateTrieDB {db: self.engine.clone() };
       Trie::new(db).unwrap()
    }
}


