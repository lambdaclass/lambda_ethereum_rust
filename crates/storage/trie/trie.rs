use super::{db::TrieDB, node::NodeHash, node_ref::NodeRef};
use crate::error::StoreError;

//pub type WorldStateTrie = PatriciaMerkleTree<Vec<u8>, Vec<u8>, Keccak256>;

pub struct Trie {
    /// Root node hash.
    root: NodeRef,
    /// Contains all the nodes and all the node's values
    db: TrieDB,
    hash: (bool, u64),
}

impl Trie {
    pub fn new(trie_dir: &str) -> Result<Self, StoreError> {
        Ok(Self {
            root: NodeRef::default(),
            db: TrieDB::init(trie_dir)?,
            hash: (false, 0),
        })
    }

    /// Insert a value into the tree.
    /// TODO: Make inputs T: RLPEncode (we will ignore generics for now)
    pub fn insert(&mut self, k: Vec<u8>, v: Vec<u8>) -> Result<(), StoreError> {
        // Mark hash as dirty
        self.hash.0 = false;
        if let Some(root_node) = self.db.try_remove_node(self.root)? {
            // If the tree is not empty, call the root node's insertion logic
        }
        Ok(())
    }
}
