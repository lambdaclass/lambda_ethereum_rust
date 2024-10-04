use bytes::BufMut;
use ethereum_rust_core::{
    types::{BlockHash, ForkId},
    U256,
};
use ethereum_rust_rlp::{
    encode::RLPEncode,
    error::RLPDecodeError,
    structs::{Decoder, Encoder},
};
use ethereum_rust_storage::{error::StoreError, Store};
use snap::raw::{max_compress_len, Decoder as SnappyDecoder, Encoder as SnappyEncoder};

pub const ETH_VERSION: u32 = 68;

use super::message::RLPxMessage;

#[derive(Debug)]
pub(crate) struct StatusMessage {
    eth_version: u32,
    network_id: u64,
    total_difficulty: U256,
    block_hash: BlockHash,
    genesis: BlockHash,
    fork_id: ForkId,
}

impl StatusMessage {
    pub fn build_from(storage: &Store) -> Result<Self, StoreError> {
        let chain_config = storage.get_chain_config()?;
        let total_difficulty =
            U256::from(chain_config.terminal_total_difficulty.unwrap_or_default());
        let network_id = chain_config.chain_id;

        // These blocks must always be available
        let genesis_header = storage.get_block_header(0)?.unwrap();
        let block_number = storage.get_latest_block_number()?.unwrap();
        let block_header = storage.get_block_header(block_number)?.unwrap();

        let genesis = genesis_header.compute_block_hash();
        let block_hash = block_header.compute_block_hash();
        let fork_id = ForkId::new(chain_config, genesis, block_header.timestamp, block_number);
        Ok(Self {
            eth_version: ETH_VERSION,
            network_id,
            total_difficulty,
            block_hash,
            genesis,
            fork_id,
        })
    }
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
