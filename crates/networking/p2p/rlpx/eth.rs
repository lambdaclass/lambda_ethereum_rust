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
pub const HASH_FIRST_BYTE_DECODER: u8 = 160;

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
    pub fn new(storage: &Store) -> Result<Self, StoreError> {
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
        let decompressed_data = snappy_decoder
            .decompress_vec(msg_data)
            .map_err(|e| RLPDecodeError::Custom(e.to_string()))?;
        let decoder = Decoder::new(&decompressed_data)?;
        let (eth_version, decoder): (u32, _) = decoder.decode_field("protocolVersion")?;

        assert_eq!(eth_version, 68, "only eth version 68 is supported");

        let (network_id, decoder): (u64, _) = decoder.decode_field("networkId")?;

        let (total_difficulty, decoder): (U256, _) = decoder.decode_field("totalDifficulty")?;

        let (block_hash, decoder): (BlockHash, _) = decoder.decode_field("blockHash")?;

        let (genesis, decoder): (BlockHash, _) = decoder.decode_field("genesis")?;

        let (fork_id, decoder): (ForkId, _) = decoder.decode_field("forkId")?;

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
    fn decode_unfinished(buf: &[u8]) -> Result<(Self, &[u8]), RLPDecodeError> {
        let first_byte = buf.first().ok_or(RLPDecodeError::InvalidLength)?;
        // https://ethereum.org/en/developers/docs/data-structures-and-encoding/rlp/
        // hashes are 32 bytes long, so they enter in the 0-55 bytes range for rlp. This means the first byte
        // is the value 0x80 + len, where len = 32 (0x20). so we get the result of 0xa0 which is 160 in decimal
        if *first_byte == HASH_FIRST_BYTE_DECODER {
            let (hash, rest) = BlockHash::decode_unfinished(buf)?;
            return Ok((Self::Hash(hash), rest));
        }

        let (number, rest) = u64::decode_unfinished(buf)?;
        Ok((Self::Number(number), rest))
    }
}

// https://github.com/ethereum/devp2p/blob/master/caps/eth.md#getblockheaders-0x03
#[derive(Debug)]
pub(crate) struct GetBlockHeaders {
    // id is a u64 chosen by the requesting peer, the responding peer must mirror the value for the response
    // https://github.com/ethereum/devp2p/blob/master/caps/eth.md#protocol-messages
    id: u64,
    startblock: HashOrNumber,
    limit: u64,
    skip: u64,
    reverse: bool,
}

impl GetBlockHeaders {
    pub fn new(id: u64, startblock: HashOrNumber, limit: u64, skip: u64, reverse: bool) -> Self {
        Self {
            id,
            startblock,
            limit,
            skip,
            reverse,
        }
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

        // This unwrap shouldn't panic, compress only returns an error if we want to compress something
        // bigger than 4 GBytes, or the output len is smaller than the input len, this doesn't happen
        // as we create the buffer (msg_data) with max_compress_len + 1
        let compressed_size = snappy_encoder
            .compress(&encoded_data, &mut msg_data)
            .unwrap();

        msg_data.truncate(compressed_size);

        buf.put_slice(&msg_data);
    }

    fn decode(msg_data: &[u8]) -> Result<Self, RLPDecodeError> {
        let mut snappy_decoder = SnappyDecoder::new();
        let decompressed_data = snappy_decoder
            .decompress_vec(msg_data)
            .map_err(|e| RLPDecodeError::Custom(e.to_string()))?;
        let decoder = Decoder::new(&decompressed_data)?;
        let (id, decoder): (u64, _) = decoder.decode_field("request-id")?;
        let (startblock, decoder): (HashOrNumber, _) = decoder.decode_field("startblock")?;
        let (limit, decoder): (u64, _) = decoder.decode_field("limit")?;
        let (skip, decoder): (u64, _) = decoder.decode_field("skip")?;
        let (reverse, _): (bool, _) = decoder.decode_field("reverse")?;

        Ok(Self::new(id, startblock, limit, skip, reverse))
    }
}

// https://github.com/ethereum/devp2p/blob/master/caps/eth.md#blockheaders-0x04
pub(crate) struct BlockHeaders {
    // id is a u64 chosen by the requesting peer, the responding peer must mirror the value for the response
    // https://github.com/ethereum/devp2p/blob/master/caps/eth.md#protocol-messages
    id: u64,
    block_headers: Vec<BlockHeader>,
}

impl BlockHeaders {
    pub fn new(id: u64, block_headers: Vec<BlockHeader>) -> Self {
        Self { block_headers, id }
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
        let decompressed_data = snappy_decoder
            .decompress_vec(msg_data)
            .map_err(|e| RLPDecodeError::Custom(e.to_string()))?;
        let decoder = Decoder::new(&decompressed_data)?;
        let (id, decoder): (u64, _) = decoder.decode_field("request-id")?;
        let (block_headers, _): (Vec<BlockHeader>, _) = decoder.decode_field("headers")?;

        Ok(Self::new(id, block_headers))
    }
}

#[cfg(test)]
mod tests {
    use ethereum_rust_core::types::{Block, BlockBody, BlockHash, BlockHeader};
    use ethereum_rust_storage::{error::StoreError, Store};

    use crate::rlpx::{
        eth::{BlockHeaders, GetBlockHeaders, HashOrNumber},
        message::RLPxMessage,
    };

    fn get_block_header_from_store(
        storage: &Store,
        startblock: HashOrNumber,
        limit: u64,
        skip: u64,
        reverse: bool,
    ) -> Result<Vec<BlockHeader>, StoreError> {
        let mut block_headers = vec![];

        let first_block = match startblock {
            HashOrNumber::Hash(hash) => match storage.get_block_header_by_hash(hash)? {
                Some(header) => header,
                None => return Ok(block_headers),
            },
            HashOrNumber::Number(number) => match storage.get_block_header(number)? {
                Some(header) => header,
                None => return Ok(block_headers),
            },
        };
        // skip +1 because skip can be 0
        // if we have a skip == 0, we should expect to get the first block and the next continuos one (1, 2, 3, 4, ..., limit)
        // so if we don't add the + 1 we will be getting nothing from the loop
        let first_block_number = first_block.number;
        let headers_range = first_block_number..first_block_number + limit * (skip + 1);
        for i in headers_range.step_by((skip + 1) as usize) {
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

        Ok(block_headers)
    }

    #[test]
    fn get_block_headers_startblock_number_message() {
        let get_block_bodies = GetBlockHeaders::new(1, HashOrNumber::Number(1), 0, 0, false);

        let mut buf = Vec::new();
        get_block_bodies.encode(&mut buf);

        let decoded = GetBlockHeaders::decode(&buf).unwrap();
        assert_eq!(decoded.id, 1);
        assert_eq!(decoded.startblock, HashOrNumber::Number(1));
    }

    #[test]
    fn get_block_headers_startblock_hash_message() {
        let get_block_bodies =
            GetBlockHeaders::new(1, HashOrNumber::Hash(BlockHash::from([1; 32])), 0, 0, false);

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
        let body = BlockBody {
            transactions: vec![],
            ommers: vec![],
            withdrawals: None,
        };
        let number = 1;
        header1.number = number;
        let block1 = Block {
            header: header1.clone(),
            body,
        };
        store.add_block(block1.clone()).unwrap();
        store
            .set_canonical_block(number, header1.compute_block_hash())
            .unwrap();

        let block_headers =
            get_block_header_from_store(&store, HashOrNumber::Number(number), 1, 0, false).unwrap();
        let block_headers = BlockHeaders::new(1, block_headers);

        let mut buf = Vec::new();
        block_headers.encode(&mut buf);

        let decoded = BlockHeaders::decode(&buf).unwrap();
        assert_eq!(decoded.id, 1);
        assert_eq!(decoded.block_headers, vec![header1]);
    }

    #[test]
    fn block_headers_startblock_hash_message() {
        let store = Store::new("", ethereum_rust_storage::EngineType::InMemory).unwrap();
        let mut header1 = BlockHeader::default();
        let body = BlockBody {
            transactions: vec![],
            ommers: vec![],
            withdrawals: None,
        };
        let number = 1;
        header1.number = number;
        let block1 = Block {
            header: header1.clone(),
            body,
        };
        store.add_block(block1.clone()).unwrap();
        store
            .set_canonical_block(number, header1.compute_block_hash())
            .unwrap();

        let block_headers = get_block_header_from_store(
            &store,
            HashOrNumber::Hash(header1.compute_block_hash()),
            1,
            0,
            false,
        )
        .unwrap();
        let block_headers = BlockHeaders::new(1, block_headers);

        let mut buf = Vec::new();
        block_headers.encode(&mut buf);

        let decoded = BlockHeaders::decode(&buf).unwrap();
        assert_eq!(decoded.id, 1);
        assert_eq!(decoded.block_headers, vec![header1]);
    }

    #[test]
    fn block_headers_get_multiple_blocks() {
        let store = Store::new("", ethereum_rust_storage::EngineType::InMemory).unwrap();
        let body = BlockBody {
            transactions: vec![],
            ommers: vec![],
            withdrawals: None,
        };
        let mut header1 = BlockHeader::default();
        header1.number = 1;
        let mut header2 = BlockHeader::default();
        header2.number = 2;
        let mut header3 = BlockHeader::default();
        header3.number = 3;
        let block1 = Block {
            header: header1.clone(),
            body: body.clone(),
        };
        let block2 = Block {
            header: header2.clone(),
            body: body.clone(),
        };
        let block3 = Block {
            header: header3.clone(),
            body: body.clone(),
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

        let block_headers =
            get_block_header_from_store(&store, HashOrNumber::Number(1), 3, 0, false).unwrap();
        let block_headers = BlockHeaders::new(1, block_headers);

        let mut buf = Vec::new();
        block_headers.encode(&mut buf);

        let decoded = BlockHeaders::decode(&buf).unwrap();
        assert_eq!(decoded.id, 1);
        assert_eq!(decoded.block_headers, vec![header1, header2, header3]);
    }

    #[test]
    fn block_headers_multiple_blocks_skip_and_reverse() {
        let store = Store::new("", ethereum_rust_storage::EngineType::InMemory).unwrap();
        let body = BlockBody {
            transactions: vec![],
            ommers: vec![],
            withdrawals: None,
        };
        let mut header1 = BlockHeader::default();
        header1.number = 1;
        let mut header2 = BlockHeader::default();
        header2.number = 2;
        let mut header3 = BlockHeader::default();
        header3.number = 3;
        let block1 = Block {
            header: header1.clone(),
            body: body.clone(),
        };
        let block2 = Block {
            header: header2.clone(),
            body: body.clone(),
        };
        let block3 = Block {
            header: header3.clone(),
            body: body.clone(),
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

        let block_headers =
            get_block_header_from_store(&store, HashOrNumber::Number(1), 3, 1, true).unwrap();
        let block_headers = BlockHeaders::new(1, block_headers);
        // we should get 1, skip 2 and get 3, and it should be backwards
        let mut buf = Vec::new();
        block_headers.encode(&mut buf);

        let decoded = BlockHeaders::decode(&buf).unwrap();
        assert_eq!(decoded.id, 1);
        assert_eq!(decoded.block_headers, vec![header3, header1]);
    }

    #[test]
    fn get_block_headers_receive_block_headers() {
        let store = Store::new("", ethereum_rust_storage::EngineType::InMemory).unwrap();
        let body = BlockBody {
            transactions: vec![],
            ommers: vec![],
            withdrawals: None,
        };
        let mut header1 = BlockHeader::default();
        let mut header2 = BlockHeader::default();
        let mut header3 = BlockHeader::default();
        header1.parent_hash = BlockHash::from([0; 32]);
        header1.number = 1;
        header2.parent_hash = BlockHash::from([1; 32]);
        header2.number = 2;
        header3.parent_hash = BlockHash::from([2; 32]);
        header3.number = 3;
        let block1 = Block {
            header: header1.clone(),
            body: body.clone(),
        };
        let block2 = Block {
            header: header2.clone(),
            body: body.clone(),
        };
        let block3 = Block {
            header: header3.clone(),
            body: body.clone(),
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

        let sender_address = "127.0.0.1:3000";
        let receiver_address = "127.0.0.1:4000";
        let sender = std::net::UdpSocket::bind(sender_address).unwrap();
        let receiver = std::net::UdpSocket::bind(receiver_address).unwrap();

        let sender_chosen_id = 1;
        let get_block_headers =
            GetBlockHeaders::new(sender_chosen_id, HashOrNumber::Number(1), 3, 1, true);
        let mut send_data_of_block_headers = Vec::new();
        get_block_headers.encode(&mut send_data_of_block_headers);
        sender
            .send_to(&send_data_of_block_headers, receiver_address)
            .unwrap(); // sends the block headers request

        let mut receiver_data_of_block_headers_request = [0; 1024];
        let len = receiver
            .recv(&mut receiver_data_of_block_headers_request)
            .unwrap(); // receives the block headers request
        let received_block_header_request =
            GetBlockHeaders::decode(&receiver_data_of_block_headers_request[..len]).unwrap(); // transform the encoded received data to our struct

        assert_eq!(received_block_header_request.id, sender_chosen_id);

        let block_headers = get_block_header_from_store(
            &store,
            received_block_header_request.startblock,
            received_block_header_request.limit,
            received_block_header_request.skip,
            received_block_header_request.reverse,
        )
        .unwrap();
        let block_headers = BlockHeaders::new(received_block_header_request.id, block_headers);

        let mut block_headers_to_send = Vec::new();
        block_headers.encode(&mut block_headers_to_send); // encode the block headers that were requested
        receiver
            .send_to(&block_headers_to_send, sender_address)
            .unwrap(); // send the block bodies to the sender that requested them

        let mut received_block_headers = [0; 1024];
        let len = sender.recv(&mut received_block_headers).unwrap(); // receive the block headers
        let received_block_bodies = BlockHeaders::decode(&received_block_headers[..len]).unwrap(); // decode the received block headers

        assert_eq!(received_block_bodies.id, sender_chosen_id);
        assert_eq!(received_block_bodies.block_headers, vec![header3, header1]);
    }
}
