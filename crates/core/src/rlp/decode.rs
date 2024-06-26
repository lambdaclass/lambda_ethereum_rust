use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use super::{constants::RLP_NULL, error::RLPDecodeError};
use bytes::{Bytes, BytesMut};

/// According to the rules and process of RLP encoding, the input of RLP decode is regarded as an array of binary data.
///
/// The RLP decoding process is as follows:
/// according to the *first byte* (i.e. prefix) of input data and *decoding the data type*, *the length of the actual data* and *offset*;
/// according to the type and offset of data, decode the data correspondingly, respecting the minimal encoding rule for positive integers;
/// continue to decode the rest of the input;
///
/// Among them, the rules of decoding data types and offset is as follows:
/// - the data is a string if the range of the first byte (i.e. prefix) is [0x00, 0x7f], and the string is the first byte itself exactly;
/// - the data is a string if the range of the first byte is [0x80, 0xb7], and the string whose length is equal to the first byte minus 0x80 follows the first byte;
/// - the data is a string if the range of the first byte is [0xb8, 0xbf], and the length of the string whose length in bytes is equal to the first byte minus 0xb7 follows the first byte, and the string follows the length of the string;
/// - the data is a list if the range of the first byte is [0xc0, 0xf7], and the concatenation of the RLP encodings of all items of the list which the total payload is equal to the first byte minus 0xc0 follows the first byte;
/// - the data is a list if the range of the first byte is [0xf8, 0xff], and the total payload of the list whose length is equal to the first byte minus 0xf7 follows the first byte, and the concatenation of the RLP encodings of all items of the list follows the total payload of the list;
pub trait RLPDecode: Sized {
    fn decode(rlp: &[u8]) -> Result<Self, RLPDecodeError>;
}

impl RLPDecode for bool {
    #[inline(always)]
    fn decode(buf: &[u8]) -> Result<Self, RLPDecodeError> {
        let bytes = Bytes::copy_from_slice(buf);
        let len = bytes.len();

        if len == 0 {
            return Err(RLPDecodeError::InvalidLength);
        }

        if len == 1 {
            return Ok(buf[0] != RLP_NULL);
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
            1 if buf[0] == RLP_NULL => Ok(0),
            2 if buf[0] == RLP_NULL + 1 => Ok(buf[1]),
            _ => Err(RLPDecodeError::InvalidLength),
        }
    }
}

impl RLPDecode for u16 {
    fn decode(rlp: &[u8]) -> Result<Self, RLPDecodeError> {
        let bytes = decode_bytes(rlp)?;
        let padded_bytes = static_left_pad(bytes)?;
        Ok(u16::from_be_bytes(padded_bytes))
    }
}

impl RLPDecode for u32 {
    fn decode(rlp: &[u8]) -> Result<Self, RLPDecodeError> {
        let bytes = decode_bytes(rlp)?;
        let padded_bytes = static_left_pad(bytes)?;
        Ok(u32::from_be_bytes(padded_bytes))
    }
}

impl RLPDecode for u64 {
    fn decode(rlp: &[u8]) -> Result<Self, RLPDecodeError> {
        let bytes = decode_bytes(rlp)?;
        let padded_bytes = static_left_pad(bytes)?;
        Ok(u64::from_be_bytes(padded_bytes))
    }
}

impl RLPDecode for u128 {
    fn decode(rlp: &[u8]) -> Result<Self, RLPDecodeError> {
        let bytes = decode_bytes(rlp)?;
        let padded_bytes = static_left_pad(bytes)?;
        Ok(u128::from_be_bytes(padded_bytes))
    }
}

impl<const N: usize> RLPDecode for [u8; N] {
    fn decode(rlp: &[u8]) -> Result<Self, RLPDecodeError> {
        let decoded_bytes = decode_bytes(rlp)?;
        decoded_bytes
            .try_into()
            .map_err(|_| RLPDecodeError::InvalidLength)
    }
}

impl RLPDecode for Bytes {
    fn decode(rlp: &[u8]) -> Result<Self, RLPDecodeError> {
        decode_bytes(rlp).map(|decoded| Bytes::from(decoded.to_vec()))
    }
}

impl RLPDecode for BytesMut {
    fn decode(rlp: &[u8]) -> Result<Self, RLPDecodeError> {
        decode_bytes(rlp).map(BytesMut::from)
    }
}

impl RLPDecode for ethereum_types::H32 {
    fn decode(rlp: &[u8]) -> Result<Self, RLPDecodeError> {
        RLPDecode::decode(rlp).map(ethereum_types::H32)
    }
}

impl RLPDecode for ethereum_types::H64 {
    fn decode(rlp: &[u8]) -> Result<Self, RLPDecodeError> {
        RLPDecode::decode(rlp).map(ethereum_types::H64)
    }
}

impl RLPDecode for ethereum_types::H128 {
    fn decode(rlp: &[u8]) -> Result<Self, RLPDecodeError> {
        RLPDecode::decode(rlp).map(ethereum_types::H128)
    }
}

impl RLPDecode for ethereum_types::H256 {
    fn decode(rlp: &[u8]) -> Result<Self, RLPDecodeError> {
        RLPDecode::decode(rlp).map(ethereum_types::H256)
    }
}

impl RLPDecode for ethereum_types::H264 {
    fn decode(rlp: &[u8]) -> Result<Self, RLPDecodeError> {
        RLPDecode::decode(rlp).map(ethereum_types::H264)
    }
}

impl RLPDecode for ethereum_types::Address {
    fn decode(rlp: &[u8]) -> Result<Self, RLPDecodeError> {
        RLPDecode::decode(rlp).map(ethereum_types::H160)
    }
}

impl RLPDecode for ethereum_types::H512 {
    fn decode(rlp: &[u8]) -> Result<Self, RLPDecodeError> {
        RLPDecode::decode(rlp).map(ethereum_types::H512)
    }
}

impl RLPDecode for ethereum_types::Signature {
    fn decode(rlp: &[u8]) -> Result<Self, RLPDecodeError> {
        RLPDecode::decode(rlp).map(ethereum_types::H520)
    }
}

impl RLPDecode for String {
    fn decode(rlp: &[u8]) -> Result<Self, RLPDecodeError> {
        let str_bytes = decode_bytes(rlp)?.to_vec();
        String::from_utf8(str_bytes).map_err(|_| RLPDecodeError::MalformedData)
    }
}

impl RLPDecode for Ipv4Addr {
    fn decode(rlp: &[u8]) -> Result<Self, RLPDecodeError> {
        let ip_bytes = decode_bytes(rlp)?;
        let octets: [u8; 4] = ip_bytes
            .try_into()
            .map_err(|_| RLPDecodeError::InvalidLength)?;
        Ok(Ipv4Addr::from(octets))
    }
}

impl RLPDecode for Ipv6Addr {
    fn decode(rlp: &[u8]) -> Result<Self, RLPDecodeError> {
        let ip_bytes = decode_bytes(rlp)?;
        let octets: [u8; 16] = ip_bytes
            .try_into()
            .map_err(|_| RLPDecodeError::InvalidLength)?;
        Ok(Ipv6Addr::from(octets))
    }
}

impl RLPDecode for IpAddr {
    fn decode(rlp: &[u8]) -> Result<Self, RLPDecodeError> {
        let ip_bytes = decode_bytes(rlp)?;

        match ip_bytes.len() {
            4 => {
                let octets: [u8; 4] = ip_bytes
                    .try_into()
                    .map_err(|_| RLPDecodeError::InvalidLength)?;
                Ok(IpAddr::V4(Ipv4Addr::from(octets)))
            }
            16 => {
                let octets: [u8; 16] = ip_bytes
                    .try_into()
                    .map_err(|_| RLPDecodeError::InvalidLength)?;
                Ok(IpAddr::V6(Ipv6Addr::from(octets)))
            }
            _ => Err(RLPDecodeError::InvalidLength),
        }
    }
}

impl<T: RLPDecode> RLPDecode for Vec<T> {
    fn decode(rlp: &[u8]) -> Result<Self, RLPDecodeError> {
        todo!()
    }
}

#[inline]
fn decode_bytes(data: &[u8]) -> Result<&[u8], RLPDecodeError> {
    let mut rlp_iter = data.iter();
    let first_byte = match rlp_iter.next() {
        Some(&first_byte) => first_byte,
        None => return Err(RLPDecodeError::InvalidLength),
    };

    // calculate the length of the data based on the prefix
    match first_byte {
        // the data is a string if the range of the first byte (i.e. prefix) is [0x00, 0x7f],
        // and the string is the first byte itself exactly;
        0..=0x7F => Ok(&data[..1]),
        // the data is a string if the range of the first byte is [0x80, 0xb7], and the string
        // whose length is equal to the first byte minus 0x80 follows the first byte;
        RLP_NULL..=0xB7 => {
            let data_length = (first_byte - RLP_NULL) as usize;
            Ok(&data[1..data_length + 1])
        }
        // the data is a string if the range of the first byte is [0xb8, 0xbf], and the length
        // of the string whose length in bytes is equal to the first byte minus 0xb7 follows
        // the first byte, and the string follows the length of the string;
        b @ 0xB8..=0xBF if b < 0xF8 => {
            let length_size = (b - 0xB7) as usize;
            let data_length_bytes = &data[1..length_size];
            let data_length = usize::from_be_bytes(static_left_pad(data_length_bytes)?);
            Ok(&data[length_size + 1..data_length + 1])
        }
        _ => Err(RLPDecodeError::MalformedData),
    }
}

/// Pads a slice of bytes with zeros on the left to make it a fixed size slice.
/// The size of the data must be less than or equal to the size of the output array.
#[inline]
pub(crate) fn static_left_pad<const N: usize>(data: &[u8]) -> Result<[u8; N], RLPDecodeError> {
    let mut result = [0; N];

    if data.is_empty() {
        return Ok(result);
    }

    if data[0] == 0 {
        return Err(RLPDecodeError::MalformedData);
    }

    let data_start_index = N.saturating_sub(data.len());
    result
        .get_mut(data_start_index..)
        .ok_or(RLPDecodeError::InvalidLength)?
        .copy_from_slice(data);
    Ok(result)
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use crate::rlp::decode;

    use super::*;

    #[test]
    fn test_decode_bool() {
        let rlp = vec![0x01];
        let decoded = bool::decode(&rlp).unwrap();
        assert_eq!(decoded, true);

        let rlp = vec![RLP_NULL];
        let decoded = bool::decode(&rlp).unwrap();
        assert_eq!(decoded, false);
    }

    #[test]
    fn test_decode_u8() {
        let rlp = vec![0x01];
        let decoded = u8::decode(&rlp).unwrap();
        assert_eq!(decoded, 1);

        let rlp = vec![RLP_NULL];
        let decoded = u8::decode(&rlp).unwrap();
        assert_eq!(decoded, 0);

        let rlp = vec![0x7Fu8];
        let decoded = u8::decode(&rlp).unwrap();
        assert_eq!(decoded, 127);

        let rlp = vec![RLP_NULL + 1, RLP_NULL];
        let decoded = u8::decode(&rlp).unwrap();
        assert_eq!(decoded, 128);

        let rlp = vec![RLP_NULL + 1, 0x90];
        let decoded = u8::decode(&rlp).unwrap();
        assert_eq!(decoded, 144);

        let rlp = vec![RLP_NULL + 1, 0xFF];
        let decoded = u8::decode(&rlp).unwrap();
        assert_eq!(decoded, 255);
    }

    #[test]
    fn test_decode_u16() {
        let rlp = vec![0x01];
        let decoded = u8::decode(&rlp).unwrap();
        assert_eq!(decoded, 1);

        let rlp = vec![RLP_NULL];
        let decoded = u8::decode(&rlp).unwrap();
        assert_eq!(decoded, 0);

        let rlp = vec![0x81, 0xFF];
        let decoded = u8::decode(&rlp).unwrap();
        assert_eq!(decoded, 255);
    }

    #[test]
    fn test_decode_u32() {
        let rlp = vec![0x83, 0x01, 0x00, 0x00];
        let decoded = u32::decode(&rlp).unwrap();
        assert_eq!(decoded, 65536);
    }

    #[test]
    fn test_decode_fixed_length_array() {
        let rlp = vec![0x0f];
        let decoded = <[u8; 1]>::decode(&rlp).unwrap();
        assert_eq!(decoded, [0x0f]);

        let rlp = vec![RLP_NULL + 3, 0x02, 0x03, 0x04];
        let decoded = <[u8; 3]>::decode(&rlp).unwrap();
        assert_eq!(decoded, [0x02, 0x03, 0x04]);
    }

    #[test]
    fn test_decode_ip_addresses() {
        // IPv4
        let rlp = vec![RLP_NULL + 4, 192, 168, 0, 1];
        let decoded = Ipv4Addr::decode(&rlp).unwrap();
        let expected = Ipv4Addr::from_str("192.168.0.1").unwrap();
        assert_eq!(decoded, expected);

        // IPv6
        let rlp = vec![
            0x90, 0x20, 0x01, 0x00, 0x00, 0x13, 0x0f, 0x00, 0x00, 0x00, 0x00, 0x09, 0xc0, 0x87,
            0x6a, 0x13, 0x0b,
        ];
        let decoded = Ipv6Addr::decode(&rlp).unwrap();
        let expected = Ipv6Addr::from_str("2001:0000:130F:0000:0000:09C0:876A:130B").unwrap();
        assert_eq!(decoded, expected);
    }

    #[test]
    fn test_decode_string() {
        let rlp = vec![RLP_NULL + 3, b'd', b'o', b'g'];
        let decoded = String::decode(&rlp).unwrap();
        let expected = String::from("dog");
        assert_eq!(decoded, expected);

        let rlp = vec![RLP_NULL];
        let decoded = String::decode(&rlp).unwrap();
        let expected = String::from("");
        assert_eq!(decoded, expected);
    }
}
