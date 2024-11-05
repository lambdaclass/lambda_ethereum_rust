pub struct DumbNibbles {
    data: Vec<u8>,
}

impl DumbNibbles {
    pub fn from_hex(hex: Vec<u8>) -> Self {
        Self { data: hex }
    }

    pub fn from_bytes(bytes: Vec<u8>) -> Self {
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
    pub fn skip_prefix(&mut self, prefix: DumbNibbles) -> bool {
        if self.len() >= prefix.len() && &self.data[..prefix.len()] == prefix.as_ref() {
            self.data = self.data[prefix.len()..].to_vec();
            true
        } else {
            false
        }
    }

    /// Count how many nibbles two DumbNibbles have in common
    pub fn common_prefix(&self, other: &DumbNibbles) -> usize {
        self.as_ref()
            .iter()
            .zip(other.as_ref().iter())
            .take_while(|(a, b)| a == b)
            .count()
    }
}

impl AsRef<[u8]> for DumbNibbles {
    fn as_ref(&self) -> &[u8] {
        &self.data
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn skip_prefix_true() {
        let mut a = DumbNibbles::from_hex(vec![1, 2, 3, 4, 5]);
        let b = DumbNibbles::from_hex(vec![1, 2, 3]);
        assert!(a.skip_prefix(b));
        assert_eq!(a.as_ref(), &[4, 5])
    }

    #[test]
    fn skip_prefix_true_same_length() {
        let mut a = DumbNibbles::from_hex(vec![1, 2, 3, 4, 5]);
        let b = DumbNibbles::from_hex(vec![1, 2, 3, 4, 5]);
        assert!(a.skip_prefix(b));
        assert!(a.is_empty());
    }

    #[test]
    fn skip_prefix_longer_prefix() {
        let mut a = DumbNibbles::from_hex(vec![1, 2, 3]);
        let b = DumbNibbles::from_hex(vec![1, 2, 3, 4, 5]);
        assert!(!a.skip_prefix(b));
        assert_eq!(a.as_ref(), &[1, 2, 3])
    }

    #[test]
    fn skip_prefix_false() {
        let mut a = DumbNibbles::from_hex(vec![1, 2, 3, 4, 5]);
        let b = DumbNibbles::from_hex(vec![1, 2, 4]);
        assert!(!a.skip_prefix(b));
        assert_eq!(a.as_ref(), &[1, 2, 3, 4, 5])
    }

    #[test]
    fn common_prefix_all() {
        let a = DumbNibbles::from_hex(vec![1, 2, 3, 4, 5]);
        let b = DumbNibbles::from_hex(vec![1, 2, 3, 4, 5]);
        assert_eq!(a.common_prefix(&b), a.len());
    }

    #[test]
    fn common_prefix_partial() {
        let a = DumbNibbles::from_hex(vec![1, 2, 3, 4, 5]);
        let b = DumbNibbles::from_hex(vec![1, 2, 3]);
        assert_eq!(a.common_prefix(&b), b.len());
    }

    #[test]
    fn common_prefix_none() {
        let a = DumbNibbles::from_hex(vec![1, 2, 3, 4, 5]);
        let b = DumbNibbles::from_hex(vec![2, 3, 4, 5, 6]);
        assert_eq!(a.common_prefix(&b), 0);
    }
}
