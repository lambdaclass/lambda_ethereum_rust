mod branch;
mod extension;
mod leaf;

pub use branch::BranchNode;
use ethereum_types::H256;
pub use extension::ExtensionNode;
pub use leaf::LeafNode;

use super::{db::PathRLP, node_ref::NodeRef};
pub type NodeHash = H256;

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
