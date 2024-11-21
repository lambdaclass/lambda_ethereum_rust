use std::{sync::Arc, time::Duration};

use ethereum_rust_core::H256;
use tokio::sync::Mutex;
use tracing::{debug, info};

use crate::{
    kademlia::KademliaTable,
    rlpx::{
        eth::blocks::{BlockHeaders, GetBlockHeaders},
        message::Message,
    },
};

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
    reply_receiver: tokio::sync::mpsc::Receiver<Message>,
    active: bool,
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
            reply_receiver,
            active: false,
        }
    }
    // TODO: only uses snap sync, should also process full sync once implemented
    pub async fn start_sync(&mut self, current_head: H256, sync_head: H256) {
        const BYTES_PER_REQUEST: u64 = 500; // TODO: Adjust
        const REPLY_TIMEOUT: Duration = Duration::from_secs(30);
        info!("Starting snap-sync from current head {current_head} to sync_head {sync_head}");
        self.active = true;
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
        loop {
            // TODO: Randomize id
            // Send a GetBlockHeaders request to a peer
            if self
                .peers
                .lock()
                .await
                .send_message_to_peer(Message::GetBlockHeaders(block_headers_request.clone()))
                .await
                .is_err()
            {
                // This is the unlikely case where we just started the node and don't have peers, wait a bit and try again
                tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
                continue;
            };
            // Wait for the peer to reply
            if let Ok(Some(message)) = tokio::time::timeout(
                REPLY_TIMEOUT,
                receive_block_headers(&mut self.reply_receiver, block_headers_request.id),
            )
            .await
            {
                // We received the correct message, we can now
                // A) Validate the batch of headers received and start downloading their state
                // B) Check if we need to download another batch (aka we don't have the sync_head yet)

                // If the response is empty, lets ask another peer
                if message.block_headers.is_empty() {
                    continue;
                }
                // Discard the first header as we already have it
                let headers = &message.block_headers[1..];
                let block_hashes = headers
                    .iter()
                    .map(|header| header.compute_block_hash())
                    .collect::<Vec<_>>();
                debug!(
                    "Received header batch {}..{}",
                    block_hashes.first().unwrap(),
                    block_hashes.last().unwrap()
                );
                // Process headers (validate + download state)
                // TODO!
                // Check if we already reached our sync head or if we need to fetch more blocks
                if !block_hashes.contains(&sync_head) {
                    // Update the request to fetch the next batch
                    block_headers_request.startblock = (*block_hashes.last().unwrap()).into();
                } else {
                    // No more headers to request
                    break;
                }
            };
            // Reply timeouted/ peer shut down, lets try a different peer
        }

        // Sync finished
        self.active = false;
    }

    /// Creates a dummy SyncManager for tests where syncing is not needed
    /// This should only be used it tests as it won't be able to connect to the p2p network
    pub fn dummy() -> Self {
        let dummy_peer_table = Arc::new(Mutex::new(KademliaTable::new(Default::default())));
        Self {
            snap_mode: false,
            peers: dummy_peer_table,
            reply_receiver: tokio::sync::mpsc::channel(0).1,
            active: false,
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
