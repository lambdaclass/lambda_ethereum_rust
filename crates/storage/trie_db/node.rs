use ethereum_types::H256;
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
    pub value: Vec<u8>, // TODO: Store reference to value in other DB instead
}
