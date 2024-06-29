use super::{
    decode::{decode_rlp_item, RLPDecode},
    error::RLPDecodeError,
};

/// # Struct decoding helper.
///
/// Used to decode a struct from RLP format.
/// The struct's fields must implement [`RLPDecode`].
/// The struct is expected as a list, with its values being the fields
/// in the order they are passed to [`Decoder::decode_field`].
#[derive(Debug)]
pub struct Decoder<'a> {
    payload: &'a [u8],
    remaining: &'a [u8],
}

impl<'a> Decoder<'a> {
    pub fn new(buf: &'a [u8]) -> Result<Self, RLPDecodeError> {
        match decode_rlp_item(buf)? {
            (true, payload, remaining) => Ok(Self { payload, remaining }),
            (false, _, _) => Err(RLPDecodeError::UnexpectedList),
        }
    }

    pub fn decode_field<T: RLPDecode>(self, name: &str) -> Result<(T, Self), RLPDecodeError> {
        let (field, rest) = <T as RLPDecode>::decode_unfinished(self.payload)
            .map_err(|err| field_decode_error::<T>(name, err))?;
        let updated_self = Self {
            payload: rest,
            ..self
        };
        Ok((field, updated_self))
    }

    pub fn finish(self) -> Result<&'a [u8], RLPDecodeError> {
        if self.payload.is_empty() {
            Ok(self.remaining)
        } else {
            Err(RLPDecodeError::MalformedData)
        }
    }
}

fn field_decode_error<T>(field_name: &str, err: RLPDecodeError) -> RLPDecodeError {
    let typ = std::any::type_name::<T>();
    let err_msg = format!("Error decoding field '{field_name}' of type {typ}: {err}");
    RLPDecodeError::Custom(err_msg)
}

#[cfg(test)]
mod tests {
    use crate::rlp::{encode::RLPEncode, structs::Decoder};

    #[test]
    fn test_decoder_simple_struct() {
        #[derive(Debug, PartialEq, Eq)]
        struct Simple {
            pub a: u8,
            pub b: u16,
        }
        let expected = Simple { a: 61, b: 75 };
        let mut buf = Vec::new();
        (expected.a, expected.b).encode(&mut buf);
        let decoder = Decoder::new(&buf).unwrap();

        let (a, decoder) = decoder.decode_field("a").unwrap();
        let (b, decoder) = decoder.decode_field("b").unwrap();
        let rest = decoder.finish().unwrap();

        assert!(rest.is_empty());
        let got = Simple { a, b };
        assert_eq!(got, expected);
    }
}
