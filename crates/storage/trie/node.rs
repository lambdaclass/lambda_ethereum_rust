mod branch;
mod extension;
mod leaf;

pub use branch::BranchNode;
use ethereum_types::H256;
pub use extension::ExtensionNode;
pub use leaf::LeafNode;

use crate::error::StoreError;

use super::{db::TrieDB, nibble::NibbleSlice, node_hash::NodeHash, node_ref::NodeRef, ValueRLP};

/// A Node in an Ethereum Compatible Patricia Merkle Trie
#[derive(Debug)]
pub enum Node {
    Branch(BranchNode),
    Extension(ExtensionNode),
    Leaf(LeafNode),
}

impl From<BranchNode> for Node {
    fn from(val: BranchNode) -> Self {
        Node::Branch(val)
    }
}

impl From<ExtensionNode> for Node {
    fn from(val: ExtensionNode) -> Self {
        Node::Extension(val)
    }
}

impl From<LeafNode> for Node {
    fn from(val: LeafNode) -> Self {
        Node::Leaf(val)
    }
}

impl Node {
    /// Retrieves a value from the subtrie originating from this node given its path
    pub fn get(&self, db: &TrieDB, path: NibbleSlice) -> Result<Option<ValueRLP>, StoreError> {
        match self {
            Node::Branch(n) => n.get(db, path),
            Node::Extension(n) => n.get(db, path),
            Node::Leaf(n) => n.get(path),
        }
    }

    /// Inserts a value into the subtrie originating from this node and returns the new root of the subtrie
    pub fn insert(
        self,
        db: &mut TrieDB,
        path: NibbleSlice,
        value: ValueRLP,
    ) -> Result<Node, StoreError> {
        match self {
            Node::Branch(n) => n.insert(db, path, value),
            Node::Extension(n) => n.insert(db, path, value),
            Node::Leaf(n) => n.insert(db, path, value),
        }
    }

    /// Removes a value from the subtrie originating from this node given its path
    /// Returns the new root of the subtrie (if any) and the removed value if it existed in the subtrie
    pub fn remove(
        self,
        db: &mut TrieDB,
        path: NibbleSlice,
    ) -> Result<(Option<Node>, Option<ValueRLP>), StoreError> {
        match self {
            Node::Branch(n) => n.remove(db, path),
            Node::Extension(n) => n.remove(db, path),
            Node::Leaf(n) => n.remove(path),
        }
    }

    pub fn insert_self(self, path_offset: usize, db: &mut TrieDB) -> Result<NodeHash, StoreError> {
        match self {
            Node::Branch(n) => n.insert_self(db),
            Node::Extension(n) => n.insert_self(db),
            Node::Leaf(n) => n.insert_self(path_offset, db),
        }
    }

    pub fn compute_hash(&self, path_offset: usize) -> NodeHash {
        match self {
            Node::Branch(n) => n.compute_hash(),
            Node::Extension(n) => n.compute_hash(),
            Node::Leaf(n) => n.compute_hash(path_offset),
        }
    }
}
