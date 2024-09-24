use bytes::BufMut;
use ethereum_rust_core::{
    types::{BlockHash, BlockNumber},
    H32, U256,
};
use ethereum_rust_rlp::{
    decode::RLPDecode,
    encode::RLPEncode,
    error::RLPDecodeError,
    structs::{Decoder, Encoder},
};
use snap::raw::{max_compress_len, Decoder as SnappyDecoder, Encoder as SnappyEncoder};

use super::message::RLPxMessage;

// TODO: Find a better place for this. Maybe core types.
#[derive(Debug)]
pub struct ForkId {
    fork_hash: H32,
    fork_next: BlockNumber,
}

impl RLPEncode for ForkId {
    fn encode(&self, buf: &mut dyn bytes::BufMut) {
        Encoder::new(buf)
            .encode_field(&self.fork_hash)
            .encode_field(&self.fork_next)
            .finish();
    }
}

impl RLPDecode for ForkId {
    fn decode_unfinished(rlp: &[u8]) -> Result<(Self, &[u8]), RLPDecodeError> {
        let decoder = Decoder::new(rlp)?;
        let (fork_hash, decoder) = decoder.decode_field("forkHash")?;
        let (fork_next, decoder) = decoder.decode_field("forkNext")?;
        let remaining = decoder.finish()?;
        let fork_id = ForkId {
            fork_hash,
            fork_next,
        };
        Ok((fork_id, remaining))
    }
}

#[derive(Debug)]
pub(crate) struct StatusMessage {
    eth_version: u32,
    network_id: u64,
    total_difficulty: U256,
    block_hash: BlockHash,
    genesis: BlockHash,
    fork_id: ForkId,
}

impl RLPxMessage for StatusMessage {
    fn encode(&self, buf: &mut dyn BufMut) {
        16_u8.encode(buf); // msg_id

        let mut encoded_data = vec![];
        Encoder::new(&mut encoded_data)
            .encode_field(&self.eth_version)
            .encode_field(&self.network_id)
            .encode_field(&self.total_difficulty)
            .encode_field(&self.block_hash)
            .encode_field(&self.genesis)
            .encode_field(&self.fork_id)
            .finish();

        let mut snappy_encoder = SnappyEncoder::new();
        let mut msg_data = vec![0; max_compress_len(encoded_data.len()) + 1];

        let compressed_size = snappy_encoder
            .compress(&encoded_data, &mut msg_data)
            .unwrap();

        msg_data.truncate(compressed_size);

        buf.put_slice(&msg_data);
    }

    fn decode(msg_data: &[u8]) -> Result<Self, RLPDecodeError> {
        let mut snappy_decoder = SnappyDecoder::new();
        let decompressed_data = snappy_decoder.decompress_vec(msg_data).unwrap();
        let decoder = Decoder::new(&decompressed_data)?;
        let (eth_version, decoder): (u32, _) = decoder.decode_field("protocolVersion").unwrap();

        assert_eq!(eth_version, 68, "only eth version 68 is supported");

        let (network_id, decoder): (u64, _) = decoder.decode_field("networkId").unwrap();

        let (total_difficulty, decoder): (U256, _) =
            decoder.decode_field("totalDifficulty").unwrap();

        let (block_hash, decoder): (BlockHash, _) = decoder.decode_field("blockHash").unwrap();

        let (genesis, decoder): (BlockHash, _) = decoder.decode_field("genesis").unwrap();

        let (fork_id, decoder): (ForkId, _) = decoder.decode_field("forkId").unwrap();

        // Implementations must ignore any additional list elements
        let _padding = decoder.finish_unchecked();

        Ok(Self {
            eth_version,
            network_id,
            total_difficulty,
            block_hash,
            genesis,
            fork_id,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use ethereum_rust_core::H32;
    use ethereum_rust_rlp::encode::RLPEncode;
    use hex_literal::hex;

    use super::ForkId;

    #[test]
    fn encode_fork_id() {
        let fork = ForkId {
            fork_hash: H32::zero(),
            fork_next: 0,
        };
        let expexted = hex!("c6840000000080");
        assert_eq!(fork.encode_to_vec(), expexted);
    }
    #[test]
    fn encode_fork_id2() {
        let fork = ForkId {
            fork_hash: H32::from_str("0xdeadbeef").unwrap(),
            fork_next: u64::from_str_radix("baddcafe", 16).unwrap(),
        };
        let expexted = hex!("ca84deadbeef84baddcafe");
        assert_eq!(fork.encode_to_vec(), expexted);
    }
    #[test]
    fn encode_fork_id3() {
        let fork = ForkId {
            fork_hash: H32::from_low_u64_le(u32::MAX.into()),
            fork_next: u64::MAX,
        };
        let expexted = hex!("ce84ffffffff88ffffffffffffffff");
        assert_eq!(fork.encode_to_vec(), expexted);
    }
}
