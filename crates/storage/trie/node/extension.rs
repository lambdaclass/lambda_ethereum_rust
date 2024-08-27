use crate::trie::{nibble::NibbleVec, node_ref::NodeRef};

use crate::trie::hashing::NodeHash;

pub struct ExtensionNode {
    pub hash: NodeHash,
    pub prefix: NibbleVec,
    pub child: NodeRef,
}

impl ExtensionNode {
    pub(crate) fn new(prefix: NibbleVec, child: NodeRef) -> Self {
        Self {
            prefix,
            child,
            hash: Default::default(),
        }
    }
}
