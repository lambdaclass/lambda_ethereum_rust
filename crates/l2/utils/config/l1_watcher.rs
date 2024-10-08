use ethereum_types::{Address, H256, U256};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct L1WatcherConfig {
    pub bridge_address: Address,
    pub topics: Vec<H256>,
    pub check_interval_ms: u64,
    pub max_block_step: U256,
}

impl L1WatcherConfig {
    pub fn from_env() -> Result<Self, String> {
        match envy::prefixed("L1_WATCHER_").from_env::<Self>() {
            Ok(config) => Ok(config),
            Err(error) => Err(error.to_string()),
        }
    }
}
