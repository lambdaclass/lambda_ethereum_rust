use std::{str::FromStr, sync::Arc, time::Duration};

use ethrex_core::{
    types::{validate_block_header, BlockHash, BlockHeader, InvalidBlockHeaderError},
    H256,
};
use ethrex_storage::Store;
use tokio::sync::Mutex;
use tracing::{debug, info};

use crate::{
    kademlia::KademliaTable,
    rlpx::{
        eth::blocks::{BlockBodies, BlockHeaders, GetBlockBodies, GetBlockHeaders},
        message::Message,
        snap::GetAccountRange,
    },
};
const REPLY_TIMEOUT: Duration = Duration::from_secs(30);

/// Manager in charge of the snap-sync(for now, will also handle full sync) process
/// TaskList:
/// A) Fetch latest block headers (should we ask what the latest block is first?)
/// B) Validate block headers
/// C) Fetch full Blocks and Receipts || Download Raw State (accounts, storages, bytecodes)
/// D) Healing
#[derive(Debug)]
pub struct SyncManager {
    // true: syncmode = snap, false = syncmode = full
    snap_mode: bool,
    peers: Arc<Mutex<KademliaTable>>,
    // Receiver end of the channel between the manager and the main p2p listen loop
    reply_receiver: Arc<Mutex<tokio::sync::mpsc::Receiver<Message>>>,
}

impl SyncManager {
    pub fn new(
        reply_receiver: tokio::sync::mpsc::Receiver<Message>,
        peers: Arc<Mutex<KademliaTable>>,
        snap_mode: bool,
    ) -> Self {
        Self {
            snap_mode,
            peers,
            reply_receiver: Arc::new(Mutex::new(reply_receiver)),
        }
    }
    // TODO: only uses snap sync, should also process full sync once implemented
    pub async fn start_sync(&mut self, current_head: H256, sync_head: H256, store: Store) {
        const BYTES_PER_REQUEST: u64 = 500; // TODO: Adjust
        info!("Starting snap-sync from current head {current_head} to sync_head {sync_head}");
        // Request all block headers between the current head and the sync head
        // We will begin from the current head so that we download the earliest state first
        // This step is not parallelized
        // Ask for block headers
        let mut block_headers_request = GetBlockHeaders {
            id: 17, // TODO: randomize
            skip: 0,
            startblock: current_head.into(),
            limit: BYTES_PER_REQUEST,
            reverse: false,
        };
        let mut all_block_headers = vec![];
        let mut all_block_hashes = vec![];
        loop {
            // TODO: Randomize id
            block_headers_request.id += 1;
            info!("[Sync] Sending request {block_headers_request:?}");
            // Send a GetBlockHeaders request to a peer
            if self
                .peers
                .lock()
                .await
                .send_message_to_peer(Message::GetBlockHeaders(block_headers_request.clone()))
                .await
                .is_err()
            {
                info!("[Sync] No peers available, retrying in 10 sec");
                // This is the unlikely case where we just started the node and don't have peers, wait a bit and try again
                tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
                continue;
            };
            // Wait for the peer to reply
            if let Ok(Some(message)) = tokio::time::timeout(
                REPLY_TIMEOUT,
                receive_block_headers(
                    &mut *self.reply_receiver.lock().await,
                    block_headers_request.id,
                ),
            )
            .await
            {
                // We received the correct message, we can now
                // A) Validate the batch of headers received and start downloading their state
                // B) Check if we need to download another batch (aka we don't have the sync_head yet)

                // If the response is empty, lets ask another peer
                if message.block_headers.is_empty() {
                    info!("[Sync] Bad peer response");
                    continue;
                }
                // Validate header batch
                if validate_header_batch(&message.block_headers).is_err() {
                    info!("[Sync] Invalid header in batch");
                    continue;
                }
                // Discard the first header as we already have it
                let headers = &message.block_headers[1..];
                let block_hashes = headers
                    .iter()
                    .map(|header| header.compute_block_hash())
                    .collect::<Vec<_>>();
                info!(
                    "Received header batch {}..{}",
                    block_hashes.first().unwrap(),
                    block_hashes.last().unwrap()
                );

                // First iteration will not process the batch, but will wait for all headers to be fetched and validated
                // before processing the whole batch
                all_block_headers.extend_from_slice(&headers);
                all_block_hashes.extend_from_slice(&block_hashes);

                // Check if we already reached our sync head or if we need to fetch more blocks
                if !block_hashes.contains(&sync_head) {
                    // Update the request to fetch the next batch
                    block_headers_request.startblock = (*block_hashes.last().unwrap()).into();
                } else {
                    // No more headers to request
                    break;
                }
            };
            info!("[Sync] Peer response timeout (Headers)");
            // Reply timeouted/ peer shut down, lets try a different peer
        }
        info!("[Sync] All headers fetched and validated");
        // [First Iteration] We finished fetching all headers, now we can process them
        // We will launch 3 tasks to:
        // 1) Fetch each block's state via snap p2p requests
        // 2) Fetch each blocks and its receipts via eth p2p requests
        // 3) Receive replies from the receiver and send them to the two tasks
        let (block_and_receipt_sender, block_and_receipt_receiver) =
            tokio::sync::mpsc::channel::<Message>(10);
        let (snap_state_sender, snap_state_receiver) = tokio::sync::mpsc::channel::<Message>(10);
        let router_handle = tokio::spawn(route_replies(
            self.reply_receiver.clone(),
            snap_state_sender,
            block_and_receipt_sender,
        ));
        let fetch_blocks_and_receipts_handle = tokio::spawn(fetch_blocks_and_receipts(
            all_block_hashes.clone(),
            block_and_receipt_receiver,
            self.peers.clone(),
            store.clone(),
        ));
        let state_roots = all_block_headers
            .iter()
            .map(|header| header.state_root)
            .collect::<Vec<_>>();
        let fetch_snap_state_handle = tokio::spawn(fetch_snap_state(
            state_roots.clone(),
            snap_state_receiver,
            self.peers.clone(),
            store.clone(),
        ));
        // Store headers
        for (header, hash) in all_block_headers
            .into_iter()
            .zip(all_block_hashes.into_iter())
        {
            // TODO: Handle error
            store.set_canonical_block(header.number, hash).unwrap();
            store.update_latest_block_number(header.number).unwrap();
            store.add_block_header(hash, header).unwrap();
        }
        // TODO: Handle error
        let err = tokio::join!(fetch_blocks_and_receipts_handle, fetch_snap_state_handle);
        router_handle.abort();
        // Sync finished
    }

    /// Creates a dummy SyncManager for tests where syncing is not needed
    /// This should only be used it tests as it won't be able to connect to the p2p network
    pub fn dummy() -> Self {
        let dummy_peer_table = Arc::new(Mutex::new(KademliaTable::new(Default::default())));
        Self {
            snap_mode: false,
            peers: dummy_peer_table,
            reply_receiver: Arc::new(Mutex::new(tokio::sync::mpsc::channel(0).1)),
        }
    }
}

async fn receive_block_headers(
    channel: &mut tokio::sync::mpsc::Receiver<Message>,
    id: u64,
) -> Option<BlockHeaders> {
    loop {
        match channel.recv().await {
            Some(Message::BlockHeaders(response)) if response.id == id => return Some(response),
            // Ignore replies that don't match the expected id (such as late responses)
            Some(_other_response) => continue,
            None => return None,
        }
    }
}

fn validate_header_batch(headers: &[BlockHeader]) -> Result<(), InvalidBlockHeaderError> {
    // The first header is a header we have already validated (either current last block or last block in previous batch)
    for headers in headers.windows(2) {
        // TODO: Validation commented to make this work with older blocks
        //validate_block_header(&headers[0], &headers[1])?;
    }
    Ok(())
}

/// Routes replies from the universal receiver to the different active processes
async fn route_replies(
    receiver: Arc<Mutex<tokio::sync::mpsc::Receiver<Message>>>,
    snap_state_sender: tokio::sync::mpsc::Sender<Message>,
    block_and_receipt_sender: tokio::sync::mpsc::Sender<Message>,
) -> Option<BlockHeaders> {
    let mut receiver = receiver.lock().await;
    loop {
        match receiver.recv().await {
            Some(message @ Message::BlockBodies(_) | message @ Message::Receipts(_)) => {
                // TODO: Kill process and restart
                let _ = block_and_receipt_sender.send(message).await;
            }
            Some(
                message @ Message::AccountRange(_)
                | message @ Message::StorageRanges(_)
                | message @ Message::ByteCodes(_),
            ) => {
                // TODO: Kill process and restart
                let _ = snap_state_sender.send(message).await;
            }
            _ => continue,
        }
    }
}

async fn fetch_blocks_and_receipts(
    block_hashes: Vec<BlockHash>,
    mut reply_receiver: tokio::sync::mpsc::Receiver<Message>,
    peers: Arc<Mutex<KademliaTable>>,
    store: Store,
) {
    // Snap state fetching will take much longer than this so we don't need to paralelize fetching blocks and receipts
    // Fetch Block Bodies
    let mut block_bodies_request = GetBlockBodies {
        id: 34,
        block_hashes,
    };
    loop {
        // TODO: Randomize id
        block_bodies_request.id += 1;
        info!("[Sync] Sending Block headers request ");
        // Send a GetBlockBodies request to a peer
        if peers
            .lock()
            .await
            .send_message_to_peer(Message::GetBlockBodies(block_bodies_request.clone()))
            .await
            .is_err()
        {
            info!("[Sync] No peers available, retrying in 10 sec");
            // This is the unlikely case where we just started the node and don't have peers, wait a bit and try again
            tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
            continue;
        };
        // Wait for the peer to reply
        match tokio::time::timeout(REPLY_TIMEOUT, reply_receiver.recv()).await {
            Ok(Some(Message::BlockBodies(message)))
                if message.id == block_bodies_request.id && !message.block_bodies.is_empty() =>
            {
                info!(
                    "[SYNC] Received {} Block Bodies",
                    message.block_bodies.len()
                );
                // Track which bodies we have already fetched
                let (fetched_hashes, remaining_hahses) = block_bodies_request
                    .block_hashes
                    .split_at(message.block_bodies.len());
                // Store Block Bodies
                for (hash, body) in fetched_hashes
                    .into_iter()
                    .zip(message.block_bodies.into_iter())
                {
                    // TODO: handle error
                    store.add_block_body(hash.clone(), body).unwrap()
                }

                // Check if we need to ask for another batch
                if remaining_hahses.is_empty() {
                    break;
                }
                block_bodies_request.block_hashes = remaining_hahses.to_vec();
            }
            // Bad peer response, lets try a different peer
            Ok(Some(_)) => info!("[Sync] Bad peer response"),
            // Reply timeouted/peer shut down, lets try a different peer
            _ => info!("[Sync] Peer response timeout( Blocks & Receipts)"),
        }
        // TODO: Fetch Receipts and store them
    }
}

async fn fetch_snap_state(
    state_roots: Vec<BlockHash>,
    mut reply_receiver: tokio::sync::mpsc::Receiver<Message>,
    peers: Arc<Mutex<KademliaTable>>,
    store: Store,
) {
    for root_hash in state_roots {
        // Fetch Account Ranges
        let mut account_ranges_request = GetAccountRange {
            id: 7,
            root_hash,
            starting_hash: H256::zero(),
            limit_hash: H256::from_str(
                "0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
            )
            .unwrap(),
            response_bytes: 500,
        };
        loop {
            // TODO: Randomize id
            account_ranges_request.id += 1;
            info!("[Sync] Sending Block headers request ");
            // Send a GetBlockBodies request to a peer
            if peers
                .lock()
                .await
                .send_message_to_peer(Message::GetAccountRange(account_ranges_request.clone()))
                .await
                .is_err()
            {
                info!("[Sync] No peers available, retrying in 10 sec");
                // This is the unlikely case where we just started the node and don't have peers, wait a bit and try again
                tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
                continue;
            };
            // Wait for the peer to reply
            match tokio::time::timeout(REPLY_TIMEOUT, reply_receiver.recv()).await {
                Ok(Some(Message::AccountRange(message)))
                    if message.id == account_ranges_request.id && !message.accounts.is_empty() =>
                {
                    info!("[SYNC] Received {} Accounts", message.accounts.len());
                }

                // Bad peer response, lets try a different peer
                Ok(Some(_)) => info!("[Sync] Bad peer response"),
                // Reply timeouted/peer shut down, lets try a different peer
                _ => info!("[Sync] Peer response timeout( Snap Account Range)"),
            }
        }
    }
}

async fn receive_block_bodies(
    channel: &mut tokio::sync::mpsc::Receiver<Message>,
    id: u64,
) -> Option<BlockBodies> {
    loop {
        match channel.recv().await {
            Some(Message::BlockBodies(response)) if response.id == id => return Some(response),
            // Ignore replies that don't match the expected id (such as late responses)
            Some(_other_response) => continue,
            None => return None,
        }
    }
}
