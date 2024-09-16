mod branch;
mod extension;
mod leaf;

pub use branch::BranchNode;
pub use extension::ExtensionNode;
pub use leaf::LeafNode;

use crate::error::StoreError;

use super::{nibble::NibbleSlice, node_hash::NodeHash, state::TrieState, ValueRLP};

/// A Node in an Ethereum Compatible Patricia Merkle Trie
#[derive(Debug, Clone)]
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
    pub fn get(
        &self,
        state: &TrieState,
        path: NibbleSlice,
    ) -> Result<Option<ValueRLP>, StoreError> {
        match self {
            Node::Branch(n) => n.get(state, path),
            Node::Extension(n) => n.get(state, path),
            Node::Leaf(n) => n.get(path),
        }
    }

    /// Inserts a value into the subtrie originating from this node and returns the new root of the subtrie
    pub fn insert(
        self,
        state: &mut TrieState,
        path: NibbleSlice,
        value: ValueRLP,
    ) -> Result<Node, StoreError> {
        match self {
            Node::Branch(n) => n.insert(state, path, value),
            Node::Extension(n) => n.insert(state, path, value),
            Node::Leaf(n) => n.insert(state, path, value),
        }
    }

    /// Removes a value from the subtrie originating from this node given its path
    /// Returns the new root of the subtrie (if any) and the removed value if it existed in the subtrie
    pub fn remove(
        self,
        state: &mut TrieState,
        path: NibbleSlice,
    ) -> Result<(Option<Node>, Option<ValueRLP>), StoreError> {
        match self {
            Node::Branch(n) => n.remove(state, path),
            Node::Extension(n) => n.remove(state, path),
            Node::Leaf(n) => n.remove(path),
        }
    }

    pub fn insert_self(
        self,
        path_offset: usize,
        state: &mut TrieState,
    ) -> Result<NodeHash, StoreError> {
        match self {
            Node::Branch(n) => n.insert_self(state),
            Node::Extension(n) => n.insert_self(state),
            Node::Leaf(n) => n.insert_self(path_offset, state),
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
