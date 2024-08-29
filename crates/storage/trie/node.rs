mod branch;
mod extension;
mod leaf;

pub use branch::BranchNode;
pub use extension::ExtensionNode;
pub use leaf::LeafNode;

use crate::error::StoreError;

use super::{
    db::{PathRLP, TrieDB, ValueRLP},
    hashing::NodeHashRef,
    nibble::NibbleSlice,
    node_ref::NodeRef,
};

#[derive(Debug)]
pub enum Node {
    Branch(BranchNode),
    Extension(ExtensionNode),
    Leaf(LeafNode),
}

/// Returned by .insert() to update the values' storage.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum InsertAction {
    /// An insertion is required. The argument points to a node.
    Insert(NodeRef),
    /// A replacement is required. The argument points to a value.
    Replace(PathRLP),

    /// Special insert where its node_ref is not known.
    InsertSelf,
}

impl InsertAction {
    /// Replace `Self::InsertSelf` with `Self::Insert(node_ref)`.
    pub fn quantize_self(&self, node_ref: NodeRef) -> Self {
        match self {
            Self::InsertSelf => Self::Insert(node_ref),
            _ => self.clone(),
        }
    }
}

impl Into<Node> for BranchNode {
    fn into(self) -> Node {
        Node::Branch(self)
    }
}

impl Into<Node> for ExtensionNode {
    fn into(self) -> Node {
        Node::Extension(self)
    }
}

impl Into<Node> for LeafNode {
    fn into(self) -> Node {
        Node::Leaf(self)
    }
}

impl Node {
    pub fn get(&self, db: &TrieDB, path: NibbleSlice) -> Result<Option<ValueRLP>, StoreError> {
        match self {
            Node::Branch(n) => n.get(db, path),
            Node::Extension(n) => n.get(db, path),
            Node::Leaf(n) => n.get(db, path),
        }
    }

    pub fn insert(
        self,
        db: &mut TrieDB,
        path: NibbleSlice,
    ) -> Result<(Node, InsertAction), StoreError> {
        match self {
            Node::Branch(n) => n.insert(db, path),
            Node::Extension(n) => n.insert(db, path),
            Node::Leaf(n) => n.insert(db, path),
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
            Node::Leaf(n) => n.remove(db, path),
        }
    }

    pub fn compute_hash(&self, db: &TrieDB, path_offset: usize) -> Result<NodeHashRef, StoreError> {
        match self {
            Node::Branch(n) => n.compute_hash(db, path_offset),
            Node::Extension(n) => n.compute_hash(db, path_offset),
            Node::Leaf(n) => n.compute_hash(db, path_offset),
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
                format!("Node::Branch(choices: {choices:?}, path: {:?}", n.path)
            }
            Node::Extension(n) => {
                format!("Node::Extension(child: {}, prefix {:?}", *n.child, n.prefix)
            }
            Node::Leaf(n) => format!("Node::Leaf(path: {:?}", n.path),
        }
    }
}
