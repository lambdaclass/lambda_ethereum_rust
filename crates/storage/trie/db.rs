use crate::{error::StoreError, rlp::Rlp};
use libmdbx::{
    orm::{table, Database},
    table_info,
};

use super::{node::Node, node_ref::NodeRef};
pub struct TrieDB(Database);

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
        Ok(TrieDB(
            Database::create(path, &tables).map_err(StoreError::LibmdbxError)?,
        ))
    }

    pub fn get_node(&self, node_hash: NodeRef) -> Result<Option<Node>, StoreError> {
        Ok(self.read::<Nodes>(node_hash.into())?.map(|n| n.to()))
    }

    pub fn insert_node(&self, node_hash: NodeRef, node: Node) -> Result<(), StoreError> {
        self.write::<Nodes>(node_hash.into(), node.into())
    }

    pub fn remove_node(&self, node_hash: NodeRef) -> Result<(), StoreError> {
        self.remove::<Nodes>(node_hash.into())
    }

    /// Returns the removed node if it existed
    pub fn try_remove_node(&self, node_hash: NodeRef) -> Result<Option<Node>, StoreError> {
        let node = self.get_node(node_hash)?;
        if node.is_some() {
            self.remove_node(node_hash)?;
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
        let txn = self.0.begin_readwrite().map_err(StoreError::LibmdbxError)?;
        txn.upsert::<T>(key, value)
            .map_err(StoreError::LibmdbxError)?;
        txn.commit().map_err(StoreError::LibmdbxError)
    }

    // Helper method to read from a libmdx table
    fn read<T: libmdbx::orm::Table>(&self, key: T::Key) -> Result<Option<T::Value>, StoreError> {
        let txn = self.0.begin_read().map_err(StoreError::LibmdbxError)?;
        txn.get::<T>(key).map_err(StoreError::LibmdbxError)
    }

    // Helper method to remove an entry from a libmdx table
    fn remove<T: libmdbx::orm::Table>(&self, key: T::Key) -> Result<(), StoreError> {
        let txn = self.0.begin_readwrite().map_err(StoreError::LibmdbxError)?;
        txn.delete::<T>(key, None)
            .map_err(StoreError::LibmdbxError)?;
        txn.commit().map_err(StoreError::LibmdbxError)
    }
}
