use crate::error::StoreError;
use ethereum_rust_core::rlp::{decode::RLPDecode, encode::RLPEncode};
use ethereum_types::H256;
use libmdbx::{
    orm::{table, Database},
    table_info,
};

/// Libmbdx database representing trie state
/// It contains two tables, one mapping node references to rlp encoded nodes, and another one mapping node hashes to node references
/// All nodes are stored in the DB and no node is ever removed
/// Only node hahses for root nodes computed with `Trie::compute_hash` are stored
/// Contains the next node reference to be used when storing a new node
use super::{node::Node, node_ref::NodeRef};
pub struct TrieDB {
    db: Database,
    next_node_ref: NodeRef,
}

/// RLP-encoded trie node
pub type NodeRLP = Vec<u8>;
/// RLP-encoded node hash
pub type NodeHashRLP = [u8; 32];

table!(
    /// NodeRef to Node table
    ( Nodes ) NodeRef => NodeRLP
);

table!(
    /// NodeRef to Node table
    ( NodesByHash ) NodeHashRLP => NodeRLP
);

table!(
    /// NodeHash to NodeRef table
    /// Stores root nodes which's hashes have been computed by `compute_hash`
    ( RootNodes ) NodeHashRLP => NodeRef
);

impl TrieDB {
    /// Opens a DB created by a previous execution or creates a new one if it doesn't exist
    pub fn init(trie_dir: &str) -> Result<(TrieDB, Option<NodeRef>), StoreError> {
        TrieDB::open(trie_dir).or_else(|_| TrieDB::create(trie_dir).map(|db| (db, None)))
    }

    /// Creates a new clean DB
    pub fn create(trie_dir: &str) -> Result<TrieDB, StoreError> {
        let tables = [
            table_info!(Nodes),
            table_info!(RootNodes),
            table_info!(NodesByHash),
        ]
        .into_iter()
        .collect();
        let path = Some(trie_dir.into());
        Ok(TrieDB {
            db: Database::create(path, &tables).map_err(StoreError::LibmdbxError)?,
            next_node_ref: NodeRef::new(0),
        })
    }

    /// Opens a DB created by a previous execution
    /// Also returns root node reference if available
    pub fn open(trie_dir: &str) -> Result<(TrieDB, Option<NodeRef>), StoreError> {
        // Open DB
        let tables = [
            table_info!(Nodes),
            table_info!(RootNodes),
            table_info!(NodesByHash),
        ]
        .into_iter()
        .collect();
        let db = Database::open(trie_dir, &tables).map_err(StoreError::LibmdbxError)?;
        // Set the next node reference based on the last stored node
        let last_node_ref = last_node_ref(&db)?;
        let next_node_ref = last_node_ref.map(|nr| nr.next()).unwrap_or_default();
        Ok((TrieDB { db, next_node_ref }, last_node_ref))
    }

    /// Retrieves a node based on its reference
    pub fn get_node(&self, node_ref: NodeRef) -> Result<Option<Node>, StoreError> {
        self.read::<Nodes>(node_ref)?
            .map(|rlp| Node::decode(&rlp).map_err(StoreError::RLPDecode))
            .transpose()
    }

    /// Inserts a node and returns its reference
    pub fn insert_node(&mut self, node: Node, hash: H256) -> Result<NodeRef, StoreError> {
        let node_ref = self.next_node_ref;
        self.write::<Nodes>(node_ref, node.encode_to_vec())?;
        self.write::<NodesByHash>(hash.0, node.encode_to_vec())?;
        self.next_node_ref = node_ref.next();
        Ok(node_ref)
    }

    /// Retrieves a node reference based on the hash of the referenced node
    /// Will only work if the hash was computed using `Trie::compute_hash`
    pub fn get_root_ref(&self, hash: H256) -> Result<Option<NodeRef>, StoreError> {
        self.read::<RootNodes>(hash.0)
    }

    /// Inserts a node reference by its node's hash
    pub fn insert_root_ref(&mut self, hash: H256, node_ref: NodeRef) -> Result<(), StoreError> {
        self.write::<RootNodes>(hash.0, node_ref)
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
        let tables = [
            table_info!(Nodes),
            table_info!(RootNodes),
            table_info!(NodesByHash),
        ]
        .into_iter()
        .collect();
        TrieDB {
            db: Database::create(None, &tables).expect("Failed to create temp DB"),
            next_node_ref: NodeRef::new(0),
        }
    }
}

/// Returns the reference to the last node stored in the database
/// Used when opening a previous execution's database
fn last_node_ref(db: &Database) -> Result<Option<NodeRef>, StoreError> {
    let txn = db.begin_read().map_err(StoreError::LibmdbxError)?;
    let mut cursor = txn.cursor::<Nodes>().map_err(StoreError::LibmdbxError)?;
    Ok(cursor
        .last()
        .map_err(StoreError::LibmdbxError)?
        .map(|(node_ref, _)| node_ref))
}
