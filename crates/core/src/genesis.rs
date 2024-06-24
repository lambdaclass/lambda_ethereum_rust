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
    homestead_block: Option<U256>,

    dao_fork_block: Option<U256>,
    #[serde(default)]
    dao_fork_support: bool,

    eip150_block: Option<U256>,
    eip155_block: Option<U256>,
    eip158_block: Option<U256>,

    byzantinum_block: Option<U256>,
    constantinople_block: Option<U256>,
    petersburg_block: Option<U256>,
    instanbul_block: Option<U256>,
    muir_glacier_block: Option<U256>,
    berlin_block: Option<U256>,
    london_block: Option<U256>,
    arrow_glacier_block: Option<U256>,
    gray_glacier_block: Option<U256>,
    merge_netsplit_block: Option<U256>,

    shangai_time: Option<u64>,
    cancun_time: Option<u64>,
    prague_time: Option<u64>,
    verkle_time: Option<u64>
}

pub type Account = u32; // TODO(placeholder)
