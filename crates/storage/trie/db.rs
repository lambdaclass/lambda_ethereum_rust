pub mod libmdbx;

/// RLP-encoded trie node
pub type NodeRLP = Vec<u8>;
/// RLP-encoded node hash
pub type NodeHashRLP = [u8; 32];

use crate::error::StoreError;
pub trait TrieDB {
    fn get(&self, key: NodeHashRLP) -> Result<Option<NodeRLP>, StoreError>;
    fn put(&self, key: NodeHashRLP, value: NodeRLP) -> Result<(), StoreError>;
}
