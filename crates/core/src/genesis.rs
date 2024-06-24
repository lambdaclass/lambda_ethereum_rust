use crate::account::Account;
use ethereum_types::{Address, H256, U256};
use serde::Deserialize;
use std::collections::HashMap;

#[allow(unused)]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Genesis {
    /// Chain configuration
    pub config: ChainConfig,
    /// The initial state of the accounts in the genesis block.
    pub alloc: HashMap<Address, Account>,
    /// Genesis header values
    pub coinbase: Address,
    pub difficulty: U256,
    pub extra_data: Vec<u8>,
    pub gas_limit: u64,
    pub nonce: u64,
    pub mix_hash: H256,
    pub timestamp: u64,
}

/// Blockchain settings defined per block
#[allow(unused)]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChainConfig {
    /// Current chain identifier
    pub chain_id: U256,

    /// Block numbers for the block where each fork was activated
    /// (None = no fork, 0 = fork is already active)
    pub homestead_block: Option<u64>,

    pub dao_fork_block: Option<u64>,
    /// Whether the nodes supports or opposes the DAO hard-fork
    #[serde(default)]
    pub dao_fork_support: bool,

    pub eip150_block: Option<u64>,
    pub eip155_block: Option<u64>,
    pub eip158_block: Option<u64>,

    pub byzantinum_block: Option<u64>,
    pub constantinople_block: Option<u64>,
    pub petersburg_block: Option<u64>,
    pub instanbul_block: Option<u64>,
    pub muir_glacier_block: Option<u64>,
    pub berlin_block: Option<u64>,
    pub london_block: Option<u64>,
    pub arrow_glacier_block: Option<u64>,
    pub gray_glacier_block: Option<u64>,
    pub merge_netsplit_block: Option<u64>,

    /// Timestamp at which each fork was activated
    /// (None = no fork, 0 = fork is already active)
    pub shangai_time: Option<u64>,
    pub cancun_time: Option<u64>,
    pub prague_time: Option<u64>,
    pub verkle_time: Option<u64>,

    /// Amount of total difficulty reached by the network that triggers the consensus upgrade.
    pub terminal_total_difficulty: Option<U256>,
    /// Network has already passed the terminal total difficult
    #[serde(default)]
    pub terminal_total_difficulty_passed: bool,
}
