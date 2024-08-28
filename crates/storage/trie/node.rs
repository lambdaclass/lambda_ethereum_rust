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
    fn get(&self, db: &TrieDB, path: NibbleSlice) -> Result<Option<ValueRLP>, StoreError> {
        match self {
            Node::Branch(n) => n.get(db, path),
            Node::Extension(_) => todo!(),
            Node::Leaf(n) => n.get(db, path),
        }
    }

    fn insert(
        &mut self,
        db: &mut TrieDB,
        path: NibbleSlice,
    ) -> Result<(Node, InsertAction), StoreError> {
        match self {
            Node::Branch(n) => n.insert(db, path),
            Node::Extension(_) => todo!(),
            Node::Leaf(n) => n.insert(db, path),
        }
    }

    fn remove(
        self,
        db: &mut TrieDB,
        path: NibbleSlice,
    ) -> Result<(Option<Node>, Option<ValueRLP>), StoreError> {
        match self {
            Node::Branch(n) => n.remove(db, path),
            Node::Extension(_) => todo!(),
            Node::Leaf(n) => n.remove(db, path),
        }
    }

    fn compute_hash(&self, db: &TrieDB, path_offset: usize) -> Result<NodeHashRef, StoreError> {
        match self {
            Node::Branch(n) => n.compute_hash(db, path_offset),
            Node::Extension(_) => todo!(),
            Node::Leaf(n) => n.compute_hash(db, path_offset),
        }
    }
}
