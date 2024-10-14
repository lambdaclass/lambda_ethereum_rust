use bytes::BufMut;
use ethereum_rust_core::{
    types::{BlockBody, BlockHash, ForkId},
    U256,
};
use ethereum_rust_rlp::{
    encode::RLPEncode,
    error::{RLPDecodeError, RLPEncodeError},
    structs::{Decoder, Encoder},
};
use ethereum_rust_storage::{error::StoreError, Store};
use snap::raw::{max_compress_len, Decoder as SnappyDecoder, Encoder as SnappyEncoder};

pub const ETH_VERSION: u32 = 68;

use super::message::RLPxMessage;

fn snappy_encode(encoded_data: Vec<u8>) -> Result<Vec<u8>, RLPEncodeError> {
    let mut snappy_encoder = SnappyEncoder::new();
    let mut msg_data = vec![0; max_compress_len(encoded_data.len()) + 1];
    let compressed_size = snappy_encoder
        .compress(&encoded_data, &mut msg_data)
        .map_err(|_| RLPEncodeError::InvalidCompression)?;

    msg_data.truncate(compressed_size);
    Ok(msg_data)
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
    fn encode(&self, buf: &mut dyn BufMut) -> Result<(), RLPEncodeError> {
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

        let msg_data = snappy_encode(encoded_data)?;
        buf.put_slice(&msg_data);
        Ok(())
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

        let mut snappy_encoder = SnappyEncoder::new();
        let mut msg_data = vec![0; max_compress_len(encoded_data.len()) + 1];

        let compressed_size = snappy_encoder
            .compress(&encoded_data, &mut msg_data)
            .map_err(|_| RLPEncodeError::InvalidCompression)?;

        msg_data.truncate(compressed_size);

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
    use ethereum_rust_core::types::{Block, BlockBody, BlockHash, BlockHeader};
    use ethereum_rust_storage::Store;

    use crate::rlpx::{
        eth::{BlockBodies, GetBlockBodies},
        message::RLPxMessage,
    };

    fn get_block_bodies_from_hash(store: &Store, blocks_hash: Vec<BlockHash>) -> Vec<BlockBody> {
        let mut block_bodies = vec![];
        for block_hash in blocks_hash {
            let block = store.get_block_by_hash(block_hash).unwrap().unwrap();
            block_bodies.push(block.body);
        }
        block_bodies
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

    #[test]
    fn block_bodies_for_multiple_block() {
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
        header2.parent_hash = BlockHash::from([1; 32]);
        header3.parent_hash = BlockHash::from([2; 32]);
        let block1 = Block {
            header: header1,
            body: body.clone(),
        };
        let block2 = Block {
            header: header2,
            body: body.clone(),
        };
        let block3 = Block {
            header: header3,
            body: body.clone(),
        };
        store.add_block(block1.clone()).unwrap();
        store.add_block(block2.clone()).unwrap();
        store.add_block(block3.clone()).unwrap();

        let blocks_hash = vec![
            block1.header.compute_block_hash(),
            block2.header.compute_block_hash(),
            block3.header.compute_block_hash(),
        ];

        let block_bodies = get_block_bodies_from_hash(&store, blocks_hash);
        let block_bodies = BlockBodies::new(1, block_bodies);

        let mut buf = Vec::new();
        block_bodies.encode(&mut buf).unwrap();

        let decoded = BlockBodies::decode(&buf).unwrap();
        assert_eq!(decoded.id, 1);
        assert_eq!(decoded.block_bodies, vec![body.clone(), body.clone(), body]);
    }

    #[test]
    fn get_block_bodies_receive_block_bodies() {
        let store = Store::new("", ethereum_rust_storage::EngineType::InMemory).unwrap();
        let body = BlockBody {
            transactions: vec![],
            ommers: vec![],
            withdrawals: None,
        };
        let mut header1 = BlockHeader::default();
        let mut header2 = BlockHeader::default();
        header1.parent_hash = BlockHash::from([0; 32]);
        header2.parent_hash = BlockHash::from([1; 32]);
        let block1 = Block {
            header: header1,
            body: body.clone(),
        };
        let block2 = Block {
            header: header2,
            body: body.clone(),
        };
        store.add_block(block1.clone()).unwrap();
        store.add_block(block2.clone()).unwrap();
        let blocks_hash = vec![
            block1.header.compute_block_hash(),
            block2.header.compute_block_hash(),
        ];
        let sender_chosen_id = 1;
        let sender_address = "127.0.0.1:3000";
        let receiver_address = "127.0.0.1:4000";
        let get_block_bodies = GetBlockBodies::new(sender_chosen_id, blocks_hash.clone());

        let mut send_data_of_blocks_hash = Vec::new();
        get_block_bodies
            .encode(&mut send_data_of_blocks_hash)
            .unwrap();

        let sender = std::net::UdpSocket::bind(sender_address).unwrap();
        let receiver = std::net::UdpSocket::bind(receiver_address).unwrap();

        sender
            .send_to(&send_data_of_blocks_hash, receiver_address)
            .unwrap(); // sends the blocks_hash
        let mut receiver_data_of_blocks_hash = [0; 1024];
        let len = receiver.recv(&mut receiver_data_of_blocks_hash).unwrap(); // receives the blocks_hash

        let received_block_hashes =
            GetBlockBodies::decode(&receiver_data_of_blocks_hash[..len]).unwrap(); // transform the encoded received data to blockhashes
        assert_eq!(received_block_hashes.id, sender_chosen_id);
        assert_eq!(received_block_hashes.block_hashes, blocks_hash);
        let block_bodies = get_block_bodies_from_hash(&store, blocks_hash);
        let block_bodies = BlockBodies::new(received_block_hashes.id, block_bodies.clone());

        let mut block_bodies_to_send = Vec::new();
        block_bodies.encode(&mut block_bodies_to_send).unwrap(); // encode the block bodies that we got

        receiver
            .send_to(&block_bodies_to_send, sender_address)
            .unwrap(); // send the block bodies to the sender that requested them

        let mut received_block_bodies = [0; 1024];
        let len = sender.recv(&mut received_block_bodies).unwrap(); // receive the block bodies
        let received_block_bodies = BlockBodies::decode(&received_block_bodies[..len]).unwrap();
        // decode the received block bodies

        assert_eq!(received_block_bodies.id, sender_chosen_id);
        assert_eq!(received_block_bodies.block_bodies, vec![body.clone(), body]);
    }
}
