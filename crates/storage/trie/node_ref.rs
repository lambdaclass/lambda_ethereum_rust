use std::ops::Deref;

use ethereum_rust_core::rlp::{decode::RLPDecode, encode::RLPEncode, error::RLPDecodeError};
use libmdbx::{orm::Decodable, orm::Encodable};

const INVALID_REF: usize = usize::MAX;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct NodeRef(usize);

impl NodeRef {
    pub fn new(value: usize) -> Self {
        assert_ne!(value, INVALID_REF);
        Self(value)
    }

    pub const fn is_valid(&self) -> bool {
        self.0 != INVALID_REF
    }

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
