use super::error::RLPDecodeError;
use bytes::Bytes;

pub trait RLPDecode: Sized {
    fn decode(rlp: &[u8]) -> Result<Self, RLPDecodeError>;
}

// According to the rules and process of RLP encoding, the input of RLP decode is regarded as an array of binary data. The RLP decoding process is as follows:

// according to the *first byte* (i.e. prefix) of input data and *decoding the data type*, *the length of the actual data* and *offset*;

// according to the type and offset of data, decode the data correspondingly, respecting the minimal encoding rule for positive integers;

// continue to decode the rest of the input;

// Among them, the rules of decoding data types and offset is as follows:

// the data is a string if the range of the first byte (i.e. prefix) is [0x00, 0x7f], and the string is the first byte itself exactly;

// the data is a string if the range of the first byte is [0x80, 0xb7], and the string whose length is equal to the first byte minus 0x80 follows the first byte;

// the data is a string if the range of the first byte is [0xb8, 0xbf], and the length of the string whose length in bytes is equal to the first byte minus 0xb7 follows the first byte, and the string follows the length of the string;

// the data is a list if the range of the first byte is [0xc0, 0xf7], and the concatenation of the RLP encodings of all items of the list which the total payload is equal to the first byte minus 0xc0 follows the first byte;

// the data is a list if the range of the first byte is [0xf8, 0xff], and the total payload of the list whose length is equal to the first byte minus 0xf7 follows the first byte, and the concatenation of the RLP encodings of all items of the list follows the total payload of the list;

impl RLPDecode for bool {
    #[inline(always)]
    fn decode(buf: &[u8]) -> Result<Self, RLPDecodeError> {
        let bytes = Bytes::copy_from_slice(buf);
        let len = bytes.len();

        if len == 0 {
            return Err(RLPDecodeError::InvalidLength);
        }

        if len == 1 {
            return Ok(buf[0] != 0x80);
        }

        Ok(false)
    }
}

// integer decoding impls
impl RLPDecode for u8 {
    fn decode(buf: &[u8]) -> Result<Self, RLPDecodeError> {
        match buf.len() {
            0 => Err(RLPDecodeError::InvalidLength),
            1 if (0x00..=0x7f).contains(&buf[0]) => Ok(buf[0]),
            1 if buf[0] == 0x80 => Ok(0),
            2 if buf[0] == 0x81 => Ok(buf[1]),
            _ => Err(RLPDecodeError::InvalidLength),
        }
    }
}

impl RLPDecode for u16 {
    fn decode(buf: &[u8]) -> Result<Self, RLPDecodeError> {
        match buf.len() {
            0 => Err(RLPDecodeError::InvalidLength),
            n if n <= 2 => u8::decode(buf).map(|v| v as u16),
            _ => Err(RLPDecodeError::InvalidLength),
        }
    }
}

impl<const N: usize> RLPDecode for [u8; N] {
    fn decode(rlp: &[u8]) -> Result<Self, RLPDecodeError> {
        let rlp_iter = rlp.iter();
        let mut len = 0;
        let first_byte = match rlp_iter.next() {
            Some(&first_byte) => first_byte,
            None => return Err(RLPDecodeError::InvalidLength),
        };


        match first_byte {
            0..=0x7F => len = 1,
            0x80..=0xB7 => len = (first_byte - 0x80) as usize,
            b @ 0xB8..=0xBF => {
                let content_type = match b >= 0xF8 {
                    //
                    true => 0xF7,
                    false => 0xB7,
                };


            }
        }
        Ok([0; N])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_bool() {
        let rlp = vec![0x01];
        let decoded = bool::decode(&rlp).unwrap();
        assert_eq!(decoded, true);

        let rlp = vec![0x80];
        let decoded = bool::decode(&rlp).unwrap();
        assert_eq!(decoded, false);
    }

    #[test]
    fn test_decode_u8() {
        let rlp = vec![0x01];
        let decoded = u8::decode(&rlp).unwrap();
        assert_eq!(decoded, 1);

        let rlp = vec![0x80];
        let decoded = u8::decode(&rlp).unwrap();
        assert_eq!(decoded, 0);

        let rlp = vec![0x7Fu8];
        let decoded = u8::decode(&rlp).unwrap();
        assert_eq!(decoded, 127);

        let rlp = vec![0x81u8, 0x80];
        let decoded = u8::decode(&rlp).unwrap();
        assert_eq!(decoded, 128);

        let rlp = vec![0x80 + 1, 0x90];
        let decoded = u8::decode(&rlp).unwrap();
        assert_eq!(decoded, 144);

        let rlp = vec![0x81, 0xFF];
        let decoded = u8::decode(&rlp).unwrap();
        assert_eq!(decoded, 255);
    }

    #[test]
    fn test_decode_u16() {
        let rlp = vec![0x01];
        let decoded = u8::decode(&rlp).unwrap();
        assert_eq!(decoded, 1);

        let rlp = vec![0x80];
        let decoded = u8::decode(&rlp).unwrap();
        assert_eq!(decoded, 0);

        let rlp = vec![0x81, 0xFF];
        let decoded = u8::decode(&rlp).unwrap();
        assert_eq!(decoded, 255);
    }

    // #[test]
    // fn test_decode_u32() {
    //     let rlp = vec![0x83, 0x01, 0x00, 0x00];
    //     let decoded = u32::decode(&rlp).unwrap();
    //     assert_eq!(decoded, 65536);
    // }

    #[test]
    fn can_decode_fixed_length_array() {
        let rlp = vec![0x83, 0x02, 0x03, 0x04];
        let decoded = <[u8; 3]>::decode(&rlp).unwrap();
        assert_eq!(decoded, [0x02, 0x03, 0x04]);
    }
}
