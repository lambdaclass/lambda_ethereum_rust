use bytes::BufMut;
use ethereum_rust_core::types::{BlockBody, BlockHash, BlockHeader, BlockNumber};
use ethereum_rust_rlp::{
    decode::RLPDecode,
    encode::RLPEncode,
    error::{RLPDecodeError, RLPEncodeError},
    structs::{Decoder, Encoder},
};
use ethereum_rust_storage::Store;
use snap::raw::Decoder as SnappyDecoder;

use crate::rlpx::{message::RLPxMessage, utils::snappy_encode};

pub const HASH_FIRST_BYTE_DECODER: u8 = 160;

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
    // FIXME: Maybe make these private again?
    pub id: u64,
    pub startblock: HashOrNumber,
    pub limit: u64,
    pub skip: u64,
    pub reverse: bool,
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
    pub fn fetch_headers(&self, storage: &Store) -> Vec<BlockHeader> {
        let start_block = match self.startblock {
            // Check we have the given block hash and fetch its number
            HashOrNumber::Hash(block_hash) => storage.get_block_number(block_hash).ok().flatten(),
            // Don't check if the block number is available
            // because if it it's not, then loop below will
            // break early and return an empty vec.
            HashOrNumber::Number(block_num) => Some(block_num),
        };
        let mut headers = vec![];

        if let Some(start_block) = start_block {
            let mut current_block = start_block as i64;
            let block_skip = if self.reverse {
                -((self.skip + 1) as i64)
            } else {
                (self.skip + 1) as i64
            };
            // Limit taken from here: https://github.com/ethereum/go-ethereum/blob/20bf543a64d7c2a590b18a1e1d907cae65707013/eth/protocols/eth/handler.go#L40
            let limit = if self.limit > 1024 { 1024 } else { self.limit };
            for _ in 0..limit {
                let Some(block_header) = storage
                    .get_block_header(current_block as u64)
                    .ok()
                    .flatten()
                else {
                    break;
                };
                headers.push(block_header);
                current_block += block_skip
            }
        }
        return headers;
    }
}

impl RLPxMessage for GetBlockHeaders {
    fn encode(&self, buf: &mut dyn BufMut) -> Result<(), RLPEncodeError> {
        let mut encoded_data = vec![];
        Encoder::new(&mut encoded_data)
            .encode_field(&self.id)
            .encode_field(&self.startblock)
            .encode_field(&self.limit)
            .encode_field(&self.skip)
            .encode_field(&self.reverse)
            .finish();
        let msg_data = snappy_encode(encoded_data)?;
        buf.put_slice(&msg_data);
        Ok(())
    }

    fn decode(msg_data: &[u8]) -> Result<Self, RLPDecodeError> {
        let mut snappy_decoder = SnappyDecoder::new();
        let decompressed_data = snappy_decoder
            .decompress_vec(msg_data)
            .map_err(|e| RLPDecodeError::Custom(e.to_string()))?;
        let decoder = Decoder::new(&decompressed_data)?;
        let (id, decoder): (u64, _) = decoder.decode_field("request-id")?;
        let ((start_block, limit, skip, reverse), _): ((HashOrNumber, u64, u64, bool), _) =
            decoder.decode_field("get headers request params")?;
        Ok(Self::new(id, start_block, limit, skip, reverse))
    }
}

// https://github.com/ethereum/devp2p/blob/master/caps/eth.md#blockheaders-0x04
#[derive(Debug)]
pub(crate) struct BlockHeaders {
    // id is a u64 chosen by the requesting peer, the responding peer must mirror the value for the response
    // https://github.com/ethereum/devp2p/blob/master/caps/eth.md#protocol-messages
    pub id: u64,
    pub block_headers: Vec<BlockHeader>,
}

impl BlockHeaders {
    pub fn new(id: u64, block_headers: Vec<BlockHeader>) -> Self {
        Self { block_headers, id }
    }
}

impl RLPxMessage for BlockHeaders {
    fn encode(&self, buf: &mut dyn BufMut) -> Result<(), RLPEncodeError> {
        let mut encoded_data = vec![];
        // FIXME: Document properly this
        0x14_u8.encode(buf);
        Encoder::new(&mut encoded_data)
            .encode_field(&self.id)
            .encode_field(&self.block_headers)
            .finish();
        let msg_data = snappy_encode(encoded_data)?;
        buf.put_slice(&msg_data);
        Ok(())
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

// https://github.com/ethereum/devp2p/blob/master/caps/eth.md#getblockbodies-0x05
#[derive(Debug)]
pub(crate) struct GetBlockBodies {
    // id is a u64 chosen by the requesting peer, the responding peer must mirror the value for the response
    // https://github.com/ethereum/devp2p/blob/master/caps/eth.md#protocol-messages
    id: u64,
    block_hashes: Vec<BlockHash>,
}

impl GetBlockBodies {
    pub fn new(id: u64, block_hashes: Vec<BlockHash>) -> Self {
        Self { block_hashes, id }
    }
}

impl RLPxMessage for GetBlockBodies {
    fn encode(&self, buf: &mut dyn BufMut) -> Result<(), RLPEncodeError> {
        let mut encoded_data = vec![];
        Encoder::new(&mut encoded_data)
            .encode_field(&self.id)
            .encode_field(&self.block_hashes)
            .finish();

        let msg_data = snappy_encode(encoded_data)?;
        buf.put_slice(&msg_data);
        Ok(())
    }

    fn decode(msg_data: &[u8]) -> Result<Self, RLPDecodeError> {
        let mut snappy_decoder = SnappyDecoder::new();
        let decompressed_data = snappy_decoder
            .decompress_vec(msg_data)
            .map_err(|err| RLPDecodeError::Custom(err.to_string()))?;
        let decoder = Decoder::new(&decompressed_data)?;
        let (id, decoder): (u64, _) = decoder.decode_field("request-id")?;
        let (block_hashes, _): (Vec<BlockHash>, _) = decoder.decode_field("blockHashes")?;

        Ok(Self::new(id, block_hashes))
    }
}

// https://github.com/ethereum/devp2p/blob/master/caps/eth.md#blockbodies-0x06
pub(crate) struct BlockBodies {
    // id is a u64 chosen by the requesting peer, the responding peer must mirror the value for the response
    // https://github.com/ethereum/devp2p/blob/master/caps/eth.md#protocol-messages
    id: u64,
    block_bodies: Vec<BlockBody>,
}

impl BlockBodies {
    pub fn new(id: u64, block_bodies: Vec<BlockBody>) -> Self {
        Self { block_bodies, id }
    }
}

impl RLPxMessage for BlockBodies {
    fn encode(&self, buf: &mut dyn BufMut) -> Result<(), RLPEncodeError> {
        let mut encoded_data = vec![];
        Encoder::new(&mut encoded_data)
            .encode_field(&self.id)
            .encode_field(&self.block_bodies)
            .finish();

        let msg_data = snappy_encode(encoded_data)?;
        buf.put_slice(&msg_data);
        Ok(())
    }

    fn decode(msg_data: &[u8]) -> Result<Self, RLPDecodeError> {
        let mut snappy_decoder = SnappyDecoder::new();
        let decompressed_data = snappy_decoder
            .decompress_vec(msg_data)
            .map_err(|err| RLPDecodeError::Custom(err.to_string()))?;
        let decoder = Decoder::new(&decompressed_data)?;
        let (id, decoder): (u64, _) = decoder.decode_field("request-id")?;
        let (block_bodies, _): (Vec<BlockBody>, _) = decoder.decode_field("blockBodies")?;

        Ok(Self::new(id, block_bodies))
    }
}

#[cfg(test)]
mod tests {
    use ethereum_rust_core::types::BlockHash;

    use crate::rlpx::{
        eth::blocks::{BlockBodies, GetBlockBodies, GetBlockHeaders},
        message::RLPxMessage,
    };

    use super::HashOrNumber;

    #[test]
    fn get_block_headers_startblock_number_message() {
        let get_block_bodies = GetBlockHeaders::new(1, HashOrNumber::Number(1), 0, 0, false);

        let mut buf = Vec::new();
        get_block_bodies.encode(&mut buf).unwrap();

        let decoded = GetBlockHeaders::decode(&buf).unwrap();
        assert_eq!(decoded.id, 1);
        assert_eq!(decoded.startblock, HashOrNumber::Number(1));
    }

    #[test]
    fn get_block_headers_startblock_hash_message() {
        let get_block_bodies =
            GetBlockHeaders::new(1, HashOrNumber::Hash(BlockHash::from([1; 32])), 0, 0, false);

        let mut buf = Vec::new();
        get_block_bodies.encode(&mut buf).unwrap();

        let decoded = GetBlockHeaders::decode(&buf).unwrap();
        assert_eq!(decoded.id, 1);
        assert_eq!(
            decoded.startblock,
            HashOrNumber::Hash(BlockHash::from([1; 32]))
        );
    }

    #[test]
    fn get_block_bodies_empty_message() {
        let blocks_hash = vec![];
        let get_block_bodies = GetBlockBodies::new(1, blocks_hash.clone());

        let mut buf = Vec::new();
        get_block_bodies.encode(&mut buf).unwrap();

        let decoded = GetBlockBodies::decode(&buf).unwrap();
        assert_eq!(decoded.id, 1);
        assert_eq!(decoded.block_hashes, blocks_hash);
    }

    #[test]
    fn get_block_bodies_not_empty_message() {
        let blocks_hash = vec![
            BlockHash::from([0; 32]),
            BlockHash::from([1; 32]),
            BlockHash::from([2; 32]),
        ];
        let get_block_bodies = GetBlockBodies::new(1, blocks_hash.clone());

        let mut buf = Vec::new();
        get_block_bodies.encode(&mut buf).unwrap();

        let decoded = GetBlockBodies::decode(&buf).unwrap();
        assert_eq!(decoded.id, 1);
        assert_eq!(decoded.block_hashes, blocks_hash);
    }

    #[test]
    fn block_bodies_empty_message() {
        let block_bodies = vec![];
        let block_bodies = BlockBodies::new(1, block_bodies);

        let mut buf = Vec::new();
        block_bodies.encode(&mut buf).unwrap();

        let decoded = BlockBodies::decode(&buf).unwrap();
        assert_eq!(decoded.id, 1);
        assert_eq!(decoded.block_bodies, vec![]);
    }
}
