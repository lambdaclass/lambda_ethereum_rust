use std::{
    collections::HashMap,
    str::FromStr,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use ethereum_rust_core::types::{BlockBody, BlockHeader, BlockNumber};
use ethereum_rust_storage::Store;
use tracing::error;

use crate::utils::{RpcErr, RpcRequest};
use crate::RpcHandler;
use rand::prelude::*;
use serde_json::{json, Value};

use super::logs::LogsFilter;

#[derive(Debug, Clone)]
pub struct GasPrice {}

// TODO: This should be some kind of configuration.
// The default limit for a gas estimation.
pub const DEFAULT_MAX_PRICE_IN_WEI: usize = 500 * (10_usize.pow(9));
// The limit for a gas estimation
pub const DEFAULT_IGNORE_PRICE: usize = 500 * (10_usize.pow(9));
// How many transactions to take as a sample from a block
// to give a gas price estimation.
pub const TXS_SAMPLE_SIZE: usize = 3;
// Determines which transaction from the sample will
// be taken as a reference for the gas price.
pub const TXS_SAMPLE_PERCENTILE: usize = 60;

// How many blocks we'll go back to calculate the estimate.
// pub const BLOCK_RANGE_LOWER_BOUND_DEC: u64 = 20;
pub const BLOCK_RANGE_LOWER_BOUND_DEC: u64 = 3;

impl RpcHandler for GasPrice {
    fn parse(params: &Option<Vec<Value>>) -> Result<Self, RpcErr> {
        Ok(GasPrice {})
    }

    // TODO: Calculating gas price involves querying multiple blocks
    // and doing some calculations with each of them, let's consider
    // caching this result.
    // FIXME: Check diffs between legacy transaction, eip2930... etc.
    // Disclaimer:
    // This estimation is based on how currently go-ethereum does it currently.
    fn handle(&self, storage: Store) -> Result<Value, RpcErr> {
        let Some(latest_block_number) = storage.get_latest_block_number()? else {
            error!("FATAL: LATEST BLOCK NUMBER IS MISSING");
            return Err(RpcErr::Internal("Error calculating gas price".to_string()));
        };
        let block_range_lower_bound =
            latest_block_number.saturating_sub(BLOCK_RANGE_LOWER_BOUND_DEC);
        // These are the blocks we'll use to estimate the price.
        let block_range = block_range_lower_bound..=latest_block_number;
        if block_range.is_empty() {
            error!(
                "Calculated block range from block {} \
                    up to block {} for gas price estimation is empty",
                block_range_lower_bound, latest_block_number
            );
            return Err(RpcErr::Internal("Error calculating gas price".to_string()));
        }
        let mut results = vec![];
        for block_num in block_range {
            let Some(block_body) = storage.get_block_body(block_num)? else {
                error!("Block body for block number {block_num} is missing but is below the latest known block!");
                return Err(RpcErr::Internal("Error calculating gas price".to_string()));
            };
            let Some(block_header) = storage.get_block_header(block_num)? else {
                error!("Block header for block number {block_num} is missing but is below the latest known block!");
                return Err(RpcErr::Internal("Error calculating gas price".to_string()));
            };
            let base_fee = block_header.base_fee_per_gas;
            let mut txs_tips = block_body
                .transactions
                .into_iter()
                .filter_map(|tx| tx.effective_gas_tip(base_fee))
                .collect::<Vec<u64>>();

            txs_tips.sort();

            results.extend(txs_tips.into_iter().take(TXS_SAMPLE_SIZE));
        }
        // FIXME: Check for overflow here.
        // FIXME: Check if we need to add the base fee to this.
        dbg!(&results);
        let sample_gas = results
            .get(((results.len() - 1) * (TXS_SAMPLE_PERCENTILE / 100)))
            .ok_or(RpcErr::Internal("Error calculating gas price".to_string()))?;

        // FIXME: Return proper default value here, investigate
        // which would be appropiate
        if (*sample_gas as usize) > DEFAULT_MAX_PRICE_IN_WEI {
            todo!("")
        }
        let gas_as_hex = format!("0x{:x}", sample_gas);
        // FIXME: Check gas price unit, should be wei according to the spec.
        return Ok(serde_json::Value::String(gas_as_hex));
    }
}

// FIXME: Test this with different block configs
#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use ethereum_rust_core::{
        types::{Block, BlockBody, BlockHeader, Genesis, LegacyTransaction, Transaction, TxKind},
        Address, Bloom, H256, U256,
    };
    use ethereum_rust_storage::{EngineType, Store};
    use hex_literal::hex;
    use std::str::FromStr;

    use crate::{utils::parse_json_hex, RpcHandler};

    use super::GasPrice;
    fn test_header(block_num: u64) -> BlockHeader {
        BlockHeader {
            parent_hash: H256::from_str(
                "0x1ac1bf1eef97dc6b03daba5af3b89881b7ae4bc1600dc434f450a9ec34d44999",
            )
            .unwrap(),
            ommers_hash: H256::from_str(
                "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
            )
            .unwrap(),
            coinbase: Address::from_str("0x2adc25665018aa1fe0e6bc666dac8fc2697ff9ba").unwrap(),
            state_root: H256::from_str(
                "0x9de6f95cb4ff4ef22a73705d6ba38c4b927c7bca9887ef5d24a734bb863218d9",
            )
            .unwrap(),
            transactions_root: H256::from_str(
                "0x578602b2b7e3a3291c3eefca3a08bc13c0d194f9845a39b6f3bcf843d9fed79d",
            )
            .unwrap(),
            receipts_root: H256::from_str(
                "0x035d56bac3f47246c5eed0e6642ca40dc262f9144b582f058bc23ded72aa72fa",
            )
            .unwrap(),
            logs_bloom: Bloom::from([0; 256]),
            difficulty: U256::zero(),
            number: block_num,
            gas_limit: 0x016345785d8a0000,
            gas_used: 0xa8de,
            timestamp: 0x03e8,
            extra_data: Bytes::new(),
            prev_randao: H256::zero(),
            nonce: 0x0000000000000000,
            base_fee_per_gas: None,
            withdrawals_root: Some(
                H256::from_str(
                    "0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421",
                )
                .unwrap(),
            ),
            blob_gas_used: Some(0x00),
            excess_blob_gas: Some(0x00),
            parent_beacon_block_root: Some(H256::zero()),
        }
    }
    #[test]
    fn test_for_gen() {
        let genesis: &str = include_str!("../../../../test_data/test-config.json");
        let genesis: Genesis =
            serde_json::from_str(genesis).expect("Fatal: test config is invalid");
        let mut store = Store::new("test-store", EngineType::InMemory)
            .expect("Fail to create in-memory db test");
        let genesis_block = genesis.get_block();
        store.add_initial_state(genesis);
        for i in 1..32 {
            let mut txs = vec![];
            for j in 0..7 {
                let legacy_tx = Transaction::LegacyTransaction(LegacyTransaction {
                    nonce: j,
                    gas_price: (j + 1) * (10_u64.pow(9)),
                    gas: 21000,
                    to: TxKind::Create,
                    value: 100.into(),
                    data: Default::default(),
                    v: U256::from(0x1b),
                    r: U256::from(hex!(
                        "7e09e26678ed4fac08a249ebe8ed680bf9051a5e14ad223e4b2b9d26e0208f37"
                    )),
                    s: U256::from(hex!(
                        "5f6e3f188e3e6eab7d7d3b6568f5eac7d687b08d307d3154ccd8c87b4630509b"
                    )),
                });
                dbg!(legacy_tx.gas_price());
                txs.push(legacy_tx)
            }
            let block_body = BlockBody {
                transactions: txs,
                ommers: Default::default(),
                withdrawals: Default::default(),
            };
            let block_header = test_header(i);
            let block = Block {
                body: block_body,
                header: block_header.clone(),
            };
            store.add_block(block).unwrap();
            store.set_canonical_block(i, block_header.compute_block_hash());
        }
        let gas_price = GasPrice {};
        let response = gas_price.handle(store).unwrap();
        dbg!(parse_json_hex(&response));
    }
}
