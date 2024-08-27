use crate::trie::{db::PathRLP, node_ref::NodeRef};

use super::NodeHash;

pub struct BranchNode {
    pub hash: NodeHash,
    pub choices: [NodeRef; 16],
    pub path: PathRLP,
}

impl BranchNode {
    pub fn new(choices: [NodeRef; 16]) -> Self {
        Self {
            choices,
            hash: Default::default(),
            path: Default::default(),
        }
    }
}
