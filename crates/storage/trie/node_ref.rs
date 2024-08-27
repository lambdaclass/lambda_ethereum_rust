// TODO: check where we should place this code
use std::ops::Deref;

use libmdbx::orm::Encodable;

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
