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
use super::{dumb_hash::DumbNodeHash, node::Node, node_ref::NodeRef};
pub struct TrieDB {
    db: Database,
}

/// RLP-encoded trie node
pub type NodeRLP = Vec<u8>;
/// RLP-encoded node hash
pub type NodeHashRLP = [u8; 32];

table!(
    /// NodeHash to Node table
    ( Nodes ) DumbNodeHash => NodeRLP
);

impl TrieDB {
    /// Opens a DB created by a previous execution or creates a new one if it doesn't exist
    pub fn init(trie_dir: &str) -> Result<TrieDB, StoreError> {
        TrieDB::open(trie_dir).or_else(|_| TrieDB::create(trie_dir))
    }

    /// Creates a new clean DB
    pub fn create(trie_dir: &str) -> Result<TrieDB, StoreError> {
        let tables = [table_info!(Nodes)].into_iter().collect();
        let path = Some(trie_dir.into());
        Ok(TrieDB {
            db: Database::create(path, &tables).map_err(StoreError::LibmdbxError)?,
        })
    }

    /// Opens a DB created by a previous execution
    /// Also returns root node reference if available
    pub fn open(trie_dir: &str) -> Result<TrieDB, StoreError> {
        // Open DB
        let tables = [table_info!(Nodes)].into_iter().collect();
        let db = Database::open(trie_dir, &tables).map_err(StoreError::LibmdbxError)?;
        Ok(TrieDB { db })
    }

    /// Retrieves a node based on its reference
    pub fn get_node(&self, hash: DumbNodeHash) -> Result<Option<Node>, StoreError> {
        self.read::<Nodes>(hash)?
            .map(|rlp| Node::decode(&rlp).map_err(StoreError::RLPDecode))
            .transpose()
    }

    /// Inserts a node and returns its reference
    pub fn insert_node(&mut self, node: Node, hash: DumbNodeHash) -> Result<(), StoreError> {
        self.write::<Nodes>(hash, node.encode_to_vec())
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
        TrieDB {
            db: Database::create(None, &tables).expect("Failed to create temp DB"),
        }
    }
}
