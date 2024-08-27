use libmdbx::orm::Database;
use sha3::Keccak256;

use crate::error::StoreError;


//pub type WorldStateTrie = PatriciaMerkleTree<Vec<u8>, Vec<u8>, Keccak256>;

pub struct TrieDB {
    /// Reference to the root node.
    pub root_ref: bool, // Node ref
    /// Contains all the nodes.
    pub nodes: Database,
    /// Stores the actual nodes' hashed paths and values.
    pub values: Database,
}

impl TrieDB {
    pub fn new(trie_dir: &str) -> Result<Self, StoreError> {
        Ok(Self { root_ref: false, nodes: init_values_db(trie_dir)?, values: init_values_db(trie_dir)? })
    }
}

fn init_nodes_db(trie_dir: &str) -> Result<Database, StoreError> {
    let tables = [
    ]
    .into_iter()
    .collect();
    let path = [trie_dir, "/nodes"].concat().try_into().ok();
    Database::create(path, &tables).map_err(StoreError::LibmdbxError)
}

fn init_values_db(trie_dir: &str) -> Result<Database, StoreError> {
    let tables = [
    ]
    .into_iter()
    .collect();
    let path = [trie_dir, "/values"].concat().try_into().ok();
    Database::create(path, &tables).map_err(StoreError::LibmdbxError)
}
