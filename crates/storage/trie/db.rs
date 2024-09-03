use crate::error::StoreError;
use ethereum_rust_core::rlp::{decode::RLPDecode, encode::RLPEncode};
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

pub type NodeRLP = Vec<u8>;
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
        let path = Some(trie_dir.into());
        Ok(TrieDB {
            db: Database::create(path, &tables).map_err(StoreError::LibmdbxError)?,
            next_node_ref: NodeRef::new(0),
        })
    }

    pub fn get_node(&self, node_ref: NodeRef) -> Result<Option<Node>, StoreError> {
        self.read::<Nodes>(node_ref)?
            .map(|rlp| Node::decode(&rlp).map_err(StoreError::RLPDecode))
            .transpose()
    }

    pub fn insert_node(&mut self, node: Node) -> Result<NodeRef, StoreError> {
        let node_ref = self.next_node_ref;
        println!("Insert Node: {} : {}", *node_ref, node.info());
        self.write::<Nodes>(node_ref.into(), node.encode_to_vec())?;
        self.next_node_ref = node_ref.next();
        Ok(node_ref)
    }

    pub fn update_node(&mut self, node_ref: NodeRef, node: Node) -> Result<(), StoreError> {
        println!("Update Node: {} : {}", *node_ref, node.info());
        self.write::<Nodes>(node_ref.into(), node.encode_to_vec())
    }

    /// Updates a node's path only if it was previously empty
    pub fn update_node_path(
        &mut self,
        node_ref: NodeRef,
        new_path: PathRLP,
    ) -> Result<(), StoreError> {
        if let Some(mut node) = self.get_node(node_ref)? {
            node.try_update_path(new_path)?;
            println!("Update Node Path: {} : {}", *node_ref, node.info());
            self.write::<Nodes>(node_ref.into(), node.encode_to_vec())?;
        }
        Ok(())
    }

    /// Returns the removed node if it existed
    pub fn remove_node(&self, node_ref: NodeRef) -> Result<Option<Node>, StoreError> {
        let node = self.get_node(node_ref)?;
        println!(
            "Remove Node: {} : {:?}",
            *node_ref,
            node.as_ref().map(|n| n.info())
        );
        if node.is_some() {
            self.remove::<Nodes>(node_ref)?;
        }
        Ok(node)
    }

    pub fn get_value(&self, path: PathRLP) -> Result<Option<ValueRLP>, StoreError> {
        self.read::<Values>(path)
    }

    pub fn insert_value(&self, path: PathRLP, value: ValueRLP) -> Result<(), StoreError> {
        debug_assert!(!path.is_empty()); // Sanity check
        self.write::<Values>(path, value)
    }

    /// Returns the removed node if it existed
    pub fn remove_value(&self, path: PathRLP) -> Result<Option<ValueRLP>, StoreError> {
        let value = self.get_value(path.clone())?;
        if value.is_some() {
            self.remove::<Values>(path)?;
        }
        Ok(value)
    }

    /// Returns the current value and then overwrites it with the new one
    pub fn replace_value(
        &self,
        path: PathRLP,
        new_value: ValueRLP,
    ) -> Result<Option<ValueRLP>, StoreError> {
        debug_assert!(!path.is_empty()); // Sanity check
        let old_value = self.get_value(path.clone())?;
        self.insert_value(path, new_value)?;
        Ok(old_value)
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
