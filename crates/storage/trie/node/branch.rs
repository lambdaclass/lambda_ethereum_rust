use crate::trie::{db::PathRLP, hashing::NodeHash, node_ref::NodeRef};

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

    pub fn update_path(&mut self, new_path: PathRLP) {
        self.path = new_path
    }
}
