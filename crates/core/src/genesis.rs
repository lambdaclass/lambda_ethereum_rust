use std::collections::HashMap;

use ethereum_types::{H256, U256};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Genesis {
    config: ChainConfig,
    alloc: HashMap<H256, Account>,
    coinbase: H256,
    difficulty: U256,
    extra_data: Vec<u8>,
    gas_limit: u64,
    nonce: u64,
    mix_hash: H256,
    timestamp: u64,
}

#[allow(unused)]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChainConfig {
    chain_id: U256,
    homestead_block: Option<u64>,

    dao_fork_block: Option<u64>,
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

    shangai_time: Option<u64>,
    cancun_time: Option<u64>,
    prague_time: Option<u64>,
    verkle_time: Option<u64>,

    terminal_total_difficulty: Option<U256>,
    #[serde(default)]
    terminal_total_difficulty_passed: bool,
}

pub type Account = u32; // TODO(placeholder)
