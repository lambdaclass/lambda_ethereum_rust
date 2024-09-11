use std::ops::Deref;

use ethereum_rust_core::rlp::{decode::RLPDecode, encode::RLPEncode, error::RLPDecodeError};
use libmdbx::{orm::Decodable, orm::Encodable};

/// Default value for a NodeReference, indicating that the referenced node does not yet exist
const INVALID_REF: usize = usize::MAX;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(transparent)]
/// Struct representing a reference to a trie node
/// Used to store and fetch nodes from the DB
/// Each reference is unique
pub struct NodeRef(usize);

impl NodeRef {
    /// Creates a new reference from an integer value
    pub fn new(value: usize) -> Self {
        assert_ne!(value, INVALID_REF);
        Self(value)
    }

    /// Returns true if the reference is a valid
    pub const fn is_valid(&self) -> bool {
        self.0 != INVALID_REF
    }

    /// Returns the next node reference based on the current one
    /// This ensures that each reference is unique as long as they are created based on this method
    // TODO: check if we should use a bigger type for node references
    pub fn next(&self) -> Self {
        let next = self.0.saturating_add(1);
        if self.is_valid() {
            Self(next)
        } else {
            panic!("Trie node limit reached")
        }
    }
}

impl Default for NodeRef {
    fn default() -> Self {
        Self(INVALID_REF)
    }
}

impl Deref for NodeRef {
    type Target = usize;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Encodable for NodeRef {
    type Encoded = [u8; 8];

    fn encode(self) -> Self::Encoded {
        self.0.to_be_bytes()
    }
}

impl Decodable for NodeRef {
    fn decode(data_val: &[u8]) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        Ok(Self(usize::from_be_bytes(data_val.try_into()?)))
    }
}

impl RLPEncode for NodeRef {
    fn encode(&self, buf: &mut dyn bytes::BufMut) {
        self.0.encode(buf)
    }
}

impl RLPDecode for NodeRef {
    fn decode_unfinished(rlp: &[u8]) -> Result<(Self, &[u8]), RLPDecodeError> {
        RLPDecode::decode_unfinished(rlp).map(|(v, rem)| (NodeRef(v), rem))
    }
}
