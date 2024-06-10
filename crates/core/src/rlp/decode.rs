use bytes::Bytes;
use tinyvec::{Array, ArrayVec};

use super::error::RLPDecodeError;

pub trait RLPDecode: Sized {
    fn decode(rlp: &[u8]) -> Result<Self, RLPDecodeError>;
}

// integer decoding impls
impl RLPDecode for u8 {
    fn decode(buf: &[u8]) -> Result<Self, RLPDecodeError> {
        let bytes = Bytes::copy_from_slice(buf);

        Ok(0)
    }
}
