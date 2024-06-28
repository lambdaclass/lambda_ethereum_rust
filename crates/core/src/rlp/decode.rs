use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use super::{
    constants::{RLP_EMPTY_LIST, RLP_NULL},
    error::RLPDecodeError,
};
use bytes::{Bytes, BytesMut};

/// Trait for decoding RLP encoded slices of data.
/// See https://ethereum.org/en/developers/docs/data-structures-and-encoding/rlp/#rlp-decoding for more information.
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

        Ok(buf[0] != RLP_NULL)
    }
}

impl RLPDecode for u8 {
    fn decode(rlp: &[u8]) -> Result<Self, RLPDecodeError> {
        if rlp.is_empty() {
            return Err(RLPDecodeError::InvalidLength);
        }

        match rlp[0] {
            // Single byte in the range [0x00, 0x7f]
            0..=0x7f => Ok(rlp[0]),

            // RLP_NULL represents zero
            RLP_NULL => Ok(0),

            // Two bytes, where the first byte is RLP_NULL + 1
            x if rlp.len() == 2 && x == RLP_NULL + 1 => Ok(rlp[1]),

            // Any other case is invalid for u8
            _ => Err(RLPDecodeError::MalformedData),
        }
    }
}

impl RLPDecode for u16 {
    fn decode(rlp: &[u8]) -> Result<Self, RLPDecodeError> {
        let (bytes, _) = decode_bytes(rlp)?;
        let padded_bytes = static_left_pad(bytes)?;
        Ok(u16::from_be_bytes(padded_bytes))
    }
}

impl RLPDecode for u32 {
    fn decode(rlp: &[u8]) -> Result<Self, RLPDecodeError> {
        let (bytes, _) = decode_bytes(rlp)?;
        let padded_bytes = static_left_pad(bytes)?;
        Ok(u32::from_be_bytes(padded_bytes))
    }
}

impl RLPDecode for u64 {
    fn decode(rlp: &[u8]) -> Result<Self, RLPDecodeError> {
        let (bytes, _) = decode_bytes(rlp)?;
        let padded_bytes = static_left_pad(bytes)?;
        Ok(u64::from_be_bytes(padded_bytes))
    }
}

impl RLPDecode for u128 {
    fn decode(rlp: &[u8]) -> Result<Self, RLPDecodeError> {
        let (bytes, _) = decode_bytes(rlp)?;
        let padded_bytes = static_left_pad(bytes)?;
        Ok(u128::from_be_bytes(padded_bytes))
    }
}

impl<const N: usize> RLPDecode for [u8; N] {
    fn decode(rlp: &[u8]) -> Result<Self, RLPDecodeError> {
        let (decoded_bytes, _) = decode_bytes(rlp)?;
        decoded_bytes
            .try_into()
            .map_err(|_| RLPDecodeError::InvalidLength)
    }
}

impl RLPDecode for Bytes {
    fn decode(rlp: &[u8]) -> Result<Self, RLPDecodeError> {
        decode_bytes(rlp).map(|decoded| Bytes::from(decoded.0.to_vec()))
    }
}

impl RLPDecode for BytesMut {
    fn decode(rlp: &[u8]) -> Result<Self, RLPDecodeError> {
        decode_bytes(rlp).map(|decoded| BytesMut::from(decoded.0))
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
        let str_bytes = decode_bytes(rlp)?.0.to_vec();
        String::from_utf8(str_bytes).map_err(|_| RLPDecodeError::MalformedData)
    }
}

impl RLPDecode for Ipv4Addr {
    fn decode(rlp: &[u8]) -> Result<Self, RLPDecodeError> {
        let (ip_bytes, _) = decode_bytes(rlp)?;
        let octets: [u8; 4] = ip_bytes
            .try_into()
            .map_err(|_| RLPDecodeError::InvalidLength)?;
        Ok(Ipv4Addr::from(octets))
    }
}

impl RLPDecode for Ipv6Addr {
    fn decode(rlp: &[u8]) -> Result<Self, RLPDecodeError> {
        let (ip_bytes, _) = decode_bytes(rlp)?;
        let octets: [u8; 16] = ip_bytes
            .try_into()
            .map_err(|_| RLPDecodeError::InvalidLength)?;
        Ok(Ipv6Addr::from(octets))
    }
}

impl RLPDecode for IpAddr {
    fn decode(rlp: &[u8]) -> Result<Self, RLPDecodeError> {
        let (ip_bytes, _) = decode_bytes(rlp)?;

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

impl RLPDecode for ethereum_types::U256 {
    fn decode(rlp: &[u8]) -> Result<Self, RLPDecodeError> {
        let (bytes, _) = decode_bytes(rlp)?;
        let padded_bytes: [u8; 32] = static_left_pad(bytes)?;
        Ok(ethereum_types::U256::from_big_endian(&padded_bytes))
    }
}

impl<T: RLPDecode> RLPDecode for Vec<T> {
    fn decode(rlp: &[u8]) -> Result<Self, RLPDecodeError> {
        if rlp.is_empty() {
            return Err(RLPDecodeError::InvalidLength);
        }

        if rlp[0] == RLP_EMPTY_LIST {
            return Ok(Vec::new());
        }

        let (is_list, payload, _) = decode_rlp_item(rlp)?;
        if !is_list {
            return Err(RLPDecodeError::MalformedData);
        }

        let mut result = Vec::new();
        let mut current_slice = payload;

        while !current_slice.is_empty() {
            let (_, _, rest) = decode_rlp_item(current_slice)?;
            let item = T::decode(current_slice)?;
            result.push(item);
            current_slice = rest;
        }

        Ok(result)
    }
}

impl<T1: RLPDecode, T2: RLPDecode> RLPDecode for (T1, T2) {
    fn decode(rlp: &[u8]) -> Result<Self, RLPDecodeError> {
        if rlp.is_empty() {
            return Err(RLPDecodeError::InvalidLength);
        }
        let (is_list, payload, _) = decode_rlp_item(rlp)?;
        if !is_list {
            return Err(RLPDecodeError::MalformedData);
        }

        let (_, first, rest) = decode_rlp_item(payload)?;
        let first = if first.is_empty() {
            T1::decode(&[RLP_EMPTY_LIST])?
        } else {
            T1::decode(payload)?
        };
        let second = T2::decode(rest)?;
        Ok((first, second))
    }
}

impl<T1: RLPDecode, T2: RLPDecode, T3: RLPDecode> RLPDecode for (T1, T2, T3) {
    fn decode(rlp: &[u8]) -> Result<Self, RLPDecodeError> {
        if rlp.is_empty() {
            return Err(RLPDecodeError::InvalidLength);
        }
        let (is_list, payload, _) = decode_rlp_item(rlp)?;
        if !is_list {
            return Err(RLPDecodeError::MalformedData);
        }

        let (_, first, first_rest) = decode_rlp_item(payload)?;
        let first_decoded = if first.is_empty() {
            T1::decode(&[RLP_EMPTY_LIST])?
        } else {
            T1::decode(payload)?
        };

        let (_, second, second_rest) = decode_rlp_item(first_rest)?;
        let second_decoded = if second.is_empty() {
            T2::decode(&[RLP_EMPTY_LIST])?
        } else {
            T2::decode(first_rest)?
        };
        let third_decoded = T3::decode(second_rest)?;

        Ok((first_decoded, second_decoded, third_decoded))
    }
}

impl<T1: RLPDecode, T2: RLPDecode, T3: RLPDecode, T4: RLPDecode> RLPDecode for (T1, T2, T3, T4) {
    fn decode(rlp: &[u8]) -> Result<Self, RLPDecodeError> {
        if rlp.is_empty() {
            return Err(RLPDecodeError::InvalidLength);
        }
        let (is_list, payload, _) = decode_rlp_item(rlp)?;
        if !is_list {
            return Err(RLPDecodeError::MalformedData);
        }

        let (_, first, first_rest) = decode_rlp_item(payload)?;
        let first_decoded = if first.is_empty() {
            T1::decode(&[RLP_EMPTY_LIST])?
        } else {
            T1::decode(payload)?
        };

        let (_, second, second_rest) = decode_rlp_item(first_rest)?;
        let second_decoded = if second.is_empty() {
            T2::decode(&[RLP_EMPTY_LIST])?
        } else {
            T2::decode(first_rest)?
        };

        let (_, third, third_rest) = decode_rlp_item(first_rest)?;
        let third_decoded = if third.is_empty() {
            T3::decode(&[RLP_EMPTY_LIST])?
        } else {
            T3::decode(second_rest)?
        };

        let fourth_decoded = T4::decode(third_rest)?;

        Ok((first_decoded, second_decoded, third_decoded, fourth_decoded))
    }
}

fn decode_rlp_item(data: &[u8]) -> Result<(bool, &[u8], &[u8]), RLPDecodeError> {
    if data.is_empty() {
        return Err(RLPDecodeError::InvalidLength);
    }

    let first_byte = data[0];

    match first_byte {
        0..=0x7F => Ok((false, &data[..1], &data[1..])),
        0x80..=0xB7 => {
            let length = (first_byte - 0x80) as usize;
            if data.len() < length + 1 {
                return Err(RLPDecodeError::InvalidLength);
            }
            Ok((false, &data[1..length + 1], &data[length + 1..]))
        }
        0xB8..=0xBF => {
            let length_of_length = (first_byte - 0xB7) as usize;
            if data.len() < length_of_length + 1 {
                return Err(RLPDecodeError::InvalidLength);
            }
            let length_bytes = &data[1..length_of_length + 1];
            let length = usize::from_be_bytes(static_left_pad(length_bytes)?);
            if data.len() < length_of_length + length + 1 {
                return Err(RLPDecodeError::InvalidLength);
            }
            Ok((
                false,
                &data[length_of_length + 1..length_of_length + length + 1],
                &data[length_of_length + length + 1..],
            ))
        }
        RLP_EMPTY_LIST..=0xF7 => {
            let length = (first_byte - RLP_EMPTY_LIST) as usize;
            if data.len() < length + 1 {
                return Err(RLPDecodeError::InvalidLength);
            }
            Ok((true, &data[1..length + 1], &data[length + 1..]))
        }
        0xF8..=0xFF => {
            let list_length = (first_byte - 0xF7) as usize;
            if data.len() < list_length + 1 {
                return Err(RLPDecodeError::InvalidLength);
            }
            let length_bytes = &data[1..list_length + 1];
            let payload_length = usize::from_be_bytes(static_left_pad(length_bytes)?);
            if data.len() < list_length + payload_length + 1 {
                return Err(RLPDecodeError::InvalidLength);
            }
            Ok((
                true,
                &data[list_length + 1..list_length + payload_length + 1],
                &data[list_length + payload_length + 1..],
            ))
        }
    }
}

fn decode_bytes(data: &[u8]) -> Result<(&[u8], &[u8]), RLPDecodeError> {
    if data.is_empty() {
        return Err(RLPDecodeError::InvalidLength);
    }

    let first_byte = data[0];

    match first_byte {
        0..=0x7F => Ok((&data[..1], &data[1..])),
        0x80..=0xB7 => {
            let length = (first_byte - 0x80) as usize;
            if data.len() < length + 1 {
                return Err(RLPDecodeError::InvalidLength);
            }
            Ok((&data[1..length + 1], &data[length + 1..]))
        }
        0xB8..=0xBF => {
            let length_of_length = (first_byte - 0xB7) as usize;
            if data.len() < length_of_length + 1 {
                return Err(RLPDecodeError::InvalidLength);
            }
            let length_bytes = &data[1..length_of_length + 1];
            let length = usize::from_be_bytes(static_left_pad(length_bytes)?);
            if data.len() < length_of_length + length + 1 {
                return Err(RLPDecodeError::InvalidLength);
            }
            Ok((
                &data[length_of_length + 1..length_of_length + length + 1],
                &data[length_of_length + length + 1..],
            ))
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
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_decode_bool() {
        let rlp = vec![0x01];
        let decoded = bool::decode(&rlp).unwrap();
        assert!(decoded);

        let rlp = vec![RLP_NULL];
        let decoded = bool::decode(&rlp).unwrap();
        assert!(!decoded);
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
    fn test_decode_u256() {
        let rlp = vec![RLP_NULL + 1, 0x01];
        let decoded = ethereum_types::U256::decode(&rlp).unwrap();
        let expected = ethereum_types::U256::from(1);
        assert_eq!(decoded, expected);

        let mut rlp = vec![RLP_NULL + 32];
        let number_bytes = [0x01; 32];
        rlp.extend(number_bytes);
        let decoded = ethereum_types::U256::decode(&rlp).unwrap();
        let expected = ethereum_types::U256::from_big_endian(&number_bytes);
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

    #[test]
    fn test_decode_lists() {
        // empty list
        let rlp = vec![RLP_EMPTY_LIST];
        let decoded: Vec<String> = Vec::decode(&rlp).unwrap();
        let expected: Vec<String> = vec![];
        assert_eq!(decoded, expected);

        //  list with a single number
        let rlp = vec![RLP_EMPTY_LIST + 1, 0x01];
        let decoded: Vec<u8> = Vec::decode(&rlp).unwrap();
        let expected = vec![1];
        assert_eq!(decoded, expected);

        // list with 3 numbers
        let rlp = vec![RLP_EMPTY_LIST + 3, 0x01, 0x02, 0x03];
        let decoded: Vec<u8> = Vec::decode(&rlp).unwrap();
        let expected = vec![1, 2, 3];
        assert_eq!(decoded, expected);

        // list of strings
        let rlp = vec![0xc8, 0x83, b'c', b'a', b't', 0x83, b'd', b'o', b'g'];
        let decoded: Vec<String> = Vec::decode(&rlp).unwrap();
        let expected = vec!["cat".to_string(), "dog".to_string()];
        assert_eq!(decoded, expected);
    }

    #[test]
    fn test_decode_list_of_lists() {
        let rlp = vec![
            0xd2, 0xc8, 0x83, b'c', b'a', b't', 0x83, b'd', b'o', b'g', 0xc8, 0x83, b'f', b'o',
            b'o', 0x83, b'b', b'a', b'r',
        ];
        let decoded: Vec<Vec<String>> = Vec::decode(&rlp).unwrap();
        let expected = vec![
            vec!["cat".to_string(), "dog".to_string()],
            vec!["foo".to_string(), "bar".to_string()],
        ];
        assert_eq!(decoded, expected);
    }

    #[test]
    fn test_decode_tuples() {
        // tuple with numbers
        let rlp = vec![RLP_EMPTY_LIST + 2, 0x01, 0x02];
        let decoded: (u8, u8) = <(u8, u8)>::decode(&rlp).unwrap();
        let expected = (1, 2);
        assert_eq!(decoded, expected);

        // tuple with string and number
        let rlp = vec![RLP_EMPTY_LIST + 5, 0x01, 0x83, b'c', b'a', b't'];
        let decoded: (u8, String) = <(u8, String)>::decode(&rlp).unwrap();
        let expected = (1, "cat".to_string());
        assert_eq!(decoded, expected);

        // tuple with bool and string
        let rlp = vec![RLP_EMPTY_LIST + 6, 0x01, 0x84, b't', b'r', b'u', b'e'];
        let decoded: (bool, String) = <(bool, String)>::decode(&rlp).unwrap();
        let expected = (true, "true".to_string());
        assert_eq!(decoded, expected);

        // tuple with list and number
        let rlp = vec![RLP_EMPTY_LIST + 2, RLP_EMPTY_LIST, 0x03];
        let decoded = <(Vec<u8>, u8)>::decode(&rlp).unwrap();
        let expected = (vec![], 3);
        assert_eq!(decoded, expected);

        // tuple with number and list
        let rlp = vec![RLP_EMPTY_LIST + 2, 0x03, RLP_EMPTY_LIST];
        let decoded = <(u8, Vec<u8>)>::decode(&rlp).unwrap();
        let expected = (3, vec![]);
        assert_eq!(decoded, expected);

        // tuple with tuples
        let rlp = vec![
            RLP_EMPTY_LIST + 6,
            RLP_EMPTY_LIST + 2,
            0x01,
            0x02,
            RLP_EMPTY_LIST + 2,
            0x03,
            0x04,
        ];
        let decoded = <((u8, u8), (u8, u8))>::decode(&rlp).unwrap();
        let expected = ((1, 2), (3, 4));
        assert_eq!(decoded, expected);
    }

    #[test]
    fn test_decode_tuples_3_elements() {
        // tuple with numbers
        let rlp = vec![RLP_EMPTY_LIST + 3, 0x01, 0x02, 0x03];
        let decoded: (u8, u8, u8) = <(u8, u8, u8)>::decode(&rlp).unwrap();
        let expected = (1, 2, 3);
        assert_eq!(decoded, expected);

        // tuple with string and number
        let rlp = vec![RLP_EMPTY_LIST + 6, 0x01, 0x02, 0x83, b'c', b'a', b't'];
        let decoded: (u8, u8, String) = <(u8, u8, String)>::decode(&rlp).unwrap();
        let expected = (1, 2, "cat".to_string());
        assert_eq!(decoded, expected);

        // tuple with bool and string
        let rlp = vec![RLP_EMPTY_LIST + 7, 0x01, 0x02, 0x84, b't', b'r', b'u', b'e'];
        let decoded: (u8, u8, String) = <(u8, u8, String)>::decode(&rlp).unwrap();
        let expected = (1, 2, "true".to_string());
        assert_eq!(decoded, expected);

        // tuple with tuples
        let rlp = vec![
            RLP_EMPTY_LIST + 9,
            RLP_EMPTY_LIST + 2,
            0x01,
            0x02,
            RLP_EMPTY_LIST + 2,
            0x03,
            0x04,
            RLP_EMPTY_LIST + 2,
            0x05,
            0x06,
        ];
        let decoded = <((u8, u8), (u8, u8), (u8, u8))>::decode(&rlp).unwrap();
        let expected = ((1, 2), (3, 4), (5, 6));
        assert_eq!(decoded, expected);
    }
}
