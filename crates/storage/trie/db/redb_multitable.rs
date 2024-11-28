use std::sync::Arc;

use redb::{Database, MultimapTableDefinition};

use crate::TrieError;

use super::TrieDB;

const STORAGE_TRIE_NODES_TABLE: MultimapTableDefinition<([u8; 32], [u8; 33]), [u8; 32]> =
    MultimapTableDefinition::new("StorageTrieNodes");

/// RedB implementation for the TrieDB trait for a dupsort table with a fixed primary key.
/// For a dupsort table (A, B)[A] -> C, this trie will have a fixed A and just work on B -> C
/// A will be a fixed-size encoded key set by the user (of generic type SK), B will be a fixed-size encoded NodeHash and C will be an encoded Node
pub struct RedBMultiTableTrieDB {
    db: Arc<Database>,
    fixed_key: [u8; 32],
}

impl RedBMultiTableTrieDB {
    pub fn new(db: Arc<Database>, fixed_key: [u8; 32]) -> Self {
        Self { db, fixed_key }
    }
}

impl TrieDB for RedBMultiTableTrieDB {
    fn get(&self, key: Vec<u8>) -> Result<Option<Vec<u8>>, TrieError> {
        let read_txn = self.db.begin_read().unwrap();
        let table = read_txn
            .open_multimap_table(STORAGE_TRIE_NODES_TABLE)
            .unwrap();

        let values = table
            .get((self.fixed_key.clone(), node_hash_to_fixed_size(key)))
            .unwrap()
            .into_iter();

        let mut ret = vec![];
        for value in values {
            ret.push(value.unwrap().value().to_vec());
        }

        let ret_flattened = ret.concat();

        if ret.is_empty() {
            Ok(None)
        } else {
            Ok(Some(ret_flattened))
        }
    }

    fn put(&self, key: Vec<u8>, value: Vec<u8>) -> Result<(), TrieError> {
        let write_txn = self.db.begin_write().unwrap();
        {
            let mut value_fixed: [u8; 32] = [0; 32];
            value_fixed.copy_from_slice(&value);
            let mut table = write_txn
                .open_multimap_table(STORAGE_TRIE_NODES_TABLE)
                .unwrap();
            table
                .insert(
                    (self.fixed_key.clone(), node_hash_to_fixed_size(key)),
                    value_fixed,
                )
                .unwrap();
        }
        write_txn.commit().unwrap();

        Ok(())
    }
}

// In order to use NodeHash as key in a dupsort table we must encode it into a fixed size type
fn node_hash_to_fixed_size(node_hash: Vec<u8>) -> [u8; 33] {
    // keep original len so we can re-construct it later
    let original_len = node_hash.len();
    // original len will always be lower or equal to 32 bytes
    debug_assert!(original_len <= 32);
    // Pad the node_hash with zeros to make it fixed_size (in case of inline)
    let mut node_hash = node_hash;
    node_hash.resize(32, 0);
    // Encode the node as [original_len, node_hash...]
    std::array::from_fn(|i| match i {
        0 => original_len as u8,
        n => node_hash[n - 1],
    })
}
