use std::{cmp::min, default};

use ethereum_rust_core::rlp::{decode::RLPDecode, encode::RLPEncode};
use ethereum_types::H256;
use libmdbx::orm::{Decodable, Encodable};
use sha3::{Digest, Keccak256};

use super::{
    hashing::Output,
    nibble::{NibbleSlice, NibbleVec},
};

#[derive(Default)]
pub struct HashBuilder {
    hash: Output,
    len: usize,
    hasher: Keccak256,
    no_inline: bool,
}

/// TODO: check wether making this `Copy` can make the code less verbose at a reasonable performance cost
#[derive(Debug, Clone, PartialEq)]
pub enum DumbNodeHash {
    Hashed(H256),
    Inline(Vec<u8>),
}

impl HashBuilder {
    pub fn new() -> Self {
        Self {
            ..Default::default()
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
        let mut length = self.len;
        let mut hash = self.hash;

        let mut current_pos = 0;
        while current_pos < value.len() {
            let copy_len = min(32 - length, value.len() - current_pos);

            let target_slice = &mut hash[length..length + copy_len];
            let source_slice = &value[current_pos..current_pos + copy_len];
            target_slice.copy_from_slice(source_slice);

            current_pos += copy_len;
            length += copy_len;

            if length == 32 {
                self.push_hash_update(&hash);
                length = 0;
            }
        }
        self.hash = hash;
        self.len = length;
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

    pub fn write_bytes(&mut self, value: &[u8]) {
        if value.len() == 1 && value[0] < 128 {
            self.write_raw(&[value[0]]);
        } else {
            self.write_len(0x80, 0xB7, value.len());
            self.write_raw(value);
        }
    }

    fn push_hash_update(&mut self, data: &[u8]) {
        self.no_inline = true;
        self.hasher.update(data)
    }

    pub fn finalize(mut self) -> DumbNodeHash {
        if self.no_inline {
            let hash = self.hash;
            self.push_hash_update(&hash[..self.len]);
            DumbNodeHash::Hashed(H256::from_slice(self.hasher.finalize().as_slice()))
        } else {
            DumbNodeHash::Inline(self.hash[..self.len].to_vec())
        }
    }
}

const fn compute_byte_usage(value: usize) -> usize {
    let bits_used = usize::BITS as usize - value.leading_zeros() as usize;
    (bits_used.saturating_sub(1) >> 3) + 1
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

impl<'a> AsRef<[u8]> for DumbNodeHash {
    fn as_ref(&self) -> &[u8] {
        match self {
            DumbNodeHash::Inline(x) => x.as_ref(),
            DumbNodeHash::Hashed(x) => x.as_bytes(),
        }
    }
}

impl DumbNodeHash {
    /// Returns the finalized hash
    /// NOTE: This will hash smaller nodes, only use to get the final root hash, not for intermediate node hashes
    pub fn finalize(self) -> H256 {
        match self {
            DumbNodeHash::Inline(x) => {
                H256::from_slice(Keccak256::new().chain_update(&*x).finalize().as_slice())
            }
            DumbNodeHash::Hashed(x) => x,
        }
    }

    /// Returns true if the hash is valid
    /// The hash will only be considered invalid if it is empty
    /// Aka if it has a default value instead of being a product of hash computation
    pub fn is_valid(&self) -> bool {
        match self {
            DumbNodeHash::Inline(v) if v.is_empty() => false,
            _ => true,
        }
    }

    /// Const version of `Default` trait impl
    pub const fn const_default() -> Self {
        Self::Inline(vec![])
    }
}

impl From<Vec<u8>> for DumbNodeHash {
    fn from(value: Vec<u8>) -> Self {
        match value.len() {
            32 => DumbNodeHash::Hashed(H256::from_slice(&value)),
            _ => DumbNodeHash::Inline(value),
        }
    }
}

impl From<H256> for DumbNodeHash {
    fn from(value: H256) -> Self {
        DumbNodeHash::Hashed(value)
    }
}

impl Into<Vec<u8>> for DumbNodeHash {
    fn into(self) -> Vec<u8> {
        match self {
            DumbNodeHash::Hashed(x) => x.0.to_vec(),
            DumbNodeHash::Inline(x) => x,
        }
    }
}

impl Into<Vec<u8>> for &DumbNodeHash {
    fn into(self) -> Vec<u8> {
        match self {
            DumbNodeHash::Hashed(x) => x.0.to_vec(),
            DumbNodeHash::Inline(x) => x.clone(),
        }
    }
}

impl Encodable for DumbNodeHash {
    type Encoded = Vec<u8>;

    fn encode(self) -> Self::Encoded {
        self.into()
    }
}

impl Decodable for DumbNodeHash {
    fn decode(b: &[u8]) -> anyhow::Result<Self> {
        Ok(match b.len() {
            32 => DumbNodeHash::Hashed(H256::from_slice(b)),
            _ => DumbNodeHash::Inline(b.into()),
        })
    }
}

impl Default for DumbNodeHash {
    fn default() -> Self {
        DumbNodeHash::Inline(Vec::new())
    }
}

// Encoded as Vec<u8>
impl RLPEncode for DumbNodeHash {
    fn encode(&self, buf: &mut dyn bytes::BufMut) {
        RLPEncode::encode(&Into::<Vec<u8>>::into(self), buf)
    }
}

impl RLPDecode for DumbNodeHash {
    fn decode_unfinished(
        rlp: &[u8],
    ) -> Result<(Self, &[u8]), ethereum_rust_core::rlp::error::RLPDecodeError> {
        let (mut hash, mut rest): (Vec<u8>, &[u8]);
        (hash, rest) = RLPDecode::decode_unfinished(rlp)?;
        let hash = DumbNodeHash::from(hash);
        Ok((hash, rest))
    }
}
