use super::{
    decode::{decode_rlp_item, RLPDecode},
    error::RLPDecodeError,
};

#[derive(Debug)]
pub struct Decoder<'a> {
    list: &'a [u8],
    rest: &'a [u8],
}

impl<'a> Decoder<'a> {
    pub fn new(buf: &'a [u8]) -> Result<Self, RLPDecodeError> {
        match decode_rlp_item(buf)? {
            (true, list, rest) => Ok(Self { list, rest }),
            (false, _, _) => Err(RLPDecodeError::UnexpectedList),
        }
    }

    pub fn decode_field<T: RLPDecode>(self, name: &str) -> Result<(T, Self), RLPDecodeError> {
        let (field, rest) = <T as RLPDecode>::decode_unfinished(self.rest)
            .map_err(|err| field_decode_error::<T>(name, err))?;
        Ok((field, Self { list: rest, ..self }))
    }

    pub fn finish(self) -> Result<&'a [u8], RLPDecodeError> {
        if self.list.is_empty() {
            Ok(self.rest)
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
