use bytes::BufMut;
use ethereum_rust_core::types::{BlockHash, Receipt};
use ethereum_rust_rlp::{
    error::{RLPDecodeError, RLPEncodeError},
    structs::{Decoder, Encoder},
};
use snap::raw::Decoder as SnappyDecoder;

use crate::rlpx::message::RLPxMessage;

use super::snappy_encode;

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
    fn encode(&self, buf: &mut dyn BufMut) -> Result<(), RLPEncodeError> {
        let mut encoded_data = vec![];
        Encoder::new(&mut encoded_data)
            .encode_field(&self.id)
            .encode_field(&self.receipts)
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
        let (receipts, _): (Vec<Vec<Receipt>>, _) = decoder.decode_field("receipts")?;

        Ok(Self::new(id, receipts))
    }
}

#[cfg(test)]
mod tests {
    use ethereum_rust_core::types::{Block, BlockBody, BlockHash, BlockHeader, Receipt, TxType};
    use ethereum_rust_storage::Store;

    use crate::rlpx::{
        eth::receipts::{GetReceipts, Receipts},
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
    fn get_receipts_empty_message() {
        let blocks_hash = vec![];
        let get_receipts = GetReceipts::new(1, blocks_hash.clone());

        let mut buf = Vec::new();
        get_receipts.encode(&mut buf).unwrap();

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
        get_receipts.encode(&mut buf).unwrap();

        let decoded = GetReceipts::decode(&buf).unwrap();
        assert_eq!(decoded.id, 1);
        assert_eq!(decoded.block_hashes, blocks_hash);
    }

    #[test]
    fn receipts_empty_message() {
        let receipts = vec![];
        let receipts = Receipts::new(1, receipts);

        let mut buf = Vec::new();
        receipts.encode(&mut buf).unwrap();

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
        receipts.encode(&mut buf).unwrap();

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

        let receipts = get_receipts_from_hash(&store, blocks_hash);
        let receipts = Receipts::new(1, receipts);

        let mut buf = Vec::new();
        receipts.encode(&mut buf).unwrap();

        let decoded = Receipts::decode(&buf).unwrap();

        assert_eq!(decoded.id, 1);
        assert_eq!(decoded.receipts.len(), 3);
        assert_eq!(decoded.receipts[0].len(), 2);
        assert_eq!(decoded.receipts[1].len(), 0);
        assert_eq!(decoded.receipts[2].len(), 1);
        // should be a vec![vec![receipt1, receipt2], vec![], vec![receipt3]]
    }

    #[test]
    fn get_receipts_receive_receipts() {
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

        let receipt1 = Receipt::new(TxType::default(), true, 100, vec![]);
        let receipt2 = Receipt::new(TxType::default(), true, 500, vec![]);
        let receipt3 = Receipt::new(TxType::default(), true, 1000, vec![]);
        let block_hash1 = block1.header.compute_block_hash();
        let block_hash2 = block2.header.compute_block_hash();
        store.add_block(block1.clone()).unwrap();
        store.add_block(block2.clone()).unwrap();
        store.add_receipt(block_hash1, 1, receipt1.clone()).unwrap();
        store.add_receipt(block_hash1, 2, receipt2.clone()).unwrap();
        store.add_receipt(block_hash2, 1, receipt3.clone()).unwrap();
        let blocks_hash = vec![block_hash1, block_hash2];

        let sender_chosen_id = 1;
        let sender_address = "127.0.0.1:3002";
        let receiver_address = "127.0.0.1:4002";
        let sender = std::net::UdpSocket::bind(sender_address).unwrap();
        sender.connect(receiver_address).unwrap();
        let receiver = std::net::UdpSocket::bind(receiver_address).unwrap();
        receiver.connect(sender_address).unwrap();

        let get_receips = GetReceipts::new(sender_chosen_id, blocks_hash.clone());
        let mut send_data_of_blocks_hash = Vec::new();
        get_receips.encode(&mut send_data_of_blocks_hash).unwrap();

        sender.send(&send_data_of_blocks_hash).unwrap(); // sends the blocks_hash
        let mut receiver_data_of_blocks_hash = [0; 1024];
        let len = receiver.recv(&mut receiver_data_of_blocks_hash).unwrap(); // receives the blocks_hash

        let received_block_hashes =
            GetReceipts::decode(&receiver_data_of_blocks_hash[..len]).unwrap(); // transform the encoded received data to blockhashes
        assert_eq!(received_block_hashes.id, sender_chosen_id);
        assert_eq!(received_block_hashes.block_hashes, blocks_hash);
        let receipts = get_receipts_from_hash(&store, blocks_hash);
        let receipts = Receipts::new(received_block_hashes.id, receipts.clone());

        let mut receipts_to_send = Vec::new();
        receipts.encode(&mut receipts_to_send).unwrap(); // encode the receipts that we got

        receiver.send(&receipts_to_send).unwrap(); // send the receipts to the sender that requested them

        let mut received_receipts = [0; 1024];
        let len = sender.recv(&mut received_receipts).unwrap(); // receive the receipts
        let received_receipts = Receipts::decode(&received_receipts[..len]).unwrap();
        // decode the receipts

        assert_eq!(received_receipts.id, sender_chosen_id);
        assert_eq!(received_receipts.receipts.len(), 2);
        assert_eq!(received_receipts.receipts[0].len(), 2);
        assert_eq!(received_receipts.receipts[1].len(), 1);
        // should be a vec![vec![receipt1, receipt2], vec![receipt3]]
    }
}
