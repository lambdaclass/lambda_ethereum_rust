use sha3::{Digest, Keccak256};

use super::{
    db::{PathRLP, TrieDB, ValueRLP},
    hashing::{NodeHashRef, Output},
    nibble::NibbleSlice,
    node::{InsertAction, LeafNode, Node},
    node_ref::NodeRef,
};
use crate::error::StoreError;

pub struct Trie {
    /// Root node ref.
    root_ref: NodeRef,
    /// Contains all the nodes and all the node's values
    db: TrieDB,
    hash: (bool, Output),
}

impl Trie {
    pub fn new(trie_dir: &str) -> Result<Self, StoreError> {
        Ok(Self {
            root_ref: NodeRef::default(),
            db: TrieDB::init(trie_dir)?,
            hash: (false, Default::default()),
        })
    }

    /// Retrieve a value from the tree given its path.
    /// TODO: Make inputs T: RLPEncode (we will ignore generics for now)
    pub fn get(&self, path: &PathRLP) -> Result<Option<ValueRLP>, StoreError> {
        if !self.root_ref.is_valid() {
            return Ok(None);
        }
        let root_node = self
            .db
            .get_node(self.root_ref)?
            .expect("inconsistent internal tree structure");

        root_node.get(&self.db, NibbleSlice::new(&path))
    }

    /// Insert a value into the tree.
    /// TODO: Make inputs T: RLPEncode (we will ignore generics for now)
    pub fn insert(
        &mut self,
        path: PathRLP,
        value: ValueRLP,
    ) -> Result<Option<ValueRLP>, StoreError> {
        // Mark hash as dirty
        self.hash.0 = false;
        if let Some(root_node) = self.db.remove_node(self.root_ref)? {
            // If the tree is not empty, call the root node's insertion logic
            let (root_node, insert_action) =
                root_node.insert(&mut self.db, NibbleSlice::new(&path))?;
            self.root_ref = self.db.insert_node(root_node)?;

            match insert_action.quantize_self(self.root_ref) {
                InsertAction::Insert(node_ref) => {
                    self.db.insert_value(path.clone(), value)?;
                    let node = match self
                        .db
                        .get_node(node_ref)? // [WARNING] get_mut
                        .expect("inconsistent internal tree structure")
                    {
                        Node::Leaf(mut leaf_node) => {
                            leaf_node.update_path(path);
                            leaf_node.into()
                        }
                        Node::Branch(mut branch_node) => {
                            branch_node.update_path(path);
                            branch_node.into()
                        }
                        _ => panic!("inconsistent internal tree structure"),
                    };
                    self.db.update_node(node_ref, node)?;

                    Ok(None)
                }
                InsertAction::Replace(path) => self.db.replace_value(path, value),
                _ => unreachable!(),
            }
        } else {
            // If the tree is empty, just add a leaf.
            self.db.insert_value(path.clone(), value)?;
            self.root_ref = self.db.insert_node(LeafNode::new(path).into())?;
            Ok(None)
        }
    }

    /// Remove a value from the tree.
    pub fn remove(&mut self, path: PathRLP) -> Result<Option<ValueRLP>, StoreError> {
        if !self.root_ref.is_valid() {
            return Ok(None);
        }

        let root_node = self
            .db
            .remove_node(self.root_ref)?
            .expect("inconsistent internal tree structure");
        let (root_node, old_value) = root_node.remove(&mut self.db, NibbleSlice::new(&path))?;
        self.root_ref = match root_node {
            Some(root_node) => self.db.insert_node(root_node)?,
            None => Default::default(),
        };

        Ok(old_value)
    }

    /// Return the root hash of the tree (or recompute if needed).
    pub fn compute_hash(&mut self) -> Result<Output, StoreError> {
        if !self.hash.0 {
            if self.root_ref.is_valid() {
                let root_node = self
                    .db
                    .get_node(self.root_ref)?
                    .expect("inconsistent internal tree structure");

                match root_node.compute_hash(&self.db, 0)? {
                    NodeHashRef::Inline(x) => {
                        Keccak256::new()
                            .chain_update(&*x)
                            .finalize_into(&mut self.hash.1);
                    }
                    NodeHashRef::Hashed(x) => self.hash.1.copy_from_slice(&x.clone()),
                };
            } else {
                Keccak256::new()
                    .chain_update([0x80])
                    .finalize_into(&mut self.hash.1);
            }
            self.hash.0 = true;
        }
        Ok(self.hash.1)
    }
}
