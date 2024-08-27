use crate::{error::StoreError, rlp::Rlp};
use libmdbx::{
    orm::{table, Database},
    table_info,
};

use super::{node::Node, node_ref::NodeRef};
pub struct TrieDB {
    db: Database,
    // TODO: This replaces the use of Slab in the reference impl
    // Check if we can find a better way to solve the problem of tracking nodes without using hashes
    next_node_ref: NodeRef,
}

pub type NodeRLP = Rlp<Node>;

pub type PathRLP = Vec<u8>;
pub type ValueRLP = Vec<u8>;

table!(
    /// NodeRef to Node table
    ( Nodes ) NodeRef => NodeRLP
);

table!(
    /// Path to Value table
    (Values) PathRLP => ValueRLP
);

impl TrieDB {
    pub fn init(trie_dir: &str) -> Result<TrieDB, StoreError> {
        let tables = [table_info!(Nodes), table_info!(Values)]
            .into_iter()
            .collect();
        let path = trie_dir.try_into().ok();
        Ok(TrieDB {
            db: Database::create(path, &tables).map_err(StoreError::LibmdbxError)?,
            next_node_ref: NodeRef::default(),
        })
    }

    pub fn get_node(&self, node_ref: NodeRef) -> Result<Option<Node>, StoreError> {
        Ok(self.read::<Nodes>(node_ref.into())?.map(|n| n.to()))
    }

    pub fn insert_node(&mut self, node: Node) -> Result<NodeRef, StoreError> {
        let node_ref = self.next_node_ref;
        self.write::<Nodes>(node_ref.into(), node.into())?;
        self.next_node_ref = node_ref.next();
        Ok(node_ref)
    }

    pub fn remove_node(&self, node_ref: NodeRef) -> Result<(), StoreError> {
        self.remove::<Nodes>(node_ref.into())
    }

    /// Returns the removed node if it existed
    pub fn try_remove_node(&self, node_ref: NodeRef) -> Result<Option<Node>, StoreError> {
        let node = self.get_node(node_ref)?;
        if node.is_some() {
            self.remove_node(node_ref)?;
        }
        Ok(node)
    }

    pub fn get_value(&self, path: PathRLP) -> Result<Option<ValueRLP>, StoreError> {
        self.read::<Values>(path)
    }

    pub fn insert_value(&self, path: PathRLP, value: ValueRLP) -> Result<(), StoreError> {
        self.write::<Values>(path, value)
    }

    pub fn remove_value(&self, path: PathRLP) -> Result<(), StoreError> {
        self.remove::<Values>(path)
    }

    // Helper method to write into a libmdx table
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

    // Helper method to read from a libmdx table
    fn read<T: libmdbx::orm::Table>(&self, key: T::Key) -> Result<Option<T::Value>, StoreError> {
        let txn = self.db.begin_read().map_err(StoreError::LibmdbxError)?;
        txn.get::<T>(key).map_err(StoreError::LibmdbxError)
    }

    // Helper method to remove an entry from a libmdx table
    fn remove<T: libmdbx::orm::Table>(&self, key: T::Key) -> Result<(), StoreError> {
        let txn = self
            .db
            .begin_readwrite()
            .map_err(StoreError::LibmdbxError)?;
        txn.delete::<T>(key, None)
            .map_err(StoreError::LibmdbxError)?;
        txn.commit().map_err(StoreError::LibmdbxError)
    }
}
