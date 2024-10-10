use bytes::BufMut;
use ethereum_rust_core::{
    types::{BlockHash, ForkId, Receipt},
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

// https://github.com/ethereum/devp2p/blob/master/caps/eth.md#getreceipts-0x0f
#[derive(Debug)]
pub(crate) struct GetReceipts {
    // id is a u64 chosen by the requesting peer, the responding peer must mirror the value for the response
    // https://github.com/ethereum/devp2p/blob/master/caps/eth.md#protocol-messages
    id: u64,
    block_hashes: Vec<BlockHash>,
}

impl GetReceipts {
    pub fn new(id: u64, block_hashes: Vec<BlockHash>) -> Self {
        Self { block_hashes, id }
    }
}

impl RLPxMessage for GetReceipts {
    fn encode(&self, buf: &mut dyn BufMut) {
        let mut encoded_data = vec![];
        Encoder::new(&mut encoded_data)
            .encode_field(&self.id)
            .encode_field(&self.block_hashes)
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
            .map_err(|err| RLPDecodeError::Custom(err.to_string()))?;
        let decoder = Decoder::new(&decompressed_data)?;
        let (id, decoder): (u64, _) = decoder.decode_field("request-id")?;
        let (block_hashes, _): (Vec<BlockHash>, _) = decoder.decode_field("blockHashes")?;

        Ok(Self::new(id, block_hashes))
    }
}

// https://github.com/ethereum/devp2p/blob/master/caps/eth.md#receipts-0x10
pub(crate) struct Receipts {
    // id is a u64 chosen by the requesting peer, the responding peer must mirror the value for the response
    // https://github.com/ethereum/devp2p/blob/master/caps/eth.md#protocol-messages
    id: u64,
    receipts: Vec<Vec<Receipt>>,
}

impl Receipts {
    pub fn new(id: u64, receipts: Vec<Vec<Receipt>>) -> Self {
        Self { receipts, id }
    }
}

impl RLPxMessage for Receipts {
    fn encode(&self, buf: &mut dyn BufMut) {
        let mut encoded_data = vec![];
        Encoder::new(&mut encoded_data)
            .encode_field(&self.id)
            .encode_field(&self.receipts)
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
            .map_err(|err| RLPDecodeError::Custom(err.to_string()))?;
        let decoder = Decoder::new(&decompressed_data)?;
        let (id, decoder): (u64, _) = decoder.decode_field("request-id")?;
        let (receipts, _): (Vec<Vec<Receipt>>, _) = decoder.decode_field("receipts")?;

        Ok(Self::new(id, receipts))
    }
}

#[cfg(test)]
mod tests {
    use ethereum_rust_core::types::{Block, BlockBody, BlockHash, BlockHeader, Receipt, TxType};
    use ethereum_rust_storage::Store;

    use crate::rlpx::{
        eth::{GetReceipts, Receipts},
        message::RLPxMessage,
    };

    fn get_receipts_from_hash(store: &Store, blocks_hash: Vec<BlockHash>) -> Vec<Vec<Receipt>> {
        let mut receipts = vec![];
        for block_hash in blocks_hash {
            let block_receipts = store
                .get_all_receipts_by_hash(block_hash)
                .unwrap()
                .unwrap_or_default();
            receipts.push(block_receipts);
        }
        receipts
    }

    #[test]
    fn get_recepits_bodies_empty_message() {
        let blocks_hash = vec![];
        let get_receipts = GetReceipts::new(1, blocks_hash.clone());

        let mut buf = Vec::new();
        get_receipts.encode(&mut buf);

        let decoded = GetReceipts::decode(&buf).unwrap();
        assert_eq!(decoded.id, 1);
        assert_eq!(decoded.block_hashes, blocks_hash);
    }

    #[test]
    fn get_receipts_not_empty_message() {
        let blocks_hash = vec![
            BlockHash::from([0; 32]),
            BlockHash::from([1; 32]),
            BlockHash::from([2; 32]),
        ];
        let get_receipts = GetReceipts::new(1, blocks_hash.clone());

        let mut buf = Vec::new();
        get_receipts.encode(&mut buf);

        let decoded = GetReceipts::decode(&buf).unwrap();
        assert_eq!(decoded.id, 1);
        assert_eq!(decoded.block_hashes, blocks_hash);
    }

    #[test]
    fn receipts_empty_message() {
        let receipts = vec![];
        let receipts = Receipts::new(1, receipts);

        let mut buf = Vec::new();
        receipts.encode(&mut buf);

        let decoded = Receipts::decode(&buf).unwrap();
        assert_eq!(decoded.id, 1);
        assert_eq!(decoded.receipts, Vec::<Vec<Receipt>>::new());
    }

    #[test]
    fn multiple_receipts_one_block() {
        let store = Store::new("", ethereum_rust_storage::EngineType::InMemory).unwrap();
        let body = BlockBody {
            transactions: vec![],
            ommers: vec![],
            withdrawals: None,
        };
        let header = BlockHeader::default();
        let block = Block {
            header,
            body: body.clone(),
        };
        let receipt1 = Receipt::new(TxType::default(), true, 100, vec![]);
        let receipt2 = Receipt::new(TxType::default(), true, 500, vec![]);
        let receipt3 = Receipt::new(TxType::default(), true, 1000, vec![]);
        let block_hash = block.header.compute_block_hash();
        store.add_block(block.clone()).unwrap();
        store.add_receipt(block_hash, 1, receipt1.clone()).unwrap();
        store.add_receipt(block_hash, 2, receipt2.clone()).unwrap();
        store.add_receipt(block_hash, 3, receipt3.clone()).unwrap();

        let blocks_hash = vec![block_hash];

        let receipts = get_receipts_from_hash(&store, blocks_hash);
        let receipts = Receipts::new(1, receipts);

        let mut buf = Vec::new();
        receipts.encode(&mut buf);

        let decoded = Receipts::decode(&buf).unwrap();

        assert_eq!(decoded.id, 1);
        assert_eq!(decoded.receipts.len(), 1);
        assert_eq!(decoded.receipts[0].len(), 3);
        // should be a vec![vec![receipt1, receipt2, receipt3]]
    }

    #[test]
    fn multiple_receipts_multiple_blocks() {
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
        let receipt1 = Receipt::new(TxType::default(), true, 100, vec![]);
        let receipt2 = Receipt::new(TxType::default(), true, 500, vec![]);
        let receipt3 = Receipt::new(TxType::default(), true, 1000, vec![]);
        let block_hash1 = block1.header.compute_block_hash();
        let block_hash2 = block2.header.compute_block_hash();
        let block_hash3 = block3.header.compute_block_hash();
        store.add_block(block1.clone()).unwrap();
        store.add_block(block2.clone()).unwrap();
        store.add_block(block3.clone()).unwrap();
        store.add_receipt(block_hash1, 1, receipt1.clone()).unwrap();
        store.add_receipt(block_hash1, 2, receipt2.clone()).unwrap();
        store.add_receipt(block_hash3, 1, receipt3.clone()).unwrap();

        let blocks_hash = vec![block_hash1, block_hash2, block_hash3];

        let block_bodies = get_receipts_from_hash(&store, blocks_hash);
        let receipts = Receipts::new(1, block_bodies);

        let mut buf = Vec::new();
        receipts.encode(&mut buf);

        let decoded = Receipts::decode(&buf).unwrap();

        assert_eq!(decoded.id, 1);
        assert_eq!(decoded.receipts.len(), 3);
        assert_eq!(decoded.receipts[0].len(), 2);
        assert_eq!(decoded.receipts[1].len(), 0);
        assert_eq!(decoded.receipts[2].len(), 1);
        // should be a vec![vec![receipt1, receipt2], vec![], vec![receipt3]]
    }
}
