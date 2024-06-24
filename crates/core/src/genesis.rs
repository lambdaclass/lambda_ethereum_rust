use crate::account::Account;
use ethereum_types::{Address, H256, U256};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Genesis {
    /// Chain configuration
    config: ChainConfig,
    /// The initial state of the accounts in the genesis block.
    alloc: HashMap<H256, Account>,
    /// Genesis header values
    coinbase: Address,
    difficulty: U256,
    extra_data: Vec<u8>,
    gas_limit: u64,
    nonce: u64,
    mix_hash: H256,
    timestamp: u64,
}

/// Blockchain settings defined per block
#[allow(unused)]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChainConfig {
    /// Current chain identifier
    chain_id: U256,

    /// Block numbers for the block where each fork was activated
    /// (None = no fork, 0 = fork is already active)
    homestead_block: Option<u64>,

    dao_fork_block: Option<u64>,
    /// Whether the nodes supports or opposes the DAO hard-fork
    #[serde(default)]
    dao_fork_support: bool,

    eip150_block: Option<u64>,
    eip155_block: Option<u64>,
    eip158_block: Option<u64>,

    byzantinum_block: Option<u64>,
    constantinople_block: Option<u64>,
    petersburg_block: Option<u64>,
    instanbul_block: Option<u64>,
    muir_glacier_block: Option<u64>,
    berlin_block: Option<u64>,
    london_block: Option<u64>,
    arrow_glacier_block: Option<u64>,
    gray_glacier_block: Option<u64>,
    merge_netsplit_block: Option<u64>,

    /// Timestamp at which each fork was activated
    /// (None = no fork, 0 = fork is already active)
    shangai_time: Option<u64>,
    cancun_time: Option<u64>,
    prague_time: Option<u64>,
    verkle_time: Option<u64>,

    /// Amount of total difficulty reached by the network that triggers the consensus upgrade.
    terminal_total_difficulty: Option<U256>,
    /// Network has already passed the terminal total difficult
    #[serde(default)]
    terminal_total_difficulty_passed: bool,
}
