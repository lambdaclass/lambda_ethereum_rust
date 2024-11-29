use std::{sync::Arc, time::Duration};

use bytes::Bytes;
use ethrex_core::{
    types::{AccountState, BlockBody, BlockHeader},
    H256, U256,
};
use ethrex_rlp::encode::RLPEncode;
use ethrex_trie::verify_range;
use tokio::sync::{mpsc, Mutex};

use crate::{
    rlpx::{
        eth::blocks::{
            BlockBodies, BlockHeaders, GetBlockBodies, GetBlockHeaders, BLOCK_HEADER_LIMIT,
        },
        snap::{
            AccountRange, ByteCodes, GetAccountRange, GetByteCodes, GetStorageRanges, StorageRanges,
        },
    },
    snap::encodable_to_proof,
    RLPxMessage,
};

pub const PEER_REPLY_TIMOUT: Duration = Duration::from_secs(45);
pub const MAX_MESSAGES_IN_PEER_CHANNEL: usize = 25;
pub const MAX_RESPONSE_BYTES: u64 = 512 * 1024;
pub const HASH_MAX: H256 = H256([0xFF; 32]);

#[derive(Debug, Clone)]
/// Holds the respective sender and receiver ends of the communication channels bewteen the peer data and its active connection
pub struct PeerChannels {
    sender: mpsc::Sender<RLPxMessage>,
    receiver: Arc<Mutex<mpsc::Receiver<RLPxMessage>>>,
}

impl PeerChannels {
    /// Sets up the communication channels for the peer
    /// Returns the channel endpoints to send to the active connection's listen loop
    pub(crate) fn create() -> (Self, mpsc::Sender<RLPxMessage>, mpsc::Receiver<RLPxMessage>) {
        let (sender, connection_receiver) =
            mpsc::channel::<RLPxMessage>(MAX_MESSAGES_IN_PEER_CHANNEL);
        let (connection_sender, receiver) =
            mpsc::channel::<RLPxMessage>(MAX_MESSAGES_IN_PEER_CHANNEL);
        (
            Self {
                sender,
                receiver: Arc::new(Mutex::new(receiver)),
            },
            connection_sender,
            connection_receiver,
        )
    }

    /// Requests block headers from the peer, starting from the `start` block hash towards newer blocks
    /// Returns the block headers or None if:
    /// - There are no available peers (the node just started up or was rejected by all other nodes)
    /// - The response timed out
    /// - The response was empty or not valid
    pub async fn request_block_headers(&self, start: H256) -> Option<Vec<BlockHeader>> {
        let request_id = rand::random();
        let request = RLPxMessage::GetBlockHeaders(GetBlockHeaders {
            id: request_id,
            startblock: start.into(),
            limit: BLOCK_HEADER_LIMIT,
            skip: 0,
            reverse: false,
        });
        self.sender.send(request).await.ok()?;
        let mut receiver = self.receiver.lock().await;
        let block_headers = tokio::time::timeout(PEER_REPLY_TIMOUT, async move {
            loop {
                match receiver.recv().await {
                    Some(RLPxMessage::BlockHeaders(BlockHeaders { id, block_headers }))
                        if id == request_id =>
                    {
                        return Some(block_headers)
                    }
                    // Ignore replies that don't match the expected id (such as late responses)
                    Some(_) => continue,
                    None => return None,
                }
            }
        })
        .await
        .ok()??;
        (!block_headers.is_empty()).then_some(block_headers)
    }

    /// Requests block headers from the peer given their block hashes
    /// Returns the block bodies or None if:
    /// - There are no available peers (the node just started up or was rejected by all other nodes)
    /// - The response timed out
    /// - The response was empty or not valid
    pub async fn request_block_bodies(&self, block_hashes: Vec<H256>) -> Option<Vec<BlockBody>> {
        let block_hashes_len = block_hashes.len();
        let request_id = rand::random();
        let request = RLPxMessage::GetBlockBodies(GetBlockBodies {
            id: request_id,
            block_hashes,
        });
        self.sender.send(request).await.ok()?;
        let mut receiver = self.receiver.lock().await;
        let block_bodies = tokio::time::timeout(PEER_REPLY_TIMOUT, async move {
            loop {
                match receiver.recv().await {
                    Some(RLPxMessage::BlockBodies(BlockBodies { id, block_bodies }))
                        if id == request_id =>
                    {
                        return Some(block_bodies)
                    }
                    // Ignore replies that don't match the expected id (such as late responses)
                    Some(_) => continue,
                    None => return None,
                }
            }
        })
        .await
        .ok()??;
        // Check that the response is not empty and does not contain more bodies than the ones requested
        (!block_bodies.is_empty() && block_bodies.len() <= block_hashes_len).then_some(block_bodies)
    }

    /// Requests an account range from the peer given the state trie's root and the starting hash (the limit hash will be the maximum value of H256)
    /// Will also return a boolean indicating if there is more state to be fetched towards the right of the trie
    /// Returns the response message or None if:
    /// - There are no available peers (the node just started up or was rejected by all other nodes)
    /// - The response timed out
    /// - The response was empty or not valid
    pub async fn request_account_range(
        &self,
        state_root: H256,
        start: H256,
    ) -> Option<(Vec<H256>, Vec<AccountState>, bool)> {
        let request_id = rand::random();
        let request = RLPxMessage::GetAccountRange(GetAccountRange {
            id: request_id,
            root_hash: state_root,
            starting_hash: start,
            limit_hash: HASH_MAX,
            response_bytes: MAX_RESPONSE_BYTES,
        });
        self.sender.send(request).await.ok()?;
        let mut receiver = self.receiver.lock().await;
        let (accounts, proof) = tokio::time::timeout(PEER_REPLY_TIMOUT, async move {
            loop {
                match receiver.recv().await {
                    Some(RLPxMessage::AccountRange(AccountRange {
                        id,
                        accounts,
                        proof,
                    })) if id == request_id => return Some((accounts, proof)),
                    // Ignore replies that don't match the expected id (such as late responses)
                    Some(_) => continue,
                    None => return None,
                }
            }
        })
        .await
        .ok()??;
        // Unzip & validate response
        let proof = encodable_to_proof(&proof);
        let (account_hashes, accounts): (Vec<_>, Vec<_>) = accounts
            .into_iter()
            .map(|unit| (unit.hash, AccountState::from(unit.account)))
            .unzip();
        let encoded_accounts = accounts
            .iter()
            .map(|acc| acc.encode_to_vec())
            .collect::<Vec<_>>();
        let should_continue = verify_range(
            state_root,
            &start,
            &account_hashes,
            &encoded_accounts,
            &proof,
        )
        .ok()?;
        Some((account_hashes, accounts, should_continue))
    }

    // TODO: Inefficient method -> replace with request_storage_ranges
    pub async fn request_storage_range(
        &self,
        storage_root: H256,
        account_hash: H256,
        start: H256,
    ) -> Option<(Vec<H256>, Vec<U256>, bool)> {
        let request_id = rand::random();
        let request = RLPxMessage::GetStorageRanges(GetStorageRanges {
            id: request_id,
            root_hash: storage_root,
            account_hashes: vec![account_hash],
            starting_hash: start,
            limit_hash: HASH_MAX,
            response_bytes: MAX_RESPONSE_BYTES,
        });
        self.sender.send(request).await.ok()?;
        let mut receiver = self.receiver.lock().await;
        let (mut slots, proof) = tokio::time::timeout(PEER_REPLY_TIMOUT, async move {
            loop {
                match receiver.recv().await {
                    Some(RLPxMessage::StorageRanges(StorageRanges { id, slots, proof }))
                        if id == request_id =>
                    {
                        return Some((slots, proof))
                    }
                    // Ignore replies that don't match the expected id (such as late responses)
                    Some(_) => continue,
                    None => return None,
                }
            }
        })
        .await
        .ok()??;
        // We only requested 1 account so lets make sure we got it:
        if slots.len() != 1 {
            return None;
        }
        // Unzip & validate response
        let proof = encodable_to_proof(&proof);
        let (hahsed_keys, values): (Vec<_>, Vec<_>) = slots
            .remove(0)
            .into_iter()
            .map(|slot| (slot.hash, slot.data))
            .unzip();
        let encoded_values = values
            .iter()
            .map(|val| val.encode_to_vec())
            .collect::<Vec<_>>();
        let should_continue =
            verify_range(storage_root, &start, &hahsed_keys, &encoded_values, &proof).ok()?;
        Some((hahsed_keys, values, should_continue))
    }

    /// Requests bytecodes for the given code hashes
    /// Returns the bytecodes or None if:
    /// - There are no available peers (the node just started up or was rejected by all other nodes)
    /// - The response timed out
    /// - The response was empty or not valid
    pub async fn request_bytecodes(&self, hashes: Vec<H256>) -> Option<Vec<Bytes>> {
        let request_id = rand::random();
        let hashes_len = hashes.len();
        let request = RLPxMessage::GetByteCodes(GetByteCodes {
            id: request_id,
            hashes,
            bytes: MAX_RESPONSE_BYTES,
        });
        self.sender.send(request).await.ok()?;
        let mut receiver = self.receiver.lock().await;
        let codes = tokio::time::timeout(PEER_REPLY_TIMOUT, async move {
            loop {
                match receiver.recv().await {
                    Some(RLPxMessage::ByteCodes(ByteCodes { id, codes })) if id == request_id => {
                        return Some(codes)
                    }
                    // Ignore replies that don't match the expected id (such as late responses)
                    Some(_) => continue,
                    None => return None,
                }
            }
        })
        .await
        .ok()??;
        (!codes.is_empty() && codes.len() <= hashes_len).then_some(codes)
    }
}
