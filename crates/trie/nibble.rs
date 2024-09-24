use ethereum_rust_rlp::{
    decode::RLPDecode,
    encode::RLPEncode,
    error::RLPDecodeError,
    structs::{Decoder, Encoder},
};
use smallvec::SmallVec;

/// Struct representing a half-byte
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Nibble {
    V0 = 0,
    V1 = 1,
    V2 = 2,
    V3 = 3,
    V4 = 4,
    V5 = 5,
    V6 = 6,
    V7 = 7,
    V8 = 8,
    V9 = 9,
    V10 = 10,
    V11 = 11,
    V12 = 12,
    V13 = 13,
    V14 = 14,
    V15 = 15,
}

impl TryFrom<u8> for Nibble {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Ok(match value {
            0x00 => Self::V0,
            0x01 => Self::V1,
            0x02 => Self::V2,
            0x03 => Self::V3,
            0x04 => Self::V4,
            0x05 => Self::V5,
            0x06 => Self::V6,
            0x07 => Self::V7,
            0x08 => Self::V8,
            0x09 => Self::V9,
            0x0A => Self::V10,
            0x0B => Self::V11,
            0x0C => Self::V12,
            0x0D => Self::V13,
            0x0E => Self::V14,
            0x0F => Self::V15,
            x => return Err(x),
        })
    }
}

impl From<Nibble> for u8 {
    fn from(value: Nibble) -> Self {
        value as u8
    }
}

impl From<Nibble> for usize {
    fn from(value: Nibble) -> Self {
        value as usize
    }
}

/// A slice of half-bytes (represented as a slice of bytes) with an offset
/// When used as a path, the offset indicates how much of the path has been traversed
#[derive(Clone, Debug)]
pub struct NibbleSlice<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> NibbleSlice<'a> {
    /// Creates a new NibbleSlice form a slice of bytes
    pub const fn new(inner: &'a [u8]) -> Self {
        Self {
            data: inner,
            offset: 0,
        }
    }

    /// Returns the amount of half-bytes in the slice, taking into account the current offset
    pub const fn len(&self) -> usize {
        2 * self.data.len() - self.offset
    }

    pub const fn offset(&self) -> usize {
        self.offset
    }

    /// Returns the full original path, ignoring the current offset
    pub fn data(&self) -> super::PathRLP {
        self.data.to_vec()
    }

    /// Splits the slice into a vec at the given offset
    // TODO: Improve doc (help wanted)
    pub fn split_to_vec(&self, offset: usize) -> NibbleVec {
        NibbleVec {
            data: SmallVec::from_slice(
                &self.data[self.offset >> 1..(self.offset + offset + 1) >> 1],
            ),
            first_is_half: self.offset % 2 != 0,
            last_is_half: (self.offset + offset) % 2 != 0,
        }
    }

    /// Advance the current offset by a given amount
    pub fn offset_add(&mut self, delta: usize) {
        self.offset += delta;
    }

    /// If `prefix` is a prefix of self (with the correct nibble alignment), move the offset after
    /// the prefix and return true, otherwise return false.
    ///
    /// Unaligned comparations are bugs (panic).
    pub fn skip_prefix(&mut self, prefix: &NibbleVec) -> bool {
        // Check alignment.
        assert_eq!(
            (self.offset % 2 != 0),
            prefix.first_is_half,
            "inconsistent internal tree structure",
        );

        // Prefix can only be a prefix if self.len() >= prefix.len()
        if self.data.len() < prefix.data.len() {
            return false;
        }

        // Prepare slices.
        let mut prfx_slice = prefix.data.as_slice();
        let mut self_slice = &self.data[self.offset >> 1..(self.offset >> 1) + prfx_slice.len()];

        // If the prefix is empty, it's always a prefix.
        if prfx_slice.is_empty()
            || (prfx_slice.len() == 1 && prefix.first_is_half && prefix.last_is_half)
        {
            return true;
        }

        // Check the first nibble when unaligned.
        if prefix.first_is_half {
            if (prfx_slice[0] & 0x0F) != (self_slice[0] & 0x0F) {
                return false;
            }

            self_slice = &self_slice[1..];
            prfx_slice = &prfx_slice[1..];
        }

        // Check the last nibble when unaligned.
        if prefix.last_is_half {
            let i = self_slice.len() - 1;
            if (prfx_slice[i] & 0xF0) != (self_slice[i] & 0xF0) {
                return false;
            }

            self_slice = &self_slice[..i];
            prfx_slice = &prfx_slice[..i];
        }

        // Check the rest of the values.
        if self_slice != prfx_slice {
            return false;
        }

        // Advance self.
        self.offset = self.offset + (prefix.data.len() << 1)
            - prefix.first_is_half as usize
            - prefix.last_is_half as usize;

        true
    }

    /// Compare the rest of the data in self with the data in `other` after the offset in self.
    pub fn cmp_rest(&self, other: &[u8]) -> bool {
        // Prepare slices.
        let mut othr_slice = &other[self.offset >> 1..];
        let mut self_slice = &self.data[self.offset >> 1..];

        if self.offset % 2 != 0 {
            if (othr_slice[0] & 0x0F) != (self_slice[0] & 0x0F) {
                return false;
            }

            othr_slice = &othr_slice[1..];
            self_slice = &self_slice[1..];
        }

        self_slice == othr_slice
    }

    /// Compares self to a NibbleVec and returns the shared nibble count (amount of nibbles that are equal, starting from self's offset)
    pub fn count_prefix_vec(&mut self, other: &NibbleVec) -> usize {
        if other.data.is_empty() {
            return 0;
        }

        // Check alignment and length.
        assert_eq!(self.offset % 2 != 0, other.first_is_half);

        // Compare first nibble (if not byte-aligned).
        let mut eq_count = 0;
        if other.first_is_half {
            if self.next().map(u8::from) == Some(other.data[0] & 0x0F) {
                eq_count += 1;
            } else {
                return 0;
            }
        }

        // Compare middle bytes.
        let mut byte_nibble_count = 0;
        let mut check_last_half = true;
        for (a, b) in self.data[(self.offset + 1) >> 1..].iter().zip(
            &other.data
                [other.first_is_half as usize..other.data.len() - (other.last_is_half as usize)],
        ) {
            if a == b {
                byte_nibble_count += 2;
            } else if (a & 0xF0) == (b & 0xF0) {
                byte_nibble_count += 1;
                check_last_half = false;
                break;
            } else {
                check_last_half = false;
                break;
            }
        }
        eq_count += byte_nibble_count;
        self.offset += byte_nibble_count;

        // Compare last nibble (if not byte-aligned).
        if check_last_half
            && other.last_is_half
            && self.clone().next().map(u8::from) == other.data.last().map(|x| x >> 4)
        {
            eq_count += 1;
        }

        eq_count
    }

    /// Compares self to another NibbleSlice and returns the shared nibble count (amount of nibbles that are equal, starting the common offset)
    /// Both slices should have the same offset
    pub fn count_prefix_slice(&self, other: &NibbleSlice) -> usize {
        // Check offset (and therefore alignment implicitly).
        assert_eq!(self.offset, other.offset);

        // Check first nibble (if not byte-aligned).
        let mut eq_count = 0;
        if self.offset % 2 != 0 {
            if (self.data[self.offset >> 1] & 0x0F) == (other.data[self.offset >> 1] & 0x0F) {
                eq_count += 1;
            } else {
                return 0;
            }
        }

        // Compare the rest.
        for (a, b) in self.data[(self.offset + 1) >> 1..]
            .iter()
            .zip(&other.data[(self.offset + 1) >> 1..])
        {
            if a == b {
                eq_count += 2;
            } else if (a & 0xF0) == (b & 0xF0) {
                eq_count += 1;
                break;
            } else {
                break;
            }
        }

        eq_count
    }
}

impl<'a> AsRef<[u8]> for NibbleSlice<'a> {
    fn as_ref(&self) -> &'a [u8] {
        self.data
    }
}

impl<'a> Iterator for NibbleSlice<'a> {
    type Item = Nibble;

    fn next(&mut self) -> Option<Self::Item> {
        self.data.get(self.offset >> 1).map(|byte| {
            let byte = if self.offset % 2 != 0 {
                byte & 0x0F
            } else {
                byte >> 4
            };

            self.offset += 1;
            Nibble::try_from(byte).unwrap_or_else(|_| unreachable!())
        })
    }
}

/// A vector of nibbles, represented as a byte array, with flags indicating wether the first or last bytes represent only half a byte
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct NibbleVec {
    data: SmallVec<[u8; 111]>,

    first_is_half: bool,
    last_is_half: bool,
}

impl NibbleVec {
    #[cfg(test)]
    /// Creates a new, empty NibbleVec
    pub fn new() -> Self {
        NibbleVec {
            data: Default::default(),
            first_is_half: false,
            last_is_half: false,
        }
    }

    #[cfg(test)]
    /// Create a NibbleVec from an iterator of nibbles
    pub fn from_nibbles(
        data_iter: impl Iterator<Item = Nibble>,
        starts_with_half_byte: bool,
    ) -> Self {
        let mut last_is_half = false;
        let mut data = SmallVec::new();

        let mut data_iter = data_iter.peekable();
        if starts_with_half_byte && data_iter.peek().is_some() {
            data.push(0);
            last_is_half = true;
        }

        for nibble in data_iter {
            if !last_is_half {
                data.push((nibble as u8) << 4);
            } else {
                *data.last_mut().unwrap() |= nibble as u8;
            }

            last_is_half = !last_is_half;
        }

        Self {
            data,
            first_is_half: starts_with_half_byte,
            last_is_half,
        }
    }

    /// Create a NibbleVec from a single nibble
    pub fn from_single(nibble: Nibble, is_right_half: bool) -> Self {
        Self {
            data: SmallVec::from_elem(
                if is_right_half {
                    nibble as u8
                } else {
                    (nibble as u8) << 4
                },
                1,
            ),
            first_is_half: is_right_half,
            last_is_half: !is_right_half,
        }
    }

    /// Returns true if the NibbleVec is empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Returns the amount of nibbles (half-bytes) in the NibbleVec
    pub fn len(&self) -> usize {
        2 * self.data.len() - self.first_is_half as usize - self.last_is_half as usize
    }

    /// Returns an iterator over the NibbleVec
    pub const fn iter(&self) -> NibbleVecIter {
        NibbleVecIter {
            inner: self,
            pos: self.first_is_half as usize,
        }
    }

    /// Splits the NibbleVec at a given offset, returning the nibble at the specified offset
    ///  and the NibbleVecs containing the nibbles before and after the speficic nibble
    pub fn split_extract_at(self, index: usize) -> (NibbleVec, Nibble, NibbleVec) {
        let offset = (index + 1 + self.first_is_half as usize) >> 1;
        let mut left_vec = NibbleVec {
            data: SmallVec::from_slice(&self.data[..offset]),
            first_is_half: self.first_is_half,
            last_is_half: (index + self.first_is_half as usize) % 2 != 0,
        };
        left_vec.normalize();

        let offset = index + self.first_is_half as usize;
        // Check out of bounds for last half-byte.
        assert!(
            ((offset + self.last_is_half as usize) >> 1) < self.data.len(),
            "out of bounds"
        );
        let value = if offset % 2 != 0 {
            self.data[offset >> 1] & 0x0F
        } else {
            self.data[offset >> 1] >> 4
        };
        let value = Nibble::try_from(value).unwrap_or_else(|_| unreachable!());

        let offset = (index + 1 + self.first_is_half as usize) >> 1;
        let mut right_vec = NibbleVec {
            data: if offset >= self.data.len() {
                SmallVec::new()
            } else {
                SmallVec::from_slice(&self.data[offset..])
            },
            first_is_half: (index + self.first_is_half as usize) % 2 == 0,
            last_is_half: self.last_is_half,
        };
        right_vec.normalize();

        (left_vec, value, right_vec)
    }

    /// Inserts a nibble at the start of the NibbleVec
    pub fn prepend(&mut self, nibble: Nibble) {
        if self.first_is_half {
            self.data[0] = (self.data[0] & 0x0F) | ((nibble as u8) << 4);
        } else {
            self.data.insert(0, nibble as u8);
        }
        self.first_is_half = !self.first_is_half;
    }

    /// Extends self with another NibbleVec
    pub fn extend(&mut self, other: &Self) {
        // Ensure alignment.
        assert_eq!(self.last_is_half, other.first_is_half);

        // Copy half-byte (if misaligned).
        if self.last_is_half {
            let last = self.data.last_mut().unwrap();
            *last = (*last & 0xF0) | (other.data[0] & 0x0F);
        }
        self.last_is_half = other.last_is_half;

        // Extend.
        self.data.extend(if other.first_is_half {
            other.data[1..].iter().copied()
        } else {
            other.data.iter().copied()
        });
    }

    /// Resets self to default (full clear) if there are no nibbles represented by self
    pub(crate) fn normalize(&mut self) {
        if self.data.is_empty() || (self.data.len() == 1 && self.first_is_half && self.last_is_half)
        {
            self.data.clear();
            self.first_is_half = false;
            self.last_is_half = false;
        }
    }
}

#[derive(Clone)]
/// An iterator of nibbles (half-bytes)
pub struct NibbleVecIter<'a> {
    inner: &'a NibbleVec,
    pos: usize,
}

impl<'a> Iterator for NibbleVecIter<'a> {
    type Item = Nibble;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.data.get(self.pos >> 1).and_then(|byte| {
            if (self.pos >> 1) + 1 == self.inner.data.len()
                && self.pos % 2 == 1
                && self.inner.last_is_half
            {
                return None;
            }

            let byte = if self.pos % 2 != 0 {
                byte & 0x0F
            } else {
                byte >> 4
            };

            self.pos += 1;
            Nibble::try_from(byte).map_or_else(|_| unreachable!(), Some)
        })
    }
}

impl RLPEncode for NibbleVec {
    fn encode(&self, buf: &mut dyn bytes::BufMut) {
        Encoder::new(buf)
            .encode_field(&self.data.clone().into_vec())
            .encode_field(&self.first_is_half)
            .encode_field(&self.last_is_half)
            .finish()
    }
}

impl RLPDecode for NibbleVec {
    fn decode_unfinished(rlp: &[u8]) -> Result<(Self, &[u8]), RLPDecodeError> {
        let decoder = Decoder::new(rlp)?;
        let (data, decoder) = decoder.decode_field("data")?;
        let (first_is_half, decoder) = decoder.decode_field("first_is_half")?;
        let (last_is_half, decoder) = decoder.decode_field("last_is_half")?;
        let data = SmallVec::<[u8; 111]>::from_vec(data);
        Ok((
            Self {
                data,
                first_is_half,
                last_is_half,
            },
            decoder.finish()?,
        ))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn nibble_try_from_u8() {
        assert_eq!(Nibble::try_from(0x00u8), Ok(Nibble::V0));
        assert_eq!(Nibble::try_from(0x01u8), Ok(Nibble::V1));
        assert_eq!(Nibble::try_from(0x02u8), Ok(Nibble::V2));
        assert_eq!(Nibble::try_from(0x03u8), Ok(Nibble::V3));
        assert_eq!(Nibble::try_from(0x04u8), Ok(Nibble::V4));
        assert_eq!(Nibble::try_from(0x05u8), Ok(Nibble::V5));
        assert_eq!(Nibble::try_from(0x06u8), Ok(Nibble::V6));
        assert_eq!(Nibble::try_from(0x07u8), Ok(Nibble::V7));
        assert_eq!(Nibble::try_from(0x08u8), Ok(Nibble::V8));
        assert_eq!(Nibble::try_from(0x09u8), Ok(Nibble::V9));
        assert_eq!(Nibble::try_from(0x0Au8), Ok(Nibble::V10));
        assert_eq!(Nibble::try_from(0x0Bu8), Ok(Nibble::V11));
        assert_eq!(Nibble::try_from(0x0Cu8), Ok(Nibble::V12));
        assert_eq!(Nibble::try_from(0x0Du8), Ok(Nibble::V13));
        assert_eq!(Nibble::try_from(0x0Eu8), Ok(Nibble::V14));
        assert_eq!(Nibble::try_from(0x0Fu8), Ok(Nibble::V15));
    }

    #[test]
    fn nibble_into_u8() {
        assert_eq!(u8::from(Nibble::V0), 0x00);
        assert_eq!(u8::from(Nibble::V1), 0x01);
        assert_eq!(u8::from(Nibble::V2), 0x02);
        assert_eq!(u8::from(Nibble::V3), 0x03);
        assert_eq!(u8::from(Nibble::V4), 0x04);
        assert_eq!(u8::from(Nibble::V5), 0x05);
        assert_eq!(u8::from(Nibble::V6), 0x06);
        assert_eq!(u8::from(Nibble::V7), 0x07);
        assert_eq!(u8::from(Nibble::V8), 0x08);
        assert_eq!(u8::from(Nibble::V9), 0x09);
        assert_eq!(u8::from(Nibble::V10), 0x0A);
        assert_eq!(u8::from(Nibble::V11), 0x0B);
        assert_eq!(u8::from(Nibble::V12), 0x0C);
        assert_eq!(u8::from(Nibble::V13), 0x0D);
        assert_eq!(u8::from(Nibble::V14), 0x0E);
        assert_eq!(u8::from(Nibble::V15), 0x0F);
    }

    #[test]
    fn nibble_into_usize() {
        assert_eq!(usize::from(Nibble::V0), 0x00);
        assert_eq!(usize::from(Nibble::V1), 0x01);
        assert_eq!(usize::from(Nibble::V2), 0x02);
        assert_eq!(usize::from(Nibble::V3), 0x03);
        assert_eq!(usize::from(Nibble::V4), 0x04);
        assert_eq!(usize::from(Nibble::V5), 0x05);
        assert_eq!(usize::from(Nibble::V6), 0x06);
        assert_eq!(usize::from(Nibble::V7), 0x07);
        assert_eq!(usize::from(Nibble::V8), 0x08);
        assert_eq!(usize::from(Nibble::V9), 0x09);
        assert_eq!(usize::from(Nibble::V10), 0x0A);
        assert_eq!(usize::from(Nibble::V11), 0x0B);
        assert_eq!(usize::from(Nibble::V12), 0x0C);
        assert_eq!(usize::from(Nibble::V13), 0x0D);
        assert_eq!(usize::from(Nibble::V14), 0x0E);
        assert_eq!(usize::from(Nibble::V15), 0x0F);
    }

    #[test]
    fn nibble_slice_skip_prefix_success() {
        let mut slice = NibbleSlice {
            data: &[0x12, 0x34, 0x56],
            offset: 0,
        };
        assert!(slice.skip_prefix(&NibbleVec {
            data: SmallVec::from_slice(&[0x12, 0x34, 0x56]),
            first_is_half: false,
            last_is_half: false,
        }));
        assert_eq!(slice.offset, 6);
    }

    #[test]
    fn nibble_slice_skip_prefix_success_first_half() {
        let mut slice = NibbleSlice {
            data: &[0x12, 0x34, 0x56],
            offset: 1,
        };
        assert!(slice.skip_prefix(&NibbleVec {
            data: SmallVec::from_slice(&[0x02, 0x34, 0x56]),
            first_is_half: true,
            last_is_half: false,
        }));
        assert_eq!(slice.offset, 6);
    }

    #[test]
    fn nibble_slice_skip_prefix_success_last_half() {
        let mut slice = NibbleSlice {
            data: &[0x12, 0x34, 0x56],
            offset: 0,
        };
        assert!(slice.skip_prefix(&NibbleVec {
            data: SmallVec::from_slice(&[0x12, 0x34, 0x50]),
            first_is_half: false,
            last_is_half: true,
        }));
        assert_eq!(slice.offset, 5);
    }

    #[test]
    fn nibble_slice_skip_prefix_success_first_last_half() {
        let mut slice = NibbleSlice {
            data: &[0x12, 0x34, 0x56],
            offset: 1,
        };
        assert!(slice.skip_prefix(&NibbleVec {
            data: SmallVec::from_slice(&[0x02, 0x34, 0x50]),
            first_is_half: true,
            last_is_half: true,
        }));
        assert_eq!(slice.offset, 5);
    }

    #[test]
    fn nibble_slice_skip_prefix_success_empty() {
        let mut slice = NibbleSlice {
            data: &[0x12, 0x34, 0x56],
            offset: 0,
        };
        assert!(slice.skip_prefix(&NibbleVec {
            data: SmallVec::new(),
            first_is_half: false,
            last_is_half: false
        }),);
        assert_eq!(slice.offset, 0);

        let mut slice = NibbleSlice {
            data: &[0x12, 0x34, 0x56],
            offset: 1,
        };
        assert!(slice.skip_prefix(&NibbleVec {
            data: SmallVec::from_slice(&[0x00]),
            first_is_half: true,
            last_is_half: true
        }),);
        assert_eq!(slice.offset, 1);
    }

    #[test]
    fn nibble_slice_skip_prefix_failure() {
        let mut slice = NibbleSlice {
            data: &[0x12, 0x34, 0x56],
            offset: 0,
        };
        assert!(!slice.skip_prefix(&NibbleVec {
            data: SmallVec::from_slice(&[0x21, 0x43, 0x65]),
            first_is_half: false,
            last_is_half: false,
        }));
        assert_eq!(slice.offset, 0);
    }

    #[test]
    #[should_panic]
    fn nibble_slice_skip_prefix_failure_alignment() {
        let mut slice = NibbleSlice {
            data: &[0x12, 0x34, 0x56],
            offset: 0,
        };
        slice.skip_prefix(&NibbleVec {
            data: SmallVec::from_slice(&[0x12, 0x34, 0x56]),
            first_is_half: true,
            last_is_half: false,
        });
    }

    #[test]
    fn nibble_slice_cmp_rest_success() {
        let slice = NibbleSlice {
            data: &[0x12, 0x34, 0x56],
            offset: 0,
        };
        assert!(slice.cmp_rest(&[0x12, 0x34, 0x56]));
    }

    #[test]
    fn nibble_slice_cmp_rest_success_offset() {
        let slice = NibbleSlice {
            data: &[0x12, 0x34, 0x56],
            offset: 3,
        };
        assert!(slice.cmp_rest(&[0xFF, 0xF4, 0x56]));
    }

    #[test]
    fn nibble_slice_cmp_rest_failure() {
        let slice = NibbleSlice {
            data: &[0x12, 0x34, 0x56],
            offset: 0,
        };
        assert!(!slice.cmp_rest(&[0x12, 0xF4, 0x56]));
    }

    #[test]
    fn nibble_slice_cmp_rest_failure_offset() {
        let slice = NibbleSlice {
            data: &[0x12, 0x34, 0x56],
            offset: 3,
        };
        assert!(!slice.cmp_rest(&[0xFF, 0xF4, 0xF6]));
    }

    #[test]
    fn nibble_slice_next() {
        let mut slice = NibbleSlice {
            data: &[0x12, 0x34, 0x56],
            offset: 0,
        };
        assert_eq!(slice.offset, 0);
        assert_eq!(slice.next(), Some(Nibble::V1));
        assert_eq!(slice.offset, 1);
        assert_eq!(slice.next(), Some(Nibble::V2));
        assert_eq!(slice.offset, 2);
        assert_eq!(slice.next(), Some(Nibble::V3));
        assert_eq!(slice.offset, 3);
        assert_eq!(slice.next(), Some(Nibble::V4));
        assert_eq!(slice.offset, 4);
        assert_eq!(slice.next(), Some(Nibble::V5));
        assert_eq!(slice.offset, 5);
        assert_eq!(slice.next(), Some(Nibble::V6));
        assert_eq!(slice.offset, 6);
        assert_eq!(slice.next(), None);
        assert_eq!(slice.offset, 6);
    }

    #[test]
    fn nibble_vec_split_extract_at_zero() {
        let vec = NibbleVec {
            data: SmallVec::from_slice(&[0x12, 0x34, 0x56]),
            first_is_half: false,
            last_is_half: false,
        };

        let (l, c, r) = vec.split_extract_at(0);
        assert!(l.data.as_slice().is_empty());
        assert!(!l.first_is_half);
        assert!(!l.last_is_half);
        assert_eq!(c, Nibble::V1);
        assert_eq!(r.data.as_slice(), &[0x12, 0x34, 0x56]);
        assert!(r.first_is_half);
        assert!(!r.last_is_half);
    }

    #[test]
    fn nibble_vec_split_extract_at_zero_first_half() {
        let vec = NibbleVec {
            data: SmallVec::from_slice(&[0x12, 0x34, 0x56]),
            first_is_half: true,
            last_is_half: false,
        };

        let (l, c, r) = vec.split_extract_at(0);
        assert!(l.data.as_slice().is_empty());
        assert!(!l.first_is_half);
        assert!(!l.last_is_half);
        assert_eq!(c, Nibble::V2);
        assert_eq!(r.data.as_slice(), &[0x34, 0x56]);
        assert!(!r.first_is_half);
        assert!(!r.last_is_half);
    }

    #[test]
    fn nibble_vec_split_extract_at_zero_last_half() {
        let vec = NibbleVec {
            data: SmallVec::from_slice(&[0x12, 0x34, 0x56]),
            first_is_half: false,
            last_is_half: true,
        };

        let (l, c, r) = vec.split_extract_at(0);
        assert!(l.data.as_slice().is_empty());
        assert!(!l.first_is_half);
        assert!(!l.last_is_half);
        assert_eq!(c, Nibble::V1);
        assert_eq!(r.data.as_slice(), &[0x12, 0x34, 0x56]);
        assert!(r.first_is_half);
        assert!(r.last_is_half);
    }

    #[test]
    fn nibble_vec_split_extract_at_zero_first_last_half() {
        let vec = NibbleVec {
            data: SmallVec::from_slice(&[0x12, 0x34, 0x56]),
            first_is_half: true,
            last_is_half: true,
        };

        let (l, c, r) = vec.split_extract_at(0);
        assert!(l.data.as_slice().is_empty());
        assert!(!l.first_is_half);
        assert!(!l.last_is_half);
        assert_eq!(c, Nibble::V2);
        assert_eq!(r.data.as_slice(), &[0x34, 0x56]);
        assert!(!r.first_is_half);
        assert!(r.last_is_half);
    }

    #[test]
    fn nibble_vec_split_extract_at_middle() {
        let vec = NibbleVec {
            data: SmallVec::from_slice(&[0x12, 0x34, 0x56]),
            first_is_half: false,
            last_is_half: false,
        };

        let (l, c, r) = vec.split_extract_at(3);
        assert_eq!(l.data.as_slice(), &[0x12, 0x34]);
        assert!(!l.first_is_half);
        assert!(l.last_is_half);
        assert_eq!(c, Nibble::V4);
        assert_eq!(r.data.as_slice(), &[0x56]);
        assert!(!r.first_is_half);
        assert!(!r.last_is_half);
    }

    #[test]
    fn nibble_vec_split_extract_at_middle_first_half() {
        let vec = NibbleVec {
            data: SmallVec::from_slice(&[0x12, 0x34, 0x56]),
            first_is_half: true,
            last_is_half: false,
        };

        let (l, c, r) = vec.split_extract_at(2);
        assert_eq!(l.data.as_slice(), &[0x12, 0x34]);
        assert!(l.first_is_half);
        assert!(l.last_is_half);
        assert_eq!(c, Nibble::V4);
        assert_eq!(r.data.as_slice(), &[0x56]);
        assert!(!r.first_is_half);
        assert!(!r.last_is_half);
    }

    #[test]
    fn nibble_vec_split_extract_at_middle_last_half() {
        let vec = NibbleVec {
            data: SmallVec::from_slice(&[0x12, 0x34, 0x56]),
            first_is_half: false,
            last_is_half: true,
        };

        let (l, c, r) = vec.split_extract_at(3);
        assert_eq!(l.data.as_slice(), &[0x12, 0x34]);
        assert!(!l.first_is_half);
        assert!(l.last_is_half);
        assert_eq!(c, Nibble::V4);
        assert_eq!(r.data.as_slice(), &[0x56]);
        assert!(!r.first_is_half);
        assert!(r.last_is_half);
    }

    #[test]
    fn nibble_vec_split_extract_at_middle_first_last_half() {
        let vec = NibbleVec {
            data: SmallVec::from_slice(&[0x12, 0x34, 0x56]),
            first_is_half: true,
            last_is_half: true,
        };

        let (l, c, r) = vec.split_extract_at(2);
        assert_eq!(l.data.as_slice(), &[0x12, 0x34]);
        assert!(l.first_is_half);
        assert!(l.last_is_half);
        assert_eq!(c, Nibble::V4);
        assert_eq!(r.data.as_slice(), &[0x56]);
        assert!(!r.first_is_half);
        assert!(r.last_is_half);
    }

    #[test]
    fn nibble_vec_split_extract_at_end_minus_one() {
        let vec = NibbleVec {
            data: SmallVec::from_slice(&[0x12, 0x34, 0x56]),
            first_is_half: false,
            last_is_half: false,
        };

        let (l, c, r) = vec.split_extract_at(5);
        assert_eq!(l.data.as_slice(), &[0x12, 0x34, 0x56]);
        assert!(!l.first_is_half);
        assert!(l.last_is_half);
        assert_eq!(c, Nibble::V6);
        assert!(r.data.as_slice().is_empty());
        assert!(!r.first_is_half);
        assert!(!r.last_is_half);
    }

    #[test]
    fn nibble_vec_split_extract_at_end_minus_one_first_half() {
        let vec = NibbleVec {
            data: SmallVec::from_slice(&[0x12, 0x34, 0x56]),
            first_is_half: true,
            last_is_half: false,
        };

        let (l, c, r) = vec.split_extract_at(4);
        assert_eq!(l.data.as_slice(), &[0x12, 0x34, 0x56]);
        assert!(l.first_is_half);
        assert!(l.last_is_half);
        assert_eq!(c, Nibble::V6);
        assert!(r.data.as_slice().is_empty());
        assert!(!r.first_is_half);
        assert!(!r.last_is_half);
    }

    #[test]
    fn nibble_vec_split_extract_at_end_minus_one_last_half() {
        let vec = NibbleVec {
            data: SmallVec::from_slice(&[0x12, 0x34, 0x56]),
            first_is_half: false,
            last_is_half: true,
        };

        let (l, c, r) = vec.split_extract_at(4);
        assert_eq!(l.data.as_slice(), &[0x12, 0x34]);
        assert!(!l.first_is_half);
        assert!(!l.last_is_half);
        assert_eq!(c, Nibble::V5);
        assert!(r.data.as_slice().is_empty());
        assert!(!r.first_is_half);
        assert!(!r.last_is_half);
    }

    #[test]
    fn nibble_vec_split_extract_at_end_minus_one_first_last_half() {
        let vec = NibbleVec {
            data: SmallVec::from_slice(&[0x12, 0x34, 0x56]),
            first_is_half: true,
            last_is_half: true,
        };

        let (l, c, r) = vec.split_extract_at(3);
        assert_eq!(l.data.as_slice(), &[0x12, 0x34]);
        assert!(l.first_is_half);
        assert!(!l.last_is_half);
        assert_eq!(c, Nibble::V5);
        assert!(r.data.as_slice().is_empty());
        assert!(!r.first_is_half);
        assert!(!r.last_is_half);
    }

    #[test]
    #[should_panic]
    fn nibble_vec_split_extract_at_end() {
        let vec = NibbleVec {
            data: SmallVec::from_slice(&[0x12, 0x34, 0x56]),
            first_is_half: false,
            last_is_half: false,
        };

        vec.split_extract_at(6);
    }

    #[test]
    fn nibble_vec_iter_next() {
        let vec = NibbleVec {
            data: SmallVec::from_slice(&[0x12, 0x34, 0x56]),
            first_is_half: false,
            last_is_half: false,
        };
        let mut vec_iter = vec.iter();

        assert_eq!(vec_iter.pos, 0);
        assert_eq!(vec_iter.next(), Some(Nibble::V1));
        assert_eq!(vec_iter.pos, 1);
        assert_eq!(vec_iter.next(), Some(Nibble::V2));
        assert_eq!(vec_iter.pos, 2);
        assert_eq!(vec_iter.next(), Some(Nibble::V3));
        assert_eq!(vec_iter.pos, 3);
        assert_eq!(vec_iter.next(), Some(Nibble::V4));
        assert_eq!(vec_iter.pos, 4);
        assert_eq!(vec_iter.next(), Some(Nibble::V5));
        assert_eq!(vec_iter.pos, 5);
        assert_eq!(vec_iter.next(), Some(Nibble::V6));
        assert_eq!(vec_iter.pos, 6);
        assert_eq!(vec_iter.next(), None);
        assert_eq!(vec_iter.pos, 6);
    }

    #[test]
    fn nibble_vec_iter_next_first_half() {
        let vec = NibbleVec {
            data: SmallVec::from_slice(&[0x12, 0x34, 0x56]),
            first_is_half: true,
            last_is_half: false,
        };
        let mut vec_iter = vec.iter();

        assert_eq!(vec_iter.pos, 1);
        assert_eq!(vec_iter.next(), Some(Nibble::V2));
        assert_eq!(vec_iter.pos, 2);
        assert_eq!(vec_iter.next(), Some(Nibble::V3));
        assert_eq!(vec_iter.pos, 3);
        assert_eq!(vec_iter.next(), Some(Nibble::V4));
        assert_eq!(vec_iter.pos, 4);
        assert_eq!(vec_iter.next(), Some(Nibble::V5));
        assert_eq!(vec_iter.pos, 5);
        assert_eq!(vec_iter.next(), Some(Nibble::V6));
        assert_eq!(vec_iter.pos, 6);
        assert_eq!(vec_iter.next(), None);
        assert_eq!(vec_iter.pos, 6);
    }

    #[test]
    fn nibble_vec_iter_next_last_half() {
        let vec = NibbleVec {
            data: SmallVec::from_slice(&[0x12, 0x34, 0x56]),
            first_is_half: false,
            last_is_half: true,
        };
        let mut vec_iter = vec.iter();

        assert_eq!(vec_iter.pos, 0);
        assert_eq!(vec_iter.next(), Some(Nibble::V1));
        assert_eq!(vec_iter.pos, 1);
        assert_eq!(vec_iter.next(), Some(Nibble::V2));
        assert_eq!(vec_iter.pos, 2);
        assert_eq!(vec_iter.next(), Some(Nibble::V3));
        assert_eq!(vec_iter.pos, 3);
        assert_eq!(vec_iter.next(), Some(Nibble::V4));
        assert_eq!(vec_iter.pos, 4);
        assert_eq!(vec_iter.next(), Some(Nibble::V5));
        assert_eq!(vec_iter.pos, 5);
        assert_eq!(vec_iter.next(), None);
        assert_eq!(vec_iter.pos, 5);
    }

    #[test]
    fn nibble_vec_iter_next_first_last_half() {
        let vec = NibbleVec {
            data: SmallVec::from_slice(&[0x12, 0x34, 0x56]),
            first_is_half: true,
            last_is_half: true,
        };
        let mut vec_iter = vec.iter();

        assert_eq!(vec_iter.pos, 1);
        assert_eq!(vec_iter.next(), Some(Nibble::V2));
        assert_eq!(vec_iter.pos, 2);
        assert_eq!(vec_iter.next(), Some(Nibble::V3));
        assert_eq!(vec_iter.pos, 3);
        assert_eq!(vec_iter.next(), Some(Nibble::V4));
        assert_eq!(vec_iter.pos, 4);
        assert_eq!(vec_iter.next(), Some(Nibble::V5));
        assert_eq!(vec_iter.pos, 5);
        assert_eq!(vec_iter.next(), None);
        assert_eq!(vec_iter.pos, 5);
    }
}
