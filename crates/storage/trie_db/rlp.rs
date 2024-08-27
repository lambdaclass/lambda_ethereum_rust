use ethereum_rust_core::rlp::{
    decode::RLPDecode,
    encode::RLPEncode,
    error::RLPDecodeError,
    structs::{Decoder, Encoder},
};

use super::node::{BranchNode, ExtensionNode, LeafNode};

impl RLPEncode for BranchNode {
    fn encode(&self, buf: &mut dyn bytes::BufMut) {
        // TODO: choices encoded as vec due to conflicting trait impls for [T;N] & [u8;N], check if we can fix this later
        Encoder::new(buf)
            .encode_field(&self.hash)
            .encode_field(&self.choices.to_vec())
            .finish()
    }
}

impl RLPEncode for ExtensionNode {
    fn encode(&self, buf: &mut dyn bytes::BufMut) {
        Encoder::new(buf)
            .encode_field(&self.hash)
            .encode_field(&self.prefix)
            .encode_field(&self.child)
            .finish()
    }
}

impl RLPEncode for LeafNode {
    fn encode(&self, buf: &mut dyn bytes::BufMut) {
        Encoder::new(buf)
            .encode_field(&self.hash)
            .encode_field(&self.value)
            .finish()
    }
}

impl RLPDecode for BranchNode {
    fn decode_unfinished(rlp: &[u8]) -> Result<(Self, &[u8]), RLPDecodeError> {
        const CHOICES_LEN_ERROR_MSG: &str =
            "Error decoding field 'choices' of type [H256;16]: Invalid Length";
        let decoder = Decoder::new(rlp)?;
        let (hash, decoder) = decoder.decode_field("hash")?;
        let (choices, decoder) = decoder.decode_field::<Vec<_>>("choices")?;
        let choices = choices
            .try_into()
            .map_err(|_| RLPDecodeError::Custom(CHOICES_LEN_ERROR_MSG.to_string()))?;
        Ok((Self { hash, choices }, decoder.finish()?))
    }
}

impl RLPDecode for ExtensionNode {
    fn decode_unfinished(rlp: &[u8]) -> Result<(Self, &[u8]), RLPDecodeError> {
        let decoder = Decoder::new(rlp)?;
        let (hash, decoder) = decoder.decode_field("hash")?;
        let (prefix, decoder) = decoder.decode_field("prefix")?;
        let (child, decoder) = decoder.decode_field("child")?;
        Ok((
            Self {
                hash,
                prefix,
                child,
            },
            decoder.finish()?,
        ))
    }
}

impl RLPDecode for LeafNode {
    fn decode_unfinished(rlp: &[u8]) -> Result<(Self, &[u8]), RLPDecodeError> {
        let decoder = Decoder::new(rlp)?;
        let (hash, decoder) = decoder.decode_field("hash")?;
        let (value, decoder) = decoder.decode_field("value")?;
        Ok((Self { hash, value }, decoder.finish()?))
    }
}
