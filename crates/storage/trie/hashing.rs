// TODO: Simplify this module
use super::nibble::{NibbleSlice, NibbleVec};
use digest::Digest;
use ethereum_rust_core::rlp::{
    decode::RLPDecode,
    encode::RLPEncode,
    error::RLPDecodeError,
    structs::{Decoder, Encoder},
};
use sha3::Keccak256;
use std::{borrow::Cow, cmp::min, mem::size_of};

pub type Output = digest::Output<Keccak256>;
#[derive(Debug)]
pub struct DelimitedHash(pub Output, pub usize);

impl AsRef<[u8]> for DelimitedHash {
    fn as_ref(&self) -> &[u8] {
        &self.0[..self.1]
    }
}

impl Default for DelimitedHash {
    fn default() -> Self {
        Self(Default::default(), 0)
    }
}

impl From<NodeHash> for DelimitedHash {
    fn from(value: NodeHash) -> Self {
        let (data, len) = value.into_inner();
        Self(data, len)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodeHash {
    length: usize,
    hash: Output,
}

impl NodeHash {
    pub fn mark_as_dirty(&mut self) {
        self.length = 0;
    }

    pub fn extract_ref(&self) -> Option<NodeHashRef> {
        match self.length {
            0 => None,
            32 => Some(NodeHashRef::Hashed(Cow::Borrowed(&self.hash))),
            l => Some(NodeHashRef::Inline(Cow::Borrowed(&self.hash[..l]))),
        }
    }

    pub fn into_inner(self) -> (Output, usize) {
        (self.hash, self.length)
    }
}

impl Default for NodeHash {
    fn default() -> Self {
        Self {
            length: 0,
            hash: Default::default(),
        }
    }
}

#[derive(Debug)]
pub enum NodeHashRef<'a> {
    Inline(Cow<'a, [u8]>),
    Hashed(Cow<'a, Output>),
}

impl<'a> AsRef<[u8]> for NodeHashRef<'a> {
    fn as_ref(&self) -> &[u8] {
        match self {
            NodeHashRef::Inline(x) => x,
            NodeHashRef::Hashed(x) => x,
        }
    }
}

pub struct NodeHasher {
    parent: NodeHash,
    hasher: Option<Keccak256>,
}

impl<'a> NodeHasher {
    pub fn new(parent: &NodeHash) -> Self {
        Self {
            parent: NodeHash {
                length: 0,
                hash: parent.hash,
            },
            hasher: None,
        }
    }

    pub fn finalize(mut self) -> NodeHashRef<'a> {
        match self.hasher {
            Some(_) => {
                {
                    let mut hash = self.parent.hash;
                    self.push_hash_update(&hash[..self.parent.length]);
                    self.hasher.take().unwrap().finalize_into(&mut hash);
                }
                self.parent.length = 32;
                NodeHashRef::Hashed(Cow::Owned(self.parent.hash))
            }
            None => {
                NodeHashRef::Inline(Cow::Owned(self.parent.hash[..self.parent.length].to_vec()))
            }
        }
    }

    pub const fn path_len(value_len: usize) -> usize {
        Self::bytes_len((value_len >> 1) + 1, 0)
    }

    pub const fn bytes_len(value_len: usize, first_value: u8) -> usize {
        match value_len {
            1 if first_value < 128 => 1,
            l if l < 56 => l + 1,
            l => l + compute_byte_usage(l) + 1,
        }
    }

    pub fn write_path_vec(&mut self, value: &NibbleVec, kind: PathKind) {
        let mut flag = kind.into_flag();

        // TODO: Do not use iterators.
        let nibble_count = value.len();
        let nibble_iter = if nibble_count & 0x01 != 0 {
            let mut iter = value.iter();
            flag |= 0x10;
            flag |= iter.next().unwrap() as u8;
            iter
        } else {
            value.iter()
        };

        let i2 = nibble_iter.clone().skip(1).step_by(2);
        if nibble_count > 1 {
            self.write_len(0x80, 0xB7, (nibble_count >> 1) + 1);
        }
        self.write_raw(&[flag]);
        for (a, b) in nibble_iter.step_by(2).zip(i2) {
            self.write_raw(&[((a as u8) << 4) | (b as u8)]);
        }
    }

    pub fn write_path_slice(&mut self, value: &NibbleSlice, kind: PathKind) {
        let mut flag = kind.into_flag();

        // TODO: Do not use iterators.
        let nibble_count = value.clone().count();
        let nibble_iter = if nibble_count & 0x01 != 0 {
            let mut iter = value.clone();
            flag |= 0x10;
            flag |= iter.next().unwrap() as u8;
            iter
        } else {
            value.clone()
        };

        let i2 = nibble_iter.clone().skip(1).step_by(2);
        if nibble_count > 1 {
            self.write_len(0x80, 0xB7, (nibble_count >> 1) + 1);
        }
        self.write_raw(&[flag]);
        for (a, b) in nibble_iter.step_by(2).zip(i2) {
            self.write_raw(&[((a as u8) << 4) | (b as u8)]);
        }
    }

    pub fn write_bytes(&mut self, value: &[u8]) {
        if value.len() == 1 && value[0] < 128 {
            self.write_raw(&[value[0]]);
        } else {
            self.write_len(0x80, 0xB7, value.len());
            self.write_raw(value);
        }
    }

    pub fn write_list_header(&mut self, children_len: usize) {
        self.write_len(0xC0, 0xF7, children_len);
    }

    fn write_len(&mut self, short_base: u8, long_base: u8, value: usize) {
        match value {
            l if l < 56 => self.write_raw(&[short_base + l as u8]),
            l => {
                let l_len = compute_byte_usage(l);
                self.write_raw(&[long_base + l_len as u8]);
                self.write_raw(&l.to_be_bytes()[size_of::<usize>() - l_len..]);
            }
        }
    }

    pub fn write_raw(&mut self, value: &[u8]) {
        let mut length = self.parent.length;
        let mut hash_ref = self.parent.hash;

        let mut current_pos = 0;
        while current_pos < value.len() {
            let copy_len = min(32 - length, value.len() - current_pos);

            let target_slice = &mut hash_ref[length..length + copy_len];
            let source_slice = &value[current_pos..current_pos + copy_len];
            target_slice.copy_from_slice(source_slice);

            current_pos += copy_len;
            length += copy_len;

            if length == 32 {
                self.push_hash_update(&hash_ref);
                length = 0;
            }
        }

        self.parent.length = length;
    }

    fn push_hash_update(&mut self, data: &[u8]) {
        let hasher = self.hasher.get_or_insert_with(Keccak256::new);
        hasher.update(data);
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PathKind {
    Extension,
    Leaf,
}

impl PathKind {
    const fn into_flag(self) -> u8 {
        match self {
            PathKind::Extension => 0x00,
            PathKind::Leaf => 0x20,
        }
    }
}

const fn compute_byte_usage(value: usize) -> usize {
    let bits_used = usize::BITS as usize - value.leading_zeros() as usize;
    (bits_used.saturating_sub(1) >> 3) + 1
}

impl RLPEncode for NodeHash {
    fn encode(&self, buf: &mut dyn bytes::BufMut) {
        let length = self.length;
        let hash = self.hash.to_vec();
        Encoder::new(buf)
            .encode_field(&length)
            .encode_field(&hash)
            .finish();
    }
}

impl RLPDecode for NodeHash {
    fn decode_unfinished(rlp: &[u8]) -> Result<(Self, &[u8]), RLPDecodeError> {
        let decoder = Decoder::new(rlp)?;
        let (length, decoder) = decoder.decode_field("length")?;
        let (hash, decoder) = decoder.decode_field::<Vec<u8>>("hash_ref")?;
        let hash: &Output = (hash.as_slice()).into();
        let hash = hash.clone();
        Ok((Self { length, hash }, decoder.finish()?))
    }
}
