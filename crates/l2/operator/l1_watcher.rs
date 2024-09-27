use std::{cmp::min, time::Duration};

use ethereum_types::U256;
use tokio::time::sleep;

use crate::rpc::l1_rpc::L1Rpc;

pub async fn start_l1_watcher() {
    L1Watcher::get_logs().await;
}

pub struct L1Watcher;

impl L1Watcher {
    pub async fn get_logs() {
        let step = U256::from(1000);

        let mut last_block: U256 = U256::zero();

        let l1_rpc = L1Rpc::new("http://localhost:8545");

        loop {
            let current_block = l1_rpc.get_block_number().await.unwrap();
            println!(
                "Current block number: {} ({:#x})",
                current_block, current_block
            );
            let new_last_block = min(last_block + step, current_block);
            println!("From {:#x} to {:#x}", last_block, new_last_block);

            let logs = l1_rpc
                .get_logs(
                    last_block,
                    new_last_block,
                    "0xe441CF0795aF14DdB9f7984Da85CD36DB1B8790d"
                        .parse()
                        .unwrap(),
                    "0xe2ea736f80f92a510d75d1a96b5c1d5e5544283362b7acd97390d60ea1c7d149"
                        .parse()
                        .unwrap(),
                )
                .await;

            println!("Logs: {:#?}", logs);

            last_block = new_last_block + 1;
            sleep(Duration::from_secs(5)).await;
        }
    }
}
