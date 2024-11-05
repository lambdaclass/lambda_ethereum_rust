use ethereum_rust_rlp::{
    decode::RLPDecode,
    encode::RLPEncode,
    error::RLPDecodeError,
    structs::{Decoder, Encoder},
};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct DumbNibbles {
    data: Vec<u8>,
}

impl DumbNibbles {
    pub fn from_hex(hex: Vec<u8>) -> Self {
        Self { data: hex }
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        Self {
            data: bytes
                .iter()
                .flat_map(|byte| [(byte >> 4 & 0x0F), byte & 0x0F])
                .collect(),
        }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// If `prefix` is a prefix of self, move the offset after
    /// the prefix and return true, otherwise return false.
    pub fn skip_prefix(&mut self, prefix: &DumbNibbles) -> bool {
        if self.len() >= prefix.len() && &self.data[..prefix.len()] == prefix.as_ref() {
            self.data = self.data[prefix.len()..].to_vec();
            true
        } else {
            false
        }
    }

    /// Compares self to another and returns the shared nibble count (amount of nibbles that are equal, from the start)
    pub fn count_prefix(&self, other: &DumbNibbles) -> usize {
        self.as_ref()
            .iter()
            .zip(other.as_ref().iter())
            .take_while(|(a, b)| a == b)
            .count()
    }

    /// Removes and returns the first nibble
    pub fn next(&mut self) -> Option<u8> {
        (!self.is_empty()).then_some(self.data.remove(0))
    }

    pub fn offset(&self, offset: usize) -> DumbNibbles {
        self.slice(offset, self.len())
    }

    pub fn slice(&self, start: usize, end: usize) -> DumbNibbles {
        DumbNibbles::from_hex(self.data[start..end].to_vec())
    }

    pub fn extend(&mut self, other: &DumbNibbles) {
        self.data.extend_from_slice(other.as_ref());
    }

    pub fn at(&self, i: usize) -> usize {
        self.data[i] as usize
    }

    /// Inserts a nibble at the start
    pub fn prepend(&mut self, nibble: u8) {
        self.data.insert(0, nibble);
    }
}

impl AsRef<[u8]> for DumbNibbles {
    fn as_ref(&self) -> &[u8] {
        &self.data
    }
}

impl RLPEncode for DumbNibbles {
    fn encode(&self, buf: &mut dyn bytes::BufMut) {
        Encoder::new(buf).encode_field(&self.data);
    }
}

impl RLPDecode for DumbNibbles {
    fn decode_unfinished(rlp: &[u8]) -> Result<(Self, &[u8]), RLPDecodeError> {
        let decoder = Decoder::new(rlp)?;
        let (data, decoder) = decoder.decode_field("data")?;
        Ok((Self { data }, decoder.finish()?))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn skip_prefix_true() {
        let mut a = DumbNibbles::from_hex(vec![1, 2, 3, 4, 5]);
        let b = DumbNibbles::from_hex(vec![1, 2, 3]);
        assert!(a.skip_prefix(&b));
        assert_eq!(a.as_ref(), &[4, 5])
    }

    #[test]
    fn skip_prefix_true_same_length() {
        let mut a = DumbNibbles::from_hex(vec![1, 2, 3, 4, 5]);
        let b = DumbNibbles::from_hex(vec![1, 2, 3, 4, 5]);
        assert!(a.skip_prefix(&b));
        assert!(a.is_empty());
    }

    #[test]
    fn skip_prefix_longer_prefix() {
        let mut a = DumbNibbles::from_hex(vec![1, 2, 3]);
        let b = DumbNibbles::from_hex(vec![1, 2, 3, 4, 5]);
        assert!(!a.skip_prefix(&b));
        assert_eq!(a.as_ref(), &[1, 2, 3])
    }

    #[test]
    fn skip_prefix_false() {
        let mut a = DumbNibbles::from_hex(vec![1, 2, 3, 4, 5]);
        let b = DumbNibbles::from_hex(vec![1, 2, 4]);
        assert!(!a.skip_prefix(&b));
        assert_eq!(a.as_ref(), &[1, 2, 3, 4, 5])
    }

    #[test]
    fn count_prefix_all() {
        let a = DumbNibbles::from_hex(vec![1, 2, 3, 4, 5]);
        let b = DumbNibbles::from_hex(vec![1, 2, 3, 4, 5]);
        assert_eq!(a.count_prefix(&b), a.len());
    }

    #[test]
    fn count_prefix_partial() {
        let a = DumbNibbles::from_hex(vec![1, 2, 3, 4, 5]);
        let b = DumbNibbles::from_hex(vec![1, 2, 3]);
        assert_eq!(a.count_prefix(&b), b.len());
    }

    #[test]
    fn count_prefix_none() {
        let a = DumbNibbles::from_hex(vec![1, 2, 3, 4, 5]);
        let b = DumbNibbles::from_hex(vec![2, 3, 4, 5, 6]);
        assert_eq!(a.count_prefix(&b), 0);
    }
}
