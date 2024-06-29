use bytes::BufMut;

use super::{
    decode::{decode_rlp_item, RLPDecode},
    encode::{encode_length, RLPEncode},
    error::RLPDecodeError,
};

/// # Struct decoding helper
///
/// Used to decode a struct from RLP format.
/// The struct's fields must implement [`RLPDecode`].
/// The struct is expected as a list, with its values being the fields
/// in the order they are passed to [`Decoder::decode_field`].
///
/// # Examples
///
/// ```
/// # use ethrex_core::rlp::structs::Decoder;
/// # use ethrex_core::rlp::error::RLPDecodeError;
/// # use ethrex_core::rlp::decode::RLPDecode;
/// #[derive(Debug, PartialEq, Eq)]
/// struct Simple {
///     pub a: u8,
///     pub b: u16,
/// }
///
/// impl RLPDecode for Simple {
///     fn decode_unfinished(buf: &[u8]) -> Result<(Self, &[u8]), RLPDecodeError> {
///         let decoder = Decoder::new(&buf).unwrap();
///         // The fields are expected in the same order as given here
///         let (a, decoder) = decoder.decode_field("a").unwrap();
///         let (b, decoder) = decoder.decode_field("b").unwrap();
///         let rest = decoder.finish().unwrap();
///         Ok((Simple { a, b }, rest))
///     }
/// }
///
/// let bytes = [0xc2, 61, 75];
/// let decoded = Simple::decode(&bytes).unwrap();
///
/// assert_eq!(decoded, Simple { a: 61, b: 75 });
/// ```
#[derive(Debug)]
pub struct Decoder<'a> {
    payload: &'a [u8],
    remaining: &'a [u8],
}

impl<'a> Decoder<'a> {
    pub fn new(buf: &'a [u8]) -> Result<Self, RLPDecodeError> {
        match decode_rlp_item(buf)? {
            (true, payload, remaining) => Ok(Self { payload, remaining }),
            (false, _, _) => Err(RLPDecodeError::UnexpectedString),
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

/// # Struct encoding helper.
///
/// Used to encode a struct into RLP format.
/// The struct's fields must implement [`RLPEncode`].
/// The struct is encoded as a list, with its values being the fields
/// in the order they are passed to [`Encoder::encode_field`].
pub struct Encoder<'a> {
    buf: &'a mut dyn BufMut,
    temp_buf: Vec<u8>,
}

// NOTE: BufMut doesn't implement Debug, so we can't derive Debug for Encoder.
impl core::fmt::Debug for Encoder<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Encoder")
            .field("buf", &"...")
            .field("temp_buf", &self.temp_buf)
            .finish()
    }
}

impl<'a> Encoder<'a> {
    pub fn new(buf: &'a mut dyn BufMut) -> Self {
        // PERF: we could pre-allocate the buffer or switch to `ArrayVec`` if we could
        // bound the size of the encoded data.
        Self {
            buf,
            temp_buf: Default::default(),
        }
    }

    pub fn encode_field<T: RLPEncode>(&mut self, value: &T) -> &mut Self {
        <T as RLPEncode>::encode(value, &mut self.temp_buf);
        self
    }

    pub fn finish(self) {
        encode_length(self.temp_buf.len(), self.buf);
        self.buf.put_slice(&self.temp_buf);
    }
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
