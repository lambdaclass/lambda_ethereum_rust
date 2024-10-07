use bytes::BufMut;
use ethereum_rust_core::{
    types::{BlockHash, BlockHeader, BlockNumber, ForkId},
    U256,
};
use ethereum_rust_rlp::{
    decode::RLPDecode,
    encode::RLPEncode,
    error::RLPDecodeError,
    structs::{Decoder, Encoder},
};
use ethereum_rust_storage::{error::StoreError, Store};
use snap::raw::{max_compress_len, Decoder as SnappyDecoder, Encoder as SnappyEncoder};

pub const ETH_VERSION: u32 = 68;
pub const MAX_NUMBER_OF_HEADERS_TO_SEND: u64 = 20;

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

#[derive(Debug, PartialEq, Eq)]
pub enum HashOrNumber {
    Hash(BlockHash),
    Number(BlockNumber),
}

impl RLPEncode for HashOrNumber {
    fn encode(&self, buf: &mut dyn BufMut) {
        match self {
            HashOrNumber::Hash(hash) => hash.encode(buf),
            HashOrNumber::Number(number) => number.encode(buf),
        }
    }

    fn length(&self) -> usize {
        match self {
            HashOrNumber::Hash(hash) => hash.length(),
            HashOrNumber::Number(number) => number.length(),
        }
    }
}

impl RLPDecode for HashOrNumber {
    #[inline(always)]
    fn decode_unfinished(buf: &[u8]) -> Result<(Self, &[u8]), RLPDecodeError> {
        let first_byte = *buf.get(0).ok_or(RLPDecodeError::InvalidLength)?;
        // after some tests, seems that the first byte is always 160 for hashes
        if first_byte == 160 {
            let (hash, rest) = BlockHash::decode_unfinished(buf)?;
            return Ok((Self::Hash(hash), rest));
        }

        let (number, rest) = u64::decode_unfinished(buf)?;
        Ok((Self::Number(number), rest))
    }
}

#[derive(Debug)]
pub(crate) struct GetBlockHeaders {
    // id is a u64 chosen by the requesting peer, the responding peer must mirror the value for the response
    // https://github.com/ethereum/devp2p/blob/master/caps/eth.md#protocol-messages
    id: u64,
    // https://github.com/ethereum/devp2p/blob/master/caps/eth.md#getblockheaders-0x03
    startblock: HashOrNumber,
    limit: u64,
    skip: u64,
    reverse: bool,
}

impl GetBlockHeaders {
    pub fn build_from(
        id: u64,
        startblock: HashOrNumber,
        limit: u64,
        skip: u64,
        reverse: bool,
    ) -> Result<Self, StoreError> {
        Ok(Self {
            id,
            startblock,
            limit,
            skip,
            reverse,
        })
    }
}

impl RLPxMessage for GetBlockHeaders {
    fn encode(&self, buf: &mut dyn BufMut) {
        let mut encoded_data = vec![];
        Encoder::new(&mut encoded_data)
            .encode_field(&self.id)
            .encode_field(&self.startblock)
            .encode_field(&self.limit)
            .encode_field(&self.skip)
            .encode_field(&self.reverse)
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
        let (id, decoder): (u64, _) = decoder.decode_field("request-id").unwrap();
        let (startblock, decoder): (HashOrNumber, _) = decoder.decode_field("startblock").unwrap();
        let (limit, decoder): (u64, _) = decoder.decode_field("limit").unwrap();
        let (skip, decoder): (u64, _) = decoder.decode_field("skip").unwrap();
        let (reverse, _): (bool, _) = decoder.decode_field("reverse").unwrap();

        Ok(Self {
            id,
            startblock,
            limit,
            skip,
            reverse,
        })
    }
}

pub(crate) struct BlockHeaders {
    // id is a u64 chosen by the requesting peer, the responding peer must mirror the value for the response
    // https://github.com/ethereum/devp2p/blob/master/caps/eth.md#protocol-messages
    id: u64,
    block_headers: Vec<BlockHeader>,
}

impl BlockHeaders {
    pub fn build_from(
        id: u64,
        storage: &Store,
        startblock: HashOrNumber,
        limit: u64,
        skip: u64,
        reverse: bool,
    ) -> Result<Self, StoreError> {
        let mut block_headers = vec![];

        // how should we get the next block starting from here?
        let first_block = match startblock {
            // TODO: couldn't find what to do if the block is not found
            HashOrNumber::Hash(hash) => storage.get_block_header_by_hash(hash)?.unwrap(),
            HashOrNumber::Number(number) => storage.get_block_header(number)?.unwrap(),
        };

        let number = first_block.number;
        for i in number..number + MAX_NUMBER_OF_HEADERS_TO_SEND {
            let header = storage.get_block_header(i)?;
            match header {
                Some(header) => {
                    block_headers.push(header);
                }
                None => break,
            }
        }

        if reverse {
            block_headers.reverse();
        }

        Ok(Self { block_headers, id })
    }
}

impl RLPxMessage for BlockHeaders {
    fn encode(&self, buf: &mut dyn BufMut) {
        let mut encoded_data = vec![];
        Encoder::new(&mut encoded_data)
            .encode_field(&self.id)
            .encode_field(&self.block_headers)
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
        let (id, decoder): (u64, _) = decoder.decode_field("request-id").unwrap();
        let (block_headers, _): (Vec<BlockHeader>, _) = decoder.decode_field("headers").unwrap();

        Ok(Self { block_headers, id })
    }
}

#[cfg(test)]
mod tests {
    use ethereum_rust_core::types::{Block, BlockHash, BlockHeader};
    use ethereum_rust_storage::Store;

    use crate::rlpx::{
        eth::{BlockHeaders, GetBlockHeaders, HashOrNumber},
        message::RLPxMessage,
    };

    #[test]
    fn get_block_headers_startblock_number_message() {
        let get_block_bodies =
            GetBlockHeaders::build_from(1, HashOrNumber::Number(1), 0, 0, false).unwrap();

        let mut buf = Vec::new();
        get_block_bodies.encode(&mut buf);

        let decoded = GetBlockHeaders::decode(&buf).unwrap();
        assert_eq!(decoded.id, 1);
        assert_eq!(decoded.startblock, HashOrNumber::Number(1));
    }

    #[test]
    fn get_block_headers_startblock_hash_message() {
        let get_block_bodies = GetBlockHeaders::build_from(
            1,
            HashOrNumber::Hash(BlockHash::from([1; 32])),
            0,
            0,
            false,
        )
        .unwrap();

        let mut buf = Vec::new();
        get_block_bodies.encode(&mut buf);

        let decoded = GetBlockHeaders::decode(&buf).unwrap();
        assert_eq!(decoded.id, 1);
        assert_eq!(
            decoded.startblock,
            HashOrNumber::Hash(BlockHash::from([1; 32]))
        );
    }

    #[test]
    fn block_headers_startblock_number_message() {
        let store = Store::new("", ethereum_rust_storage::EngineType::InMemory).unwrap();
        let mut header1 = BlockHeader::default();
        let number = 1;
        header1.number = number;
        let block1 = Block {
            header: header1.clone(),
            body: Default::default(),
        };
        store.add_block(block1.clone()).unwrap();
        store
            .set_canonical_block(number, header1.compute_block_hash())
            .unwrap();

        let block_bodies =
            BlockHeaders::build_from(1, &store, HashOrNumber::Number(number), 0, 0, false).unwrap();

        let mut buf = Vec::new();
        block_bodies.encode(&mut buf);

        let decoded = BlockHeaders::decode(&buf).unwrap();
        assert_eq!(decoded.id, 1);
        assert_eq!(decoded.block_headers, vec![header1]);
    }

    #[test]
    fn block_headers_get_multiple_blocks() {
        let store = Store::new("", ethereum_rust_storage::EngineType::InMemory).unwrap();
        let mut header1 = BlockHeader::default();
        header1.number = 1;
        let mut header2 = BlockHeader::default();
        header2.number = 2;
        let mut header3 = BlockHeader::default();
        header3.number = 3;
        let block1 = Block {
            header: header1.clone(),
            body: Default::default(),
        };
        let block2 = Block {
            header: header2.clone(),
            body: Default::default(),
        };
        let block3 = Block {
            header: header3.clone(),
            body: Default::default(),
        };
        store.add_block(block1.clone()).unwrap();
        store.add_block(block2.clone()).unwrap();
        store.add_block(block3.clone()).unwrap();

        store
            .set_canonical_block(1, header1.compute_block_hash())
            .unwrap();
        store
            .set_canonical_block(2, header2.compute_block_hash())
            .unwrap();
        store
            .set_canonical_block(3, header3.compute_block_hash())
            .unwrap();

        let block_bodies =
            BlockHeaders::build_from(1, &store, HashOrNumber::Number(1), 0, 0, false).unwrap();

        let mut buf = Vec::new();
        block_bodies.encode(&mut buf);

        let decoded = BlockHeaders::decode(&buf).unwrap();
        assert_eq!(decoded.id, 1);
        assert_eq!(decoded.block_headers, vec![header1, header2, header3]);
    }
}
