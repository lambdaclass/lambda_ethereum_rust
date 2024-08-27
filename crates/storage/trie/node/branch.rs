use super::NodeHash;

pub struct BranchNode {
    pub hash: NodeHash,
    pub choices: [NodeHash; 16],
}
