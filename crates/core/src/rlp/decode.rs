use tinyvec::{Array, ArrayVec};
use bytes::{BufMut};


pub enum RLPError {

}

trait RLPDecode: Sized {
    fn decode(buf: &mut &[u8]) -> Result<Self, RLPError>;
}
