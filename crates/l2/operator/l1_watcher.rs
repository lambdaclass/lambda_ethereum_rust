use crate::utils::{
    config::{eth::EthConfig, l1_watcher::L1WatcherConfig},
    eth_client::EthClient,
};
use ethereum_types::{Address, H256, U256};
use std::{cmp::min, time::Duration};
use tokio::time::sleep;
use tracing::debug;

pub async fn start_l1_watcher() {
    let eth_config = EthConfig::from_env().unwrap();
    let watcher_config = L1WatcherConfig::from_env().unwrap();
    let l1_watcher = L1Watcher::new_from_config(watcher_config, eth_config);
    l1_watcher.get_logs().await;
}

pub struct L1Watcher {
    rpc_url: String,
    address: Address,
    topics: Vec<H256>,
    check_interval: Duration,
    max_block_step: U256,
}

impl L1Watcher {
    pub fn new_from_config(watcher_config: L1WatcherConfig, eth_config: EthConfig) -> Self {
        Self {
            rpc_url: eth_config.rpc_url,
            address: watcher_config.bridge_address,
            topics: watcher_config.topics,
            check_interval: Duration::from_millis(watcher_config.check_interval_ms),
            max_block_step: watcher_config.max_block_step,
        }
    }

    pub async fn get_logs(&self) {
        let mut last_block: U256 = U256::zero();

        let l1_rpc = EthClient::new(&self.rpc_url);

        loop {
            let current_block = l1_rpc.get_block_number().await.unwrap();
            debug!(
                "Current block number: {} ({:#x})",
                current_block, current_block
            );
            let new_last_block = min(last_block + self.max_block_step, current_block);
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
