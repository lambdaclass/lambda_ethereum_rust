use bytes::BufMut;
use ethereum_rust_core::{
    types::{BlockHash, ForkId, Transaction},
    H256, U256,
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

#[derive(Debug)]
pub(crate) struct Transactions {
    transactions: Vec<Transaction>,
}

impl Transactions {
    pub fn build_from(transactions: Vec<Transaction>) -> Result<Self, StoreError> {
        Ok(Self { transactions })
    }
}

impl RLPxMessage for Transactions {
    fn encode(&self, buf: &mut dyn BufMut) {
        let mut encoded_data = vec![];
        Encoder::new(&mut encoded_data)
            .encode_field(&self.transactions)
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
        let (transactions, _): (Vec<Transaction>, _) =
            decoder.decode_field("transactions").unwrap();

        Ok(Self { transactions })
    }
}

#[derive(Debug)]
pub(crate) struct GetPooledTransactions {
    // id is a u64 chosen by the requesting peer, the responding peer must mirror the value for the response
    // https://github.com/ethereum/devp2p/blob/master/caps/eth.md#protocol-messages
    id: u64,
    transaction_hashes: Vec<H256>,
}

impl GetPooledTransactions {
    pub fn build_from(id: u64, transaction_hashes: Vec<H256>) -> Result<Self, StoreError> {
        Ok(Self {
            transaction_hashes,
            id,
        })
    }
}

impl RLPxMessage for GetPooledTransactions {
    fn encode(&self, buf: &mut dyn BufMut) {
        let mut encoded_data = vec![];
        Encoder::new(&mut encoded_data)
            .encode_field(&self.id)
            .encode_field(&self.transaction_hashes)
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
        let (transaction_hashes, _): (Vec<H256>, _) =
            decoder.decode_field("transactionHashes").unwrap();

        Ok(Self {
            transaction_hashes,
            id,
        })
    }
}

pub(crate) struct PooledTransactions {
    // id is a u64 chosen by the requesting peer, the responding peer must mirror the value for the response
    // https://github.com/ethereum/devp2p/blob/master/caps/eth.md#protocol-messages
    id: u64,
    pooled_transactions: Vec<Transaction>,
}

impl PooledTransactions {
    pub fn build_from(
        id: u64,
        storage: &Store,
        transaction_hashes: Vec<H256>,
    ) -> Result<Self, StoreError> {
        let mut pooled_transactions = vec![];

        for transaction_hash in transaction_hashes {
            let pooled_transaction = match storage.get_transaction_from_pool(transaction_hash)? {
                Some(body) => body,
                None => continue,
            };
            pooled_transactions.push(pooled_transaction);
        }

        Ok(Self {
            pooled_transactions,
            id,
        })
    }
}

impl RLPxMessage for PooledTransactions {
    fn encode(&self, buf: &mut dyn BufMut) {
        let mut encoded_data = vec![];
        Encoder::new(&mut encoded_data)
            .encode_field(&self.id)
            .encode_field(&self.pooled_transactions)
            .finish();
        let mut snappy_encoder: SnappyEncoder = SnappyEncoder::new();
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
        let (id, decoder): (u64, _) = decoder.decode_field("request-id")?;
        let (pooled_transactions, _): (Vec<Transaction>, _) =
            decoder.decode_field("pooledTransactions")?;

        Ok(Self {
            pooled_transactions,
            id,
        })
    }
}

#[cfg(test)]
mod tests {
    use ethereum_rust_core::{
        types::{EIP1559Transaction, EIP2930Transaction, EIP4844Transaction, Transaction, TxKind},
        Address, H256, U256,
    };
    use ethereum_rust_storage::Store;

    use crate::rlpx::{
        eth::{GetPooledTransactions, PooledTransactions, Transactions},
        message::RLPxMessage,
    };

    #[test]
    fn transactions_message() {
        // here https://github.com/belfortep/devp2p/blob/master/caps/eth.md#transactions-0x02 says:
        //"Specify transactions that the peer should make sure is included on its transaction queue"
        // This means transactions to add to the store, right?

        let transaction1 = Transaction::LegacyTransaction(Default::default());
        let transaction2 = EIP1559Transaction {
            signature_r: U256::zero(),
            signature_s: U256::max_value(),
            to: TxKind::Call(Address::zero()),
            ..Default::default()
        };
        let transaction3 = EIP2930Transaction {
            signature_r: U256::zero(),
            signature_s: U256::max_value(),
            to: TxKind::Call(Address::zero()),
            ..Default::default()
        };
        let transaction4 = EIP4844Transaction {
            signature_r: U256::zero(),
            signature_s: U256::max_value(),
            to: Address::zero(),
            ..Default::default()
        };
        let transaction2 = Transaction::EIP1559Transaction(transaction2.clone());
        let transaction3 = Transaction::EIP2930Transaction(transaction3.clone());
        let transaction4 = Transaction::EIP4844Transaction(transaction4.clone());

        let sender_address = "127.0.0.1:3000";
        let receiver_address = "127.0.0.1:4000";
        let sender = std::net::UdpSocket::bind(sender_address).unwrap();
        let receiver = std::net::UdpSocket::bind(receiver_address).unwrap();
        let transactions = vec![
            transaction1.clone(),
            transaction2.clone(),
            transaction3.clone(),
            transaction4.clone(),
        ];

        let send_transactions = Transactions::build_from(transactions.clone()).unwrap();
        let mut send_data_of_transactions = Vec::new();
        send_transactions.encode(&mut send_data_of_transactions);
        sender
            .send_to(&send_data_of_transactions, receiver_address)
            .unwrap(); // sends the transactions

        let mut receiver_data_of_transactions = [0; 1024];
        let len = receiver.recv(&mut receiver_data_of_transactions).unwrap(); // receives the transactions
        let received_transactions =
            Transactions::decode(&receiver_data_of_transactions[..len]).unwrap(); // transform the encoded received data to our struct
        assert_eq!(received_transactions.transactions, transactions);

        let store = Store::new("", ethereum_rust_storage::EngineType::InMemory).unwrap();
        for transaction in received_transactions.transactions {
            store
                .add_transaction_to_pool(transaction.compute_hash(), transaction.clone())
                .unwrap();
        }
    }

    #[test]
    fn get_pooled_transactions_empty_message() {
        let transaction_hashes = vec![];
        let get_pooled_transactions =
            GetPooledTransactions::build_from(1, transaction_hashes.clone()).unwrap();

        let mut buf = Vec::new();
        get_pooled_transactions.encode(&mut buf);

        let decoded = GetPooledTransactions::decode(&buf).unwrap();
        assert_eq!(decoded.id, 1);
        assert_eq!(decoded.transaction_hashes, transaction_hashes);
    }

    #[test]
    fn get_pooled_transactions_not_empty_message() {
        let transaction_hashes = vec![
            H256::from_low_u64_be(1),
            H256::from_low_u64_be(2),
            H256::from_low_u64_be(3),
        ];
        let get_pooled_transactions =
            GetPooledTransactions::build_from(1, transaction_hashes.clone()).unwrap();

        let mut buf = Vec::new();
        get_pooled_transactions.encode(&mut buf);

        let decoded = GetPooledTransactions::decode(&buf).unwrap();
        assert_eq!(decoded.id, 1);
        assert_eq!(decoded.transaction_hashes, transaction_hashes);
    }

    #[test]
    fn pooled_transactions_empty_message() {
        let transaction_hashes = vec![];
        let store = Store::new("", ethereum_rust_storage::EngineType::InMemory).unwrap();
        let pooled_transactions =
            PooledTransactions::build_from(1, &store, transaction_hashes).unwrap();

        let mut buf = Vec::new();
        pooled_transactions.encode(&mut buf);

        let decoded = PooledTransactions::decode(&buf).unwrap();
        assert_eq!(decoded.id, 1);
        assert_eq!(decoded.pooled_transactions, vec![]);
    }

    #[test]
    fn pooled_transactions_not_existing_transaction() {
        let store = Store::new("", ethereum_rust_storage::EngineType::InMemory).unwrap();

        store
            .add_transaction_to_pool(
                H256::from_low_u64_be(1),
                Transaction::EIP2930Transaction(Default::default()),
            )
            .unwrap();

        let transaction_hashes = vec![H256::from_low_u64_be(404)];

        let pooled_transactions =
            PooledTransactions::build_from(1, &store, transaction_hashes).unwrap();

        let mut buf = Vec::new();
        pooled_transactions.encode(&mut buf);

        let decoded = PooledTransactions::decode(&buf).unwrap();
        assert_eq!(decoded.id, 1);
        assert_eq!(decoded.pooled_transactions, vec![]);
    }

    #[test]
    fn pooled_transactions_of_one_type() {
        let store = Store::new("", ethereum_rust_storage::EngineType::InMemory).unwrap();
        let transaction1 = Transaction::LegacyTransaction(Default::default());

        store
            .add_transaction_to_pool(H256::from_low_u64_be(1), transaction1.clone())
            .unwrap();
        let transaction_hashes = vec![H256::from_low_u64_be(1)];
        let pooled_transactions =
            PooledTransactions::build_from(1, &store, transaction_hashes).unwrap();

        let mut buf = Vec::new();
        pooled_transactions.encode(&mut buf);
        let decoded = PooledTransactions::decode(&buf).unwrap();
        assert_eq!(decoded.id, 1);
        assert_eq!(decoded.pooled_transactions, vec![transaction1]);
    }

    #[test]
    fn multiple_pooled_transactions_of_different_types() {
        let store = Store::new("", ethereum_rust_storage::EngineType::InMemory).unwrap();
        let transaction1 = Transaction::LegacyTransaction(Default::default());
        let transaction2 = EIP1559Transaction {
            signature_r: U256::zero(),
            signature_s: U256::max_value(),
            to: TxKind::Call(Address::zero()),
            ..Default::default()
        };
        let transaction3 = EIP2930Transaction {
            signature_r: U256::zero(),
            signature_s: U256::max_value(),
            to: TxKind::Call(Address::zero()),
            ..Default::default()
        };
        let transaction4 = EIP4844Transaction {
            signature_r: U256::zero(),
            signature_s: U256::max_value(),
            to: Address::zero(),
            ..Default::default()
        };
        let transaction2 = Transaction::EIP1559Transaction(transaction2.clone());
        let transaction3 = Transaction::EIP2930Transaction(transaction3.clone());
        let transaction4 = Transaction::EIP4844Transaction(transaction4.clone());
        let transaction_hashes: Vec<H256> = (0..4).map(|x| H256::from_low_u64_be(x)).collect();

        store
            .add_transaction_to_pool(transaction_hashes[0], transaction1.clone())
            .unwrap();
        store
            .add_transaction_to_pool(transaction_hashes[1], transaction2.clone())
            .unwrap();
        store
            .add_transaction_to_pool(transaction_hashes[2], transaction3.clone())
            .unwrap();
        store
            .add_transaction_to_pool(transaction_hashes[3], transaction4.clone())
            .unwrap();
        let pooled_transactions =
            PooledTransactions::build_from(1, &store, transaction_hashes).unwrap();

        let mut buf = Vec::new();
        pooled_transactions.encode(&mut buf);

        let decoded = PooledTransactions::decode(&buf).unwrap();
        assert_eq!(decoded.id, 1);
        assert_eq!(
            decoded.pooled_transactions,
            vec![transaction1, transaction2, transaction3, transaction4]
        );
    }

    #[test]
    fn get_pooled_transactions_receive_pooled_transactions() {
        let store = Store::new("", ethereum_rust_storage::EngineType::InMemory).unwrap();
        let transaction1 = Transaction::LegacyTransaction(Default::default());
        let transaction2 = EIP1559Transaction {
            signature_r: U256::zero(),
            signature_s: U256::max_value(),
            to: TxKind::Call(Address::zero()),
            ..Default::default()
        };
        let transaction3 = EIP2930Transaction {
            signature_r: U256::zero(),
            signature_s: U256::max_value(),
            to: TxKind::Call(Address::zero()),
            ..Default::default()
        };
        let transaction4 = EIP4844Transaction {
            signature_r: U256::zero(),
            signature_s: U256::max_value(),
            to: Address::zero(),
            ..Default::default()
        };
        let transaction2 = Transaction::EIP1559Transaction(transaction2.clone());
        let transaction3 = Transaction::EIP2930Transaction(transaction3.clone());
        let transaction4 = Transaction::EIP4844Transaction(transaction4.clone());
        let transaction_hashes: Vec<H256> = (0..4).map(|x| H256::from_low_u64_be(x)).collect();
        store
            .add_transaction_to_pool(transaction_hashes[0], transaction1.clone())
            .unwrap();
        store
            .add_transaction_to_pool(transaction_hashes[1], transaction2.clone())
            .unwrap();
        store
            .add_transaction_to_pool(transaction_hashes[2], transaction3.clone())
            .unwrap();
        store
            .add_transaction_to_pool(transaction_hashes[3], transaction4.clone())
            .unwrap();

        let sender_chosen_id = 1;
        let sender_address = "127.0.0.1:3000";
        let receiver_address = "127.0.0.1:4000";
        let sender = std::net::UdpSocket::bind(sender_address).unwrap();
        let receiver = std::net::UdpSocket::bind(receiver_address).unwrap();

        let get_pooled_transactions =
            GetPooledTransactions::build_from(sender_chosen_id, transaction_hashes.clone())
                .unwrap();
        let mut send_data_of_transaction_hashes = Vec::new();
        get_pooled_transactions.encode(&mut send_data_of_transaction_hashes);
        sender
            .send_to(&send_data_of_transaction_hashes, receiver_address)
            .unwrap(); // sends the transaction_hashes

        let mut receiver_data_of_transaction_hashes = [0; 1024];
        let len = receiver
            .recv(&mut receiver_data_of_transaction_hashes)
            .unwrap(); // receives the transaction_hashes
        let received_transaction_hashes =
            GetPooledTransactions::decode(&receiver_data_of_transaction_hashes[..len]).unwrap(); // transform the encoded received data to our struct
        assert_eq!(received_transaction_hashes.id, sender_chosen_id);
        assert_eq!(
            received_transaction_hashes.transaction_hashes,
            transaction_hashes
        );

        let pooled_transactions = PooledTransactions::build_from(
            received_transaction_hashes.id,
            &store,
            received_transaction_hashes.transaction_hashes,
        )
        .unwrap();
        let mut pooled_transactions_to_send = Vec::new();
        pooled_transactions.encode(&mut pooled_transactions_to_send); // encode the pooled transactions that we got
        receiver
            .send_to(&pooled_transactions_to_send, sender_address)
            .unwrap(); // sends to the requester

        let mut received_pooled_transactions = [0; 1024];
        let len = sender.recv(&mut received_pooled_transactions).unwrap(); // receive the pooled transactions
        let received_pooled_transactions =
            PooledTransactions::decode(&received_pooled_transactions[..len]).unwrap();

        assert_eq!(received_pooled_transactions.id, sender_chosen_id);
        assert_eq!(
            received_pooled_transactions.pooled_transactions,
            vec![transaction1, transaction2, transaction3, transaction4]
        );
    }
}
