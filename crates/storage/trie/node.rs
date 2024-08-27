mod branch;
mod extension;
mod leaf;

pub use branch::BranchNode;
use ethereum_types::H256;
pub use extension::ExtensionNode;
pub use leaf::LeafNode;

use super::db::PathRLP;
pub type NodeHash = H256;

pub enum Node {
    Branch(BranchNode),
    Extension(ExtensionNode),
    Leaf(LeafNode),
}

pub enum InsertAction {
    Replace(PathRLP),
}
