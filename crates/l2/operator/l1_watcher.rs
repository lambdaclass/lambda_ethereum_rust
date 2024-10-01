use crate::rpc::l1_rpc::L1Rpc;
use ethereum_types::{Address, H256, U256};
use std::{cmp::min, time::Duration};
use tokio::time::sleep;
use tracing::debug;

pub async fn start_l1_watcher() {
    // This address and topic were used for testing purposes only.
    // TODO: Receive them as parameters from config.
    let l1_watcher = L1Watcher::new(
        "0xe441CF0795aF14DdB9f7984Da85CD36DB1B8790d"
            .parse()
            .unwrap(),
        vec![
            "0xe2ea736f80f92a510d75d1a96b5c1d5e5544283362b7acd97390d60ea1c7d149"
                .parse()
                .unwrap(),
        ],
    );
    l1_watcher.get_logs().await;
}

pub struct L1Watcher {
    address: Address,
    topics: Vec<H256>,
}

impl L1Watcher {
    pub fn new(address: Address, topics: Vec<H256>) -> Self {
        Self { address, topics }
    }

    pub async fn get_logs(&self) {
        let step = U256::from(5000);

        let mut last_block: U256 = U256::zero();

        let l1_rpc = L1Rpc::new("http://localhost:8545");

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
            sleep(Duration::from_secs(5)).await;
        }
    }
}
