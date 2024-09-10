mod branch;
mod extension;
mod leaf;

pub use branch::BranchNode;
use ethereum_types::H256;
pub use extension::ExtensionNode;
pub use leaf::LeafNode;

use crate::error::StoreError;

use super::{
    db::TrieDB, dumb_hash::DumbNodeHash, hashing::NodeHashRef, nibble::NibbleSlice,
    node_ref::NodeRef, ValueRLP,
};

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

    /// Computes the node's hash given the offset in the path traversed before reaching this node
    pub fn compute_hash(&self, db: &TrieDB, path_offset: usize) -> Result<NodeHashRef, StoreError> {
        dbg!("EVIL");
        match self {
            Node::Branch(n) => n.compute_hash(db, path_offset),
            Node::Extension(n) => n.compute_hash(db, path_offset),
            Node::Leaf(n) => n.compute_hash(path_offset),
        }
    }

    pub fn insert_self(
        self,
        path_offset: usize,
        db: &mut TrieDB,
    ) -> Result<DumbNodeHash, StoreError> {
        let hash = self.dumb_hash(db, path_offset);
        /// Hash is working propperly
        /// NEXT STEPS:
        /// Remove NodeRef & store by hash instead
        /// REMEMBER: Store small nodes as inline (or ban them), maybe we should encode DumbNodeHash
        db.insert_node(self, hash.clone())?;
        Ok(hash)
    }

    pub fn dumb_hash(&self, db: &TrieDB, path_offset: usize) -> DumbNodeHash {
        match self {
            Node::Branch(n) => n.dumb_hash(db, path_offset),
            Node::Extension(n) => n.dumb_hash(db, path_offset),
            Node::Leaf(n) => n.dumb_hash(path_offset),
        }
    }
}

// TODO: remove
impl Node {
    pub fn info(&self) -> String {
        match self {
            Node::Branch(n) => {
                let choices = n
                    .choices
                    .iter()
                    .filter(|nr| nr.is_valid())
                    .collect::<Vec<_>>();
                format!(
                    "Node::Branch(choices: {choices:?}, path: {:?}, value: {:?})",
                    n.path, n.value
                )
            }
            Node::Extension(n) => {
                format!("Node::Extension(child: {}, prefix {:?}", n.child, n.prefix)
            }
            Node::Leaf(n) => format!("Node::Leaf(path: {:?}, value: {:?})", n.path, n.value),
        }
    }
}
