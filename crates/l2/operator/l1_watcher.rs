use crate::utils::{config::l1_watcher::L1WatcherConfig, eth_client::EthClient};
use ethereum_types::{Address, H256, U256};
use std::{cmp::min, time::Duration};
use tokio::time::sleep;
use tracing::debug;

pub async fn start_l1_watcher() {
    let config = L1WatcherConfig::from_env().unwrap();
    let l1_watcher = L1Watcher::new_from_config(config);
    l1_watcher.get_logs().await;
}

pub struct L1Watcher {
    rpc_url: String,
    address: Address,
    topics: Vec<H256>,
    check_interval: Duration,
}

impl L1Watcher {
    pub fn new_from_config(config: L1WatcherConfig) -> Self {
        Self {
            rpc_url: config.rpc_url,
            address: config.bridge_address,
            topics: config.topics,
            check_interval: Duration::from_millis(config.check_interval_ms),
        }
    }

    pub async fn get_logs(&self) {
        let step = U256::from(5000);

        let mut last_block: U256 = U256::zero();

        let l1_rpc = EthClient::new(&self.rpc_url);

        loop {
            let current_block = l1_rpc.get_block_number().await.unwrap();
            debug!(
                "Current block number: {} ({:#x})",
                current_block, current_block
            );
            let new_last_block = min(last_block + step, current_block);
            debug!(
                "Looking logs from block {:#x} to {:#x}",
                last_block, new_last_block
            );

            let logs = l1_rpc
                .get_logs(last_block, new_last_block, self.address, self.topics[0])
                .await;

            debug!("Logs: {:#?}", logs);

            last_block = new_last_block + 1;
            sleep(self.check_interval).await;
        }
    }
}
