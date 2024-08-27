use ethereum_types::H256;
use leaf::LeafNode;
pub mod leaf;

use super::{db::PathRLP, nibble::NibbleVec};
pub type NodeHash = H256;

pub enum Node {
    Branch(BranchNode),
    Extension(ExtensionNode),
    Leaf(LeafNode),
}

pub struct BranchNode {
    pub hash: NodeHash,
    pub choices: [NodeHash; 16],
}

pub struct ExtensionNode {
    pub hash: NodeHash,
    pub prefix: NibbleVec,
    pub child: NodeHash,
}

pub enum InsertAction {
    Replace(PathRLP),
}
