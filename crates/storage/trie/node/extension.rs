use crate::trie::nibble::NibbleVec;

use super::NodeHash;

pub struct ExtensionNode {
    pub hash: NodeHash,
    pub prefix: NibbleVec,
    pub child: NodeHash,
}
