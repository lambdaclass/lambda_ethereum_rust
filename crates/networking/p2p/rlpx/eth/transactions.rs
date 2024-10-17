use bytes::BufMut;
use ethereum_rust_core::{types::MempoolTransaction, H256};
use ethereum_rust_rlp::{
    error::{RLPDecodeError, RLPEncodeError},
    structs::{Decoder, Encoder},
};
use snap::raw::Decoder as SnappyDecoder;

use crate::rlpx::message::RLPxMessage;

use super::snappy_encode;

// https://github.com/ethereum/devp2p/blob/master/caps/eth.md#transactions-0x02
// Broadcast message
#[derive(Debug)]
pub(crate) struct Transactions {
    transactions: Vec<MempoolTransaction>,
}

impl Transactions {
    pub fn new(transactions: Vec<MempoolTransaction>) -> Self {
        Self { transactions }
    }
}

impl RLPxMessage for Transactions {
    fn encode(&self, buf: &mut dyn BufMut) -> Result<(), RLPEncodeError> {
        let mut encoded_data = vec![];
        Encoder::new(&mut encoded_data)
            .encode_field(&self.transactions)
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
        let (transactions, _): (Vec<MempoolTransaction>, _) =
            decoder.decode_field("transactions")?;

        Ok(Self::new(transactions))
    }
}

// https://github.com/ethereum/devp2p/blob/master/caps/eth.md#newpooledtransactionhashes-0x08
// Broadcast message
#[derive(Debug)]
pub(crate) struct NewPooledTransactionHashes {
    transaction_types: Vec<u8>,
    transaction_sizes: Vec<usize>,
    transaction_hashes: Vec<H256>,
}

impl NewPooledTransactionHashes {
    // delete this after we use this in the main loop
    #[allow(dead_code)]
    pub fn new(transactions: Vec<MempoolTransaction>) -> Self {
        let transactions_len = transactions.len();
        let mut transaction_types = Vec::with_capacity(transactions_len);
        let mut transaction_sizes = Vec::with_capacity(transactions_len);
        let mut transaction_hashes = Vec::with_capacity(transactions_len);
        for transaction in transactions {
            let transaction_type = transaction.tx_type();
            transaction_types.push(transaction_type as u8);
            // size is defined as the len of the concatenation of tx_type and the tx_data
            // as the tx_type goes from 0x00 to 0xff, the size of tx_type is 1 byte
            let transaction_size = 1 + transaction.data().len();
            transaction_sizes.push(transaction_size);
            let transaction_hash = transaction.compute_hash();
            transaction_hashes.push(transaction_hash);
        }
        Self {
            transaction_types,
            transaction_sizes,
            transaction_hashes,
        }
    }
}

impl RLPxMessage for NewPooledTransactionHashes {
    fn encode(&self, buf: &mut dyn BufMut) -> Result<(), RLPEncodeError> {
        let mut encoded_data = vec![];
        Encoder::new(&mut encoded_data)
            .encode_field(&self.transaction_types)
            .encode_field(&self.transaction_sizes)
            .encode_field(&self.transaction_hashes)
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
        let (transaction_types, decoder): (Vec<u8>, _) =
            decoder.decode_field("transactionTypes")?;
        let (transaction_sizes, decoder): (Vec<usize>, _) =
            decoder.decode_field("transactionSizes")?;
        let (transaction_hashes, _): (Vec<H256>, _) = decoder.decode_field("transactionHashes")?;

        if transaction_hashes.len() == transaction_sizes.len()
            && transaction_sizes.len() == transaction_types.len()
        {
            Ok(Self {
                transaction_types,
                transaction_sizes,
                transaction_hashes,
            })
        } else {
            Err(RLPDecodeError::Custom(
                "transaction_hashes, transaction_sizes and transaction_types must have the same length"
                    .to_string(),
            ))
        }
    }
}

// https://github.com/ethereum/devp2p/blob/master/caps/eth.md#getpooledtransactions-0x09
#[derive(Debug)]
pub(crate) struct GetPooledTransactions {
    // id is a u64 chosen by the requesting peer, the responding peer must mirror the value for the response
    // https://github.com/ethereum/devp2p/blob/master/caps/eth.md#protocol-messages
    id: u64,
    transaction_hashes: Vec<H256>,
}

impl GetPooledTransactions {
    pub fn new(id: u64, transaction_hashes: Vec<H256>) -> Self {
        Self {
            transaction_hashes,
            id,
        }
    }
}

impl RLPxMessage for GetPooledTransactions {
    fn encode(&self, buf: &mut dyn BufMut) -> Result<(), RLPEncodeError> {
        let mut encoded_data = vec![];
        Encoder::new(&mut encoded_data)
            .encode_field(&self.id)
            .encode_field(&self.transaction_hashes)
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
        let (transaction_hashes, _): (Vec<H256>, _) = decoder.decode_field("transactionHashes")?;

        Ok(Self::new(id, transaction_hashes))
    }
}

// https://github.com/ethereum/devp2p/blob/master/caps/eth.md#pooledtransactions-0x0a
pub(crate) struct PooledTransactions {
    // id is a u64 chosen by the requesting peer, the responding peer must mirror the value for the response
    // https://github.com/ethereum/devp2p/blob/master/caps/eth.md#protocol-messages
    id: u64,
    pooled_transactions: Vec<MempoolTransaction>,
}

impl PooledTransactions {
    pub fn new(id: u64, pooled_transactions: Vec<MempoolTransaction>) -> Self {
        Self {
            pooled_transactions,
            id,
        }
    }
}

impl RLPxMessage for PooledTransactions {
    fn encode(&self, buf: &mut dyn BufMut) -> Result<(), RLPEncodeError> {
        let mut encoded_data = vec![];
        Encoder::new(&mut encoded_data)
            .encode_field(&self.id)
            .encode_field(&self.pooled_transactions)
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
        let (pooled_transactions, _): (Vec<MempoolTransaction>, _) =
            decoder.decode_field("pooledTransactions")?;

        Ok(Self::new(id, pooled_transactions))
    }
}

#[cfg(test)]
mod tests {
    use std::net::UdpSocket;

    use ethereum_rust_core::{
        types::{
            EIP1559Transaction, EIP2930Transaction, EIP4844Transaction, MempoolTransaction,
            Transaction, TxKind,
        },
        Address, H256, U256,
    };
    use ethereum_rust_storage::{error::StoreError, Store};

    use crate::rlpx::{
        eth::transactions::{
            GetPooledTransactions, NewPooledTransactionHashes, PooledTransactions, Transactions,
        },
        message::RLPxMessage,
    };

    fn get_pooled_transactions_from_hashes(
        storage: &Store,
        transaction_hashes: Vec<H256>,
    ) -> Result<Vec<MempoolTransaction>, StoreError> {
        let mut pooled_transactions = vec![];

        for transaction_hash in transaction_hashes {
            let pooled_transaction = match storage.get_transaction_from_pool(transaction_hash)? {
                Some(pooled_transaction) => pooled_transaction,
                None => continue,
            };
            pooled_transactions.push(pooled_transaction);
        }
        Ok(pooled_transactions)
    }

    fn send_transactions_with_sockets(
        sender_chosen_id: u64,
        store: &Store,
        transaction_hashes: Vec<H256>,
        sender: UdpSocket,
        receiver: UdpSocket,
    ) -> PooledTransactions {
        let get_pooled_transactions =
            GetPooledTransactions::new(sender_chosen_id, transaction_hashes.clone());
        let mut send_data_of_transaction_hashes = Vec::new();
        get_pooled_transactions
            .encode(&mut send_data_of_transaction_hashes)
            .unwrap();
        sender.send(&send_data_of_transaction_hashes).unwrap(); // sends the transaction_hashes

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
        let pooled_transactions = get_pooled_transactions_from_hashes(
            store,
            received_transaction_hashes.transaction_hashes,
        )
        .unwrap();
        let pooled_transactions =
            PooledTransactions::new(received_transaction_hashes.id, pooled_transactions);
        let mut pooled_transactions_to_send = Vec::new();
        pooled_transactions
            .encode(&mut pooled_transactions_to_send)
            .unwrap(); // encode the pooled transactions that we got
        receiver.send(&pooled_transactions_to_send).unwrap(); // sends to the requester

        let mut received_pooled_transactions = [0; 1024];
        let len = sender.recv(&mut received_pooled_transactions).unwrap(); // receive the pooled transactions

        PooledTransactions::decode(&received_pooled_transactions[..len]).unwrap()
    }

    fn create_default_transactions() -> Vec<MempoolTransaction> {
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
        vec![
            MempoolTransaction::new(transaction1),
            MempoolTransaction::new(transaction2),
            MempoolTransaction::new(transaction3),
            MempoolTransaction::new(transaction4),
        ]
    }

    #[test]
    fn transactions_message() {
        // here https://github.com/belfortep/devp2p/blob/master/caps/eth.md#transactions-0x02 says:
        //"Specify transactions that the peer should make sure is included on its transaction queue"
        // This means transactions to add to the store, right?

        let transactions = create_default_transactions();
        let sender_address = "127.0.0.1:3005";
        let receiver_address = "127.0.0.1:4005";
        let sender = std::net::UdpSocket::bind(sender_address).unwrap();
        let receiver = std::net::UdpSocket::bind(receiver_address).unwrap();

        let send_transactions = Transactions::new(transactions.clone());
        let mut send_data_of_transactions = Vec::new();
        send_transactions
            .encode(&mut send_data_of_transactions)
            .unwrap();
        sender
            .send_to(&send_data_of_transactions, receiver_address)
            .unwrap(); // sends the transactions

        let mut receiver_data_of_transactions = [0; 1024];
        let len = receiver.recv(&mut receiver_data_of_transactions).unwrap(); // receives the transactions
        let received_transactions =
            Transactions::decode(&receiver_data_of_transactions[..len]).unwrap(); // transform the encoded received data to our struct
        assert_eq!(received_transactions.transactions, transactions);

        let store = Store::new("", ethereum_rust_storage::EngineType::InMemory).unwrap();
        // probably we would want to add the transactions to the store after receiving the broadcast
        for transaction in received_transactions.transactions {
            store
                .add_transaction_to_pool(transaction.compute_hash(), transaction.clone())
                .unwrap();
        }
    }

    /// tests an example of receiving the broadcast msg of NewPooledTransactionsHashes
    /// and what to do after it
    #[test]
    fn new_pooled_transactions_hashes_message() {
        // here https://github.com/belfortep/devp2p/blob/master/caps/eth.md#newpooledtransactionhashes-0x08 says:
        //"This message announces one or more transactions that have appeared in the network and which have not yet been included in a block."
        // We need to verify if we have those transactions in our store, and if we don't we ask with the GetPooledTransactions message

        let transactions = create_default_transactions();
        let sender_address = "127.0.0.1:3007";
        let receiver_address = "127.0.0.1:4007";
        let sender = std::net::UdpSocket::bind(sender_address).unwrap();
        sender.connect("127.0.0.1:4007").unwrap();
        let receiver = std::net::UdpSocket::bind(receiver_address).unwrap();
        receiver.connect("127.0.0.1:3007").unwrap();

        // Store of the sender of broadcast
        let store_of_broadcast_sender_of_transactions =
            Store::new("", ethereum_rust_storage::EngineType::InMemory).unwrap();
        for transaction in &transactions {
            let hash = transaction.compute_hash();
            store_of_broadcast_sender_of_transactions
                .add_transaction_to_pool(hash, transaction.clone())
                .unwrap();
        }
        // Send the broadcast message
        let send_transactions = NewPooledTransactionHashes::new(transactions.clone());
        let mut send_data_of_transactions = Vec::new();
        send_transactions
            .encode(&mut send_data_of_transactions)
            .unwrap();
        sender.send(&send_data_of_transactions).unwrap(); // sends the transactions

        // Receiver of the broadcast
        let mut receiver_data_of_transactions = [0; 1024];
        let len = receiver.recv(&mut receiver_data_of_transactions).unwrap(); // receives the transactions
        let received_transactions =
            NewPooledTransactionHashes::decode(&receiver_data_of_transactions[..len]).unwrap(); // transform the encoded received data to our struct

        // As the receiver, we verify if we have the hashes or not
        let store_of_receiver_of_broadcast =
            Store::new("", ethereum_rust_storage::EngineType::InMemory).unwrap();
        let mut hashes_to_request = vec![];
        for transaction_hash in received_transactions.transaction_hashes {
            match store_of_receiver_of_broadcast
                .get_transaction_from_pool(transaction_hash)
                .unwrap()
            {
                Some(_) => {}
                None => {
                    hashes_to_request.push(transaction_hash);
                }
            }
        }

        // Now we ask for the transactions that we don't have.
        // Not necesary that we ask to the same peer that did the broadcast
        let send_id = 1;
        let received_pooled_transactions = send_transactions_with_sockets(
            send_id,
            &store_of_broadcast_sender_of_transactions,
            hashes_to_request.clone(),
            sender,
            receiver,
        );

        let mut hashes = vec![];
        for transaction in received_pooled_transactions.pooled_transactions {
            let hash = transaction.compute_hash();
            hashes.push(hash);
        }
        assert_eq!(hashes, hashes_to_request);
    }

    #[test]
    fn get_pooled_transactions_empty_message() {
        let transaction_hashes = vec![];
        let get_pooled_transactions = GetPooledTransactions::new(1, transaction_hashes.clone());

        let mut buf = Vec::new();
        get_pooled_transactions.encode(&mut buf).unwrap();

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
        let get_pooled_transactions = GetPooledTransactions::new(1, transaction_hashes.clone());

        let mut buf = Vec::new();
        get_pooled_transactions.encode(&mut buf).unwrap();

        let decoded = GetPooledTransactions::decode(&buf).unwrap();
        assert_eq!(decoded.id, 1);
        assert_eq!(decoded.transaction_hashes, transaction_hashes);
    }

    #[test]
    fn pooled_transactions_empty_message() {
        let transaction_hashes = vec![];
        let store = Store::new("", ethereum_rust_storage::EngineType::InMemory).unwrap();
        let pooled_transactions =
            get_pooled_transactions_from_hashes(&store, transaction_hashes).unwrap();
        let pooled_transactions = PooledTransactions::new(1, pooled_transactions);

        let mut buf = Vec::new();
        pooled_transactions.encode(&mut buf).unwrap();

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
                MempoolTransaction::new(Transaction::EIP2930Transaction(Default::default())),
            )
            .unwrap();

        let transaction_hashes = vec![H256::from_low_u64_be(404)];

        let pooled_transactions =
            get_pooled_transactions_from_hashes(&store, transaction_hashes).unwrap();
        let pooled_transactions = PooledTransactions::new(1, pooled_transactions);

        let mut buf = Vec::new();
        pooled_transactions.encode(&mut buf).unwrap();

        let decoded = PooledTransactions::decode(&buf).unwrap();
        assert_eq!(decoded.id, 1);
        assert_eq!(decoded.pooled_transactions, vec![]);
    }

    #[test]
    fn pooled_transactions_of_one_type() {
        let store = Store::new("", ethereum_rust_storage::EngineType::InMemory).unwrap();
        let transaction1 =
            MempoolTransaction::new(Transaction::LegacyTransaction(Default::default()));

        store
            .add_transaction_to_pool(H256::from_low_u64_be(1), transaction1.clone())
            .unwrap();
        let transaction_hashes = vec![H256::from_low_u64_be(1)];
        let pooled_transactions =
            get_pooled_transactions_from_hashes(&store, transaction_hashes).unwrap();
        let pooled_transactions = PooledTransactions::new(1, pooled_transactions);

        let mut buf = Vec::new();
        pooled_transactions.encode(&mut buf).unwrap();
        let decoded = PooledTransactions::decode(&buf).unwrap();
        assert_eq!(decoded.id, 1);
        assert_eq!(decoded.pooled_transactions, vec![transaction1]);
    }

    #[test]
    fn multiple_pooled_transactions_of_different_types() {
        let store = Store::new("", ethereum_rust_storage::EngineType::InMemory).unwrap();
        let transactions = create_default_transactions();
        let mut transaction_hashes = vec![];
        for transaction in &transactions {
            let hash = transaction.compute_hash();
            store
                .add_transaction_to_pool(hash, transaction.clone())
                .unwrap();
            transaction_hashes.push(hash);
        }
        let pooled_transactions =
            get_pooled_transactions_from_hashes(&store, transaction_hashes).unwrap();
        let pooled_transactions = PooledTransactions::new(1, pooled_transactions);

        let mut buf = Vec::new();
        pooled_transactions.encode(&mut buf).unwrap();

        let decoded = PooledTransactions::decode(&buf).unwrap();
        assert_eq!(decoded.id, 1);
        assert_eq!(decoded.pooled_transactions, transactions);
    }

    #[test]
    fn get_pooled_transactions_receive_pooled_transactions() {
        let store = Store::new("", ethereum_rust_storage::EngineType::InMemory).unwrap();
        let transactions = create_default_transactions();
        let mut transaction_hashes = vec![];
        for transaction in &transactions {
            let hash = transaction.compute_hash();
            store
                .add_transaction_to_pool(hash, transaction.clone())
                .unwrap();
            transaction_hashes.push(hash);
        }

        let sender_address = "127.0.0.1:3006";
        let receiver_address = "127.0.0.1:4006";
        let sender = std::net::UdpSocket::bind(sender_address).unwrap();
        sender.connect("127.0.0.1:4006").unwrap();
        let receiver = std::net::UdpSocket::bind(receiver_address).unwrap();
        receiver.connect("127.0.0.1:3006").unwrap();
        let send_id = 1;
        let received_pooled_transactions =
            send_transactions_with_sockets(send_id, &store, transaction_hashes, sender, receiver);
        assert_eq!(received_pooled_transactions.id, send_id);
        assert_eq!(
            received_pooled_transactions.pooled_transactions,
            transactions
        );
    }
}
