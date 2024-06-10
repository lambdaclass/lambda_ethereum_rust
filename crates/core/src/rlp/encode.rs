use bytes::BufMut;
use tinyvec::{Array, ArrayVec};

trait RLPEncode {
    fn encode(&self, buf: &mut dyn BufMut);

    fn length(&self) -> usize {
        let mut buf = Vec::new();
        self.encode(&mut buf);
        buf.len()
    }
}

impl RLPEncode for bool {
    #[inline(always)]
    fn encode(&self, buf: &mut dyn BufMut) {
        if *self {
            buf.put_u8(0x01);
        } else {
            buf.put_u8(0x80);
        }
    }

    #[inline(always)]
    fn length(&self) -> usize {
        1
    }
}

// integer types impls

impl RLPEncode for u8 {
    fn encode(&self, buf: &mut dyn BufMut) {
        match *self {
            // 0, also known as null or the empty string is 0x80
            0 => buf.put_u8(0x80),
            // for a single byte whose value is in the [0x00, 0x7f] range, that byte is its own RLP encoding.
            n @ 1..=0x7f => buf.put_u8(n),
            // Otherwise, if a string is 0-55 bytes long, the RLP encoding consists of a
            // single byte with value 0x80 plus the length of the string followed by the string.
            n => {
                let mut bytes = ArrayVec::<[u8; 8]>::new();
                bytes.extend_from_slice(&n.to_be_bytes());
                let start = bytes.iter().position(|&x| x != 0).unwrap();
                let len = bytes.len() - start;
                buf.put_u8(0x80 + len as u8);
                buf.put_slice(&bytes[start..]);
            }
        }
    }
}

impl RLPEncode for u16 {
    fn encode(&self, buf: &mut dyn BufMut) {
        match *self {
            // 0, also known as null or the empty string is 0x80
            0 => buf.put_u8(0x80),
            // for a single byte whose value is in the [0x00, 0x7f] range, that byte is its own RLP encoding.
            n @ 1..=0x7f => buf.put_u16(n),
            // Otherwise, if a string is 0-55 bytes long, the RLP encoding consists of a
            // single byte with value 0x80 plus the length of the string followed by the string.
            n => {
                let mut bytes = ArrayVec::<[u8; 8]>::new();
                bytes.extend_from_slice(&n.to_be_bytes());
                let start = bytes.iter().position(|&x| x != 0).unwrap();
                let len = bytes.len() - start;
                buf.put_u8(0x80 + len as u8);
                buf.put_slice(&bytes[start..]);
            }
        }
    }
}

impl RLPEncode for u32 {
    fn encode(&self, buf: &mut dyn BufMut) {
        match *self {
            // 0, also known as null or the empty string is 0x80
            0 => buf.put_u8(0x80),
            // for a single byte whose value is in the [0x00, 0x7f] range, that byte is its own RLP encoding.
            n @ 1..=0x7f => buf.put_u8(n as u8),
            // Otherwise, if a string is 0-55 bytes long, the RLP encoding consists of a
            // single byte with value 0x80 plus the length of the string followed by the string.
            n => {
                let mut bytes = ArrayVec::<[u8; 8]>::new();
                bytes.extend_from_slice(&n.to_be_bytes());
                let start = bytes.iter().position(|&x| x != 0).unwrap();
                let len = bytes.len() - start;
                buf.put_u8(0x80 + len as u8);
                buf.put_slice(&bytes[start..]);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::RLPEncode;

    // #[test]
    // fn can_parse_dog_string() {
    //     // encoded message
    //     let encoded: [u8; 4] = [0x83, b'd', b'o', b'g'];
    // }

    // fn can_parse_string_list() {
    //     // encoded message
    //     let encoded: [u8; 9] = [0xc8, 0x83, b'c', b'a', b't', 0x83, b'd', b'o', b'g'];
    // }

    // fn can_parse_empty_string() {
    //     // encoded message
    //     let encoded: [u8; 1] = [0x80];
    // }

    // fn can_parse_empty_list() {
    //     // encoded message
    //     let encoded: [u8; 1] = [0xc0];
    // }

    #[test]
    fn can_encode_booleans() {
        let mut encoded = Vec::new();
        true.encode(&mut encoded);
        assert_eq!(encoded, vec![0x01]);

        let mut encoded = Vec::new();
        false.encode(&mut encoded);
        assert_eq!(encoded, vec![0x80]);
    }

    #[test]
    fn can_encode_u32() {
        let mut encoded = Vec::new();
        0u16.encode(&mut encoded);
        assert_eq!(encoded, vec![0x80]);

        let mut encoded = Vec::new();
        1u16.encode(&mut encoded);
        assert_eq!(encoded, vec![0x01]);

        let mut encoded = Vec::new();
        0x7Fu16.encode(&mut encoded);
        assert_eq!(encoded, vec![0x7f]);

        let mut encoded = Vec::new();
        0x80u16.encode(&mut encoded);
        assert_eq!(encoded, vec![0x80 + 1, 0x80]);

        let mut encoded = Vec::new();
        0x90u16.encode(&mut encoded);
        assert_eq!(encoded, vec![0x80 + 1, 0x90]);
    }

    #[test]
    fn can_encode_u16() {
        let mut encoded = Vec::new();
        0u16.encode(&mut encoded);
        assert_eq!(encoded, vec![0x80]);

        let mut encoded = Vec::new();
        1u16.encode(&mut encoded);
        assert_eq!(encoded, vec![0x01]);

        let mut encoded = Vec::new();
        0x7Fu16.encode(&mut encoded);
        assert_eq!(encoded, vec![0x7f]);

        let mut encoded = Vec::new();
        0x80u16.encode(&mut encoded);
        assert_eq!(encoded, vec![0x80 + 1, 0x80]);

        let mut encoded = Vec::new();
        0x90u16.encode(&mut encoded);
        assert_eq!(encoded, vec![0x80 + 1, 0x90]);
    }

    #[test]
    fn can_encode_u8() {
        let mut encoded = Vec::new();
        0u8.encode(&mut encoded);
        assert_eq!(encoded, vec![0x80]);

        let mut encoded = Vec::new();
        1u8.encode(&mut encoded);
        assert_eq!(encoded, vec![0x01]);

        let mut encoded = Vec::new();
        0x7Fu8.encode(&mut encoded);
        assert_eq!(encoded, vec![0x7f]);

        let mut encoded = Vec::new();
        0x80u8.encode(&mut encoded);
        assert_eq!(encoded, vec![0x80 + 1, 0x80]);

        let mut encoded = Vec::new();
        0x90u8.encode(&mut encoded);
        assert_eq!(encoded, vec![0x80 + 1, 0x90]);
    }

    // fn can_parse_byte_zero() {
    //     // encoded message
    //     let encoded: [u8; 1] = [0x00];
    // }

    // fn can_parse_byte_f() {
    //     // encoded message
    //     let encoded: [u8; 1] = [0x0f];
    // }

    // fn can_parse_bytes_0400() {
    //     // encoded message
    //     let encoded: [u8; 3] = [0x82, 0x04, 0x00];
    // }
}
