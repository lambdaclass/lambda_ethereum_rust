use std::sync::Arc;

use ethrex_core::{
    types::{validate_block_header, Block, BlockHash, BlockHeader, InvalidBlockHeaderError},
    H256,
};
use ethrex_storage::Store;
use tokio::sync::Mutex;
use tracing::info;

use crate::kademlia::KademliaTable;

/// Manager in charge the sync process
/// Only performs full-sync but will also be in charge of snap-sync in the future
#[derive(Debug)]
pub struct SyncManager {
    // true: syncmode = snap, false = syncmode = full
    #[allow(unused)]
    snap_mode: bool,
    peers: Arc<Mutex<KademliaTable>>,
}

impl SyncManager {
    pub fn new(peers: Arc<Mutex<KademliaTable>>, snap_mode: bool) -> Self {
        Self { snap_mode, peers }
    }

    /// Starts a sync cycle, updating the state with all blocks between the current head and the sync head
    /// TODO: only uses full sync, should also process snap sync once implemented
    pub async fn start_sync(&mut self, mut current_head: H256, sync_head: H256, store: Store) {
        info!("Syncing from current head {current_head} to sync_head {sync_head}");
        let start_time = Instant::now();
        // Request all block headers between the current head and the sync head
        // We will begin from the current head so that we download the earliest state first
        // This step is not parallelized
        let mut all_block_headers = vec![];
        let mut all_block_hashes = vec![];
        loop {
            let peer = self.peers.lock().await.get_peer_channels().await;
            info!("Requesting Block Headers from {current_head}");
            // Request Block Headers from Peer
            if let Some(block_headers) = peer.request_block_headers(current_head).await {
                info!("Received block headers {}..{}", block_headers.first().unwrap(), block_headers.last().unwrap());
                // Keep headers so we can process them later
                // Discard the first header as we already have it
                all_block_headers.extend_from_slice(&block_headers[1..]);
                all_block_hashes.extend_from_slice(&block_hashes[1..]);

                // Check if we already reached our sync head or if we need to fetch more blocks
                if !block_hashes.contains(&sync_head) {
                    // Update the request to fetch the next batch
                    current_head = (*block_hashes.last().unwrap()).into();
                } else {
                    // No more headers to request
                    break;
                }
            }
            info!("Peer response timeout (Headers)");
        }
        info!("All headers fetched");
        // We finished fetching all headers, now we can process them
        // TODO: snap-sync: launch tasks to fetch blocks and state in parallel
        // full-sync: Fetch all block bodies and execute them sequentially to build the state
        tokio::spawn(download_and_run_blocks(
            all_block_hashes,
            all_block_headers,
            self.peers.clone(),
            store.clone(),
        ))
        .await;
        info!("Sync finished, time elapsed: {} ", start_time.elapsed());
    }

    /// Creates a dummy SyncManager for tests where syncing is not needed
    /// This should only be used in tests as it won't be able to connect to the p2p network
    pub fn dummy() -> Self {
        let dummy_peer_table = Arc::new(Mutex::new(KademliaTable::new(Default::default())));
        Self {
            snap_mode: false,
            peers: dummy_peer_table,
        }
    }
}

async fn download_and_run_blocks(
    mut block_hashes: Vec<BlockHash>,
    mut block_headers: Vec<BlockHeader>,
    peers: Arc<Mutex<KademliaTable>>,
    store: Store,
) -> Resu{
    loop {
        let peer = peers.lock().await.get_peer_channels().await;
        info!("Requesting Block Bodies ");
        if let Some(block_bodies) = peer.request_block_bodies(block_hashes.clone()).await {
            info!("Received {} Block Bodies", block_bodies.len());
            // Execute and store blocks
            for body in block_bodies {
                // We already validated that there are no more block bodies than the ones requested
                let header = block_headers.remove(0);
                let hash = block_hashes.remove(0);
                let block = Block::new(header, body);
                ethrex_blockchain::add_block(&block, &store);
            }
            info!("Executed & stored {} blocks", block_bodies.len());
            // Check if we need to ask for another batch
            if block_hashes.is_empty() {
                break;
            }
        }
        info!("Peer response timeout(Blocks)");
    }
}
