mod branch;
mod extension;
mod leaf;

pub use branch::BranchNode;
pub use extension::ExtensionNode;
pub use leaf::LeafNode;

use crate::error::StoreError;

use super::{db::TrieDB, hashing::NodeHashRef, nibble::NibbleSlice, ValueRLP};

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
    pub fn get(&self, db: &TrieDB, path: NibbleSlice) -> Result<Option<ValueRLP>, StoreError> {
        match self {
            Node::Branch(n) => n.get(db, path),
            Node::Extension(n) => n.get(db, path),
            Node::Leaf(n) => n.get(path),
        }
    }

    /// Inserts a value into the subtree that has this node as its root and returns the new root of the subtree
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

    pub fn compute_hash(&self, db: &TrieDB, path_offset: usize) -> Result<NodeHashRef, StoreError> {
        match self {
            Node::Branch(n) => n.compute_hash(db, path_offset),
            Node::Extension(n) => n.compute_hash(db, path_offset),
            Node::Leaf(n) => n.compute_hash(path_offset),
        }
    }
}

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
                format!("Node::Extension(child: {}, prefix {:?}", *n.child, n.prefix)
            }
            Node::Leaf(n) => format!("Node::Leaf(path: {:?}, value: {:?})", n.path, n.value),
        }
    }
}
