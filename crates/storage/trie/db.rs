use std::collections::HashMap;

use crate::error::StoreError;
use ethereum_rust_core::rlp::{decode::RLPDecode, encode::RLPEncode};
use ethereum_types::H256;
use libmdbx::{
    orm::{table, Database},
    table_info,
};

/// Libmbdx database representing the trie state
/// It contains a table mapping node hashes to rlp encoded nodes
/// All nodes are stored in the DB and no node is ever removed
use super::{node::Node, node_hash::NodeHash};
pub struct TrieState {
    db: Database,
    cache: HashMap<NodeHash, Node>,
}

/// RLP-encoded trie node
pub type NodeRLP = Vec<u8>;
/// RLP-encoded node hash
pub type NodeHashRLP = [u8; 32];

table!(
    /// NodeHash to Node table
    ( Nodes ) NodeHash => NodeRLP
);

impl TrieState {
    /// Opens a DB created by a previous execution or creates a new one if it doesn't exist
    pub fn init(trie_dir: &str) -> Result<TrieState, StoreError> {
        TrieState::open(trie_dir).or_else(|_| TrieState::create(trie_dir))
    }

    /// Creates a new clean DB
    pub fn create(trie_dir: &str) -> Result<TrieState, StoreError> {
        let tables = [table_info!(Nodes)].into_iter().collect();
        let path = Some(trie_dir.into());
        Ok(TrieState {
            db: Database::create(path, &tables).map_err(StoreError::LibmdbxError)?,
            cache: Default::default(),
        })
    }

    /// Opens a DB created by a previous execution
    pub fn open(trie_dir: &str) -> Result<TrieState, StoreError> {
        // Open DB
        let tables = [table_info!(Nodes)].into_iter().collect();
        let db = Database::open(trie_dir, &tables).map_err(StoreError::LibmdbxError)?;
        Ok(TrieState {
            db,
            cache: Default::default(),
        })
    }

    /// Retrieves a node based on its hash
    pub fn get_node(&self, hash: NodeHash) -> Result<Option<Node>, StoreError> {
        if let Some(node) = self.cache.get(&hash) {
            return Ok(Some(node.clone()));
        };
        self.read::<Nodes>(hash)?
            .map(|rlp| Node::decode(&rlp).map_err(StoreError::RLPDecode))
            .transpose()
    }

    /// Inserts a node
    pub fn insert_node(&mut self, node: Node, hash: NodeHash) {
        self.cache.insert(hash, node);
    }

    /// Commits cache changes to DB and clears it
    /// Only writes nodes that follow the root's canonical trie
    pub fn commit(&mut self, root: &NodeHash) -> Result<(), StoreError> {
        self.commit_node(root)?;
        self.cache.clear();
        Ok(())
    }

    // Writes a node and its children into the DB
    fn commit_node(&mut self, node_hash: &NodeHash) -> Result<(), StoreError> {
        let node = self
            .cache
            .remove(node_hash)
            .expect("inconsistent internal tree structure");
        // Commit children (if any)
        match &node {
            Node::Branch(n) => {
                for child in n.choices.iter() {
                    if child.is_valid() {
                        self.commit_node(&child)?;
                    }
                }
            }
            Node::Extension(n) => self.commit_node(&n.child)?,
            Node::Leaf(_) => {}
        }
        // Commit self
        self.write::<Nodes>(node_hash.clone(), node.encode_to_vec())
    }

    /// Helper method to write into a libmdbx table
    fn write<T: libmdbx::orm::Table>(
        &self,
        key: T::Key,
        value: T::Value,
    ) -> Result<(), StoreError> {
        let txn = self
            .db
            .begin_readwrite()
            .map_err(StoreError::LibmdbxError)?;
        txn.upsert::<T>(key, value)
            .map_err(StoreError::LibmdbxError)?;
        txn.commit().map_err(StoreError::LibmdbxError)
    }

    /// Helper method to read from a libmdbx table
    fn read<T: libmdbx::orm::Table>(&self, key: T::Key) -> Result<Option<T::Value>, StoreError> {
        let txn = self.db.begin_read().map_err(StoreError::LibmdbxError)?;
        txn.get::<T>(key).map_err(StoreError::LibmdbxError)
    }

    #[cfg(test)]
    /// Creates a temporary DB, for testing purposes only
    pub fn init_temp() -> Self {
        let tables = [table_info!(Nodes)].into_iter().collect();
        TrieState {
            db: Database::create(None, &tables).expect("Failed to create temp DB"),
            cache: Default::default(),
        }
    }
}
