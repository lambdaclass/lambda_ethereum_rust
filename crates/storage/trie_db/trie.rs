use sha3::Keccak256;

use crate::error::StoreError;

use super::{node::NodeHash, nodes_db::NodesDB};

//pub type WorldStateTrie = PatriciaMerkleTree<Vec<u8>, Vec<u8>, Keccak256>;

pub struct TrieDB {
    /// Root node hash.
    root: NodeHash,
    /// Contains all the nodes.
    nodes: NodesDB,
    /// Stores the actual nodes' hashed paths and values.
    // values: Database, // TODO -> next task, will now store values in leaf for simplicity
    hash: (bool, u64),
}

impl TrieDB {
    pub fn new(trie_dir: &str) -> Result<Self, StoreError> {
        Ok(Self {
            root: NodeHash::default(),
            nodes: NodesDB::init(trie_dir)?,
            // values: init_values_db(trie_dir)?,
            hash: (false, 0),
        })
    }

    /// Insert a value into the tree.
    pub fn insert(&mut self, path: bool, value: bool) {
        // Mark hash as dirty
        self.hash.0 = false;
    }
}

// fn init_values_db(trie_dir: &str) -> Result<Database, StoreError> {
//     let tables = [].into_iter().collect();
//     let path = [trie_dir, "/values"].concat().try_into().ok();
//     Database::create(path, &tables).map_err(StoreError::LibmdbxError)
// }
