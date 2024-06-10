use bytes::BufMut;
use tinyvec::{Array, ArrayVec};

pub enum RLPError {}

trait RLPDecode: Sized {
    fn decode(buf: &mut &[u8]) -> Result<Self, RLPError>;
}
