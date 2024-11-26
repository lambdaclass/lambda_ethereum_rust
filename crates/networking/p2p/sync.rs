use std::sync::Arc;

use ethrex_core::{
    types::{validate_block_header, BlockHash, BlockHeader, InvalidBlockHeaderError},
    H256,
};
use ethrex_storage::Store;
use tokio::sync::Mutex;
use tracing::info;

use crate::kademlia::KademliaTable;

/// Manager in charge of the snap-sync(for now, will also handle full sync) process
/// TaskList:
/// - Fetch latest block headers (should we ask what the latest block is first?)
/// - Validate block headers
/// - Fetch full Blocks and Receipts || Download Raw State (accounts, storages, bytecodes)
/// - Healing
#[derive(Debug)]
pub struct SyncManager {
    // true: syncmode = snap, false = syncmode = full
    snap_mode: bool,
    peers: Arc<Mutex<KademliaTable>>,
}

impl SyncManager {
    pub fn new(peers: Arc<Mutex<KademliaTable>>, snap_mode: bool) -> Self {
        Self { snap_mode, peers }
    }
    // TODO: only uses snap sync, should also process full sync once implemented
    pub async fn start_sync(&mut self, mut current_head: H256, sync_head: H256, store: Store) {
        info!("Starting snap-sync from current head {current_head} to sync_head {sync_head}");
        // Request all block headers between the current head and the sync head
        // We will begin from the current head so that we download the earliest state first
        // This step is not parallelized
        // Ask for block headers
        let mut all_block_headers = vec![];
        let mut all_block_hashes = vec![];
        // Make sure we have active peers before we start making requests
        loop {
            if self.peers.lock().await.has_peers() {
                break;
            }
            info!("[Sync] No peers available, retrying in 10 sec");
            // This is the unlikely case where we just started the node and don't have peers, wait a bit and try again
            tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
        }
        loop {
            info!("[Sync] Requesting Block Headers from {current_head}");
            // Request Block Headers from Peer
            if let Some(block_headers) = self
                .peers
                .lock()
                .await
                .request_block_headers(current_head)
                .await
            {
                // We received the correct message, we can now:
                // - Validate the batch of headers received and start downloading their state (Future Iteration)
                // - Check if we need to download another batch (aka we don't have the sync_head yet)

                // Validate header batch
                if validate_header_batch(&block_headers).is_err() {
                    info!("[Sync] Invalid header in batch");
                    continue;
                }
                // Discard the first header as we already have it
                let headers = &block_headers[1..];
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
                    current_head = (*block_hashes.last().unwrap()).into();
                } else {
                    // No more headers to request
                    break;
                }
            }
            info!("[Sync] Peer response timeout (Headers)");
        }
        info!("[Sync] All headers fetched and validated");
        // [First Iteration] We finished fetching all headers, now we can process them
        // We will launch 2 tasks to:
        // - Fetch each block's state via snap p2p requests
        // - Fetch each blocks and its receipts via eth p2p requests
        let fetch_blocks_and_receipts_handle = tokio::spawn(fetch_blocks_and_receipts(
            all_block_hashes.clone(),
            self.peers.clone(),
            store.clone(),
        ));
        let state_roots = all_block_headers
            .iter()
            .map(|header| header.state_root)
            .collect::<Vec<_>>();
        let fetch_snap_state_handle = tokio::spawn(fetch_snap_state(
            state_roots.clone(),
            self.peers.clone(),
            store.clone(),
        ));
        // Store headers
        let mut latest_block_number = 0;
        for (header, hash) in all_block_headers
            .into_iter()
            .zip(all_block_hashes.into_iter())
        {
            // TODO: Handle error
            latest_block_number = header.number;
            store.set_canonical_block(header.number, hash).unwrap();
            store.add_block_header(hash, header).unwrap();
        }
        // TODO: Handle error
        let err = tokio::join!(fetch_blocks_and_receipts_handle, fetch_snap_state_handle);
        // Set latest block number here to avoid reading state that is currently being synced
        store
            .update_latest_block_number(latest_block_number)
            .unwrap();
        // Sync finished
    }

    /// Creates a dummy SyncManager for tests where syncing is not needed
    /// This should only be used it tests as it won't be able to connect to the p2p network
    pub fn dummy() -> Self {
        let dummy_peer_table = Arc::new(Mutex::new(KademliaTable::new(Default::default())));
        Self {
            snap_mode: false,
            peers: dummy_peer_table,
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

async fn fetch_blocks_and_receipts(
    mut block_hashes: Vec<BlockHash>,
    peers: Arc<Mutex<KademliaTable>>,
    store: Store,
) {
    // Snap state fetching will take much longer than this so we don't need to paralelize fetching blocks and receipts
    // Fetch Block Bodies
    loop {
        info!("[Sync] Requesting Block Headers ");
        if let Some(block_bodies) = peers
            .lock()
            .await
            .request_block_bodies(block_hashes.clone())
            .await
        {
            info!("[SYNC] Received {} Block Bodies", block_bodies.len());
            // Track which bodies we have already fetched
            let (fetched_hashes, remaining_hashes) = block_hashes.split_at(block_bodies.len());
            // Store Block Bodies
            for (hash, body) in fetched_hashes.into_iter().zip(block_bodies.into_iter()) {
                // TODO: handle error
                store.add_block_body(hash.clone(), body).unwrap()
            }

            // Check if we need to ask for another batch
            if remaining_hashes.is_empty() {
                break;
            } else {
                block_hashes = remaining_hashes.to_vec();
            }
        }
        info!("[Sync] Peer response timeout( Blocks & Receipts)");
    }
    // TODO: Fetch Receipts and store them
}

async fn fetch_snap_state(
    state_roots: Vec<BlockHash>,
    peers: Arc<Mutex<KademliaTable>>,
    store: Store,
) {
    // for root_hash in state_roots {
    //     // Fetch Account Ranges
    //     let mut account_ranges_request = GetAccountRange {
    //         id: 7,
    //         root_hash,
    //         starting_hash: H256::zero(),
    //         limit_hash: H256::from_str(
    //             "0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
    //         )
    //         .unwrap(),
    //         response_bytes: 500,
    //     };
    //     loop {
    //         // TODO: Randomize id
    //         account_ranges_request.id += 1;
    //         info!("[Sync] Sending Block headers request ");
    //         // Send a GetBlockBodies request to a peer
    //         if peers
    //             .lock()
    //             .await
    //             .send_message_to_peer(Message::GetAccountRange(account_ranges_request.clone()))
    //             .await
    //             .is_err()
    //         {
    //             info!("[Sync] No peers available, retrying in 10 sec");
    //             // This is the unlikely case where we just started the node and don't have peers, wait a bit and try again
    //             tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
    //             continue;
    //         };
    //         // Wait for the peer to reply
    //         match tokio::time::timeout(REPLY_TIMEOUT, reply_receiver.recv()).await {
    //             Ok(Some(Message::AccountRange(message)))
    //                 if message.id == account_ranges_request.id && !message.accounts.is_empty() =>
    //             {
    //                 info!("[SYNC] Received {} Accounts", message.accounts.len());
    //             }

    //             // Bad peer response, lets try a different peer
    //             Ok(Some(_)) => info!("[Sync] Bad peer response"),
    //             // Reply timeouted/peer shut down, lets try a different peer
    //             _ => info!("[Sync] Peer response timeout( Snap Account Range)"),
    //         }
    //     }
    // }
}
