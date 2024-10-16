use std::ops::AddAssign;

use crate::serde_utils;
use ethereum_rust_rlp::{
    decode::RLPDecode,
    encode::RLPEncode,
    error::RLPDecodeError,
    structs::{Decoder, Encoder},
};
use serde::{Deserialize, Serialize};

use super::BYTES_PER_BLOB;

pub type Bytes48 = [u8; 48];
pub type Blob = [u8; BYTES_PER_BLOB];
pub type Commitment = Bytes48;
pub type Proof = Bytes48;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
/// Struct containing all the blobs for a blob transaction, along with the corresponding commitments and proofs
pub struct BlobsBundle {
    #[serde(with = "serde_utils::blob::vec")]
    pub blobs: Vec<Blob>,
    #[serde(with = "serde_utils::bytes48::vec")]
    pub commitments: Vec<Commitment>,
    #[serde(with = "serde_utils::bytes48::vec")]
    pub proofs: Vec<Proof>,
}

impl RLPEncode for BlobsBundle {
    fn encode(&self, buf: &mut dyn bytes::BufMut) {
        let encoder = Encoder::new(buf);
        encoder
            .encode_field(&self.blobs)
            .encode_field(&self.commitments)
            .encode_field(&self.proofs)
            .finish();
    }
}

impl RLPDecode for BlobsBundle {
    fn decode_unfinished(rlp: &[u8]) -> Result<(Self, &[u8]), RLPDecodeError> {
        let decoder = Decoder::new(rlp)?;
        let (blobs, decoder) = decoder.decode_field("blobs")?;
        let (commitments, decoder) = decoder.decode_field("commitments")?;
        let (proofs, decoder) = decoder.decode_field("proofs")?;
        Ok((
            Self {
                blobs,
                commitments,
                proofs,
            },
            decoder.finish()?,
        ))
    }
}

impl AddAssign for BlobsBundle {
    fn add_assign(&mut self, rhs: Self) {
        self.blobs.extend_from_slice(&rhs.blobs);
        self.commitments.extend_from_slice(&rhs.commitments);
        self.proofs.extend_from_slice(&rhs.proofs);
    }
}
