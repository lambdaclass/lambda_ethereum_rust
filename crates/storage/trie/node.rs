use ethereum_types::H256;

use super::db::PathRLP;
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
    pub prefix: bool, // Nibble vec
    pub child: NodeHash,
}

pub struct LeafNode {
    pub hash: NodeHash,
    pub path: PathRLP,
}
