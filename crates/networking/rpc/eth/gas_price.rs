use std::{
    collections::HashMap,
    str::FromStr,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use ethereum_rust_core::types::{BlockBody, BlockHeader, BlockNumber};
use ethereum_rust_storage::Store;
use tracing::error;

use crate::RpcHandler;
use crate::{
    eth::gas_price,
    utils::{RpcErr, RpcRequest},
};
use rand::prelude::*;
use serde_json::{json, Value};

use super::logs::LogsFilter;

#[derive(Debug, Clone)]
pub struct GasPrice {}

// TODO: Maybe these constants should be some kind of config.
// How many transactions to take as a price sample from a block.
const TXS_SAMPLE_SIZE: usize = 3;
// How many blocks we'll go back to calculate the estimate.
const BLOCK_RANGE_LOWER_BOUND_DEC: u64 = 20;

impl RpcHandler for GasPrice {
    fn parse(params: &Option<Vec<Value>>) -> Result<Self, RpcErr> {
        Ok(GasPrice {})
    }

    // Disclaimer:
    // This estimation is based on how currently go-ethereum does it currently.
    // The idea here is to:
    // - Take the last 20 blocks.
    // - For each block, take the 3 with the lower gas price.
    // - Join them all into a single vec and sort it.
    // - Return the one in the middle (what is also known as the 'median sample')
    // This specific implementation is probably is not the best way to do this
    // but it works for now for a simple estimation, in the future
    // we can look into more sophisticated estimation methods, if needed.
    /// Estimate Gas Price based on already accepted transactions.
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

        // TODO: Calculating gas price involves querying multiple blocks
        // and doing some calculations with each of them, let's consider
        // caching this result, also we can have a specific DB method
        // that returns a block range.
        for block_num in block_range {
            let Some(block_body) = storage.get_block_body(block_num)? else {
                error!("Block body for block number {block_num} is missing but is below the latest known block!");
                return Err(RpcErr::Internal(
                    "Error calculating gas price: missing data".to_string(),
                ));
            };
            let Some(block_header) = storage.get_block_header(block_num)? else {
                error!("Block header for block number {block_num} is missing but is below the latest known block!");
                return Err(RpcErr::Internal(
                    "Error calculating gas price: missing data".to_string(),
                ));
            };
            let base_fee = block_header.base_fee_per_gas;
            let mut txs_tips = block_body
                .transactions
                .into_iter()
                .filter_map(|tx| Some(tx.gas_price()))
                .collect::<Vec<u64>>();
            txs_tips.sort();
            results.extend(txs_tips.into_iter().take(TXS_SAMPLE_SIZE));
        }
        results.sort();
        if results.len() == 0 {
            return Err(RpcErr::Internal(
                "Error calculating gas price: could not find samples".to_string(),
            ));
        } else {
            let sample_gas = results
                .get(results.len() / 2)
                .ok_or(RpcErr::Internal("Error calculating gas price".to_string()))?;

            // TODO: Investigate which gas price could be an appropiate default here.
            // if (*sample_gas as usize) > default_max_price_in_wei {
            //     todo!("")
            // }
            let gas_as_hex = format!("0x{:x}", sample_gas);
            return Ok(serde_json::Value::String(gas_as_hex));
        }
    }
}

// FIXME: Test this with different block configs
#[cfg(test)]
mod tests {
    use super::GasPrice;
    use crate::{utils::parse_json_hex, RpcHandler};
    use bytes::Bytes;
    use ethereum_rust_core::{
        types::{
            Block, BlockBody, BlockHeader, EIP1559Transaction, Genesis, LegacyTransaction,
            Transaction, TxKind,
        },
        Address, Bloom, H256, U256,
    };
    use ethereum_rust_storage::{EngineType, Store};
    use hex_literal::hex;
    use std::str::FromStr;
    // Base price for each test transaction.
    const BASE_PRICE_IN_WEI: u64 = (10_u64.pow(9));
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
    fn legacy_tx_for_test(nonce: u64) -> Transaction {
        Transaction::LegacyTransaction(LegacyTransaction {
            nonce,
            gas_price: nonce * BASE_PRICE_IN_WEI,
            gas: 10000,
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
        })
    }
    fn eip1559_tx_for_test(nonce: u64) -> Transaction {
        Transaction::EIP1559Transaction(EIP1559Transaction {
            chain_id: 1,
            nonce: nonce,
            max_fee_per_gas: nonce * BASE_PRICE_IN_WEI,
            max_priority_fee_per_gas: (nonce * (10_u64.pow(9))).pow(2),
            gas_limit: 10000,
            to: TxKind::Create,
            value: 100.into(),
            data: Default::default(),
            access_list: vec![],
            signature_y_parity: true,
            signature_r: U256::default(),
            signature_s: U256::default(),
        })
    }
    fn setup_store() -> Store {
        let genesis: &str = include_str!("../../../../test_data/test-config.json");
        let genesis: Genesis =
            serde_json::from_str(genesis).expect("Fatal: test config is invalid");
        let mut store = Store::new("test-store", EngineType::InMemory)
            .expect("Fail to create in-memory db test");
        store.add_initial_state(genesis);
        return store;
    }
    #[test]
    fn test_for_legacy_txs() {
        let mut store = setup_store();
        for block_num in 1..100 {
            let mut txs = vec![];
            for nonce in 1..=3 {
                let legacy_tx = legacy_tx_for_test(nonce);
                txs.push(legacy_tx)
            }
            let block_body = BlockBody {
                transactions: txs,
                ommers: Default::default(),
                withdrawals: Default::default(),
            };
            let block_header = test_header(block_num);
            let block = Block {
                body: block_body,
                header: block_header.clone(),
            };
            store.add_block(block).unwrap();
            store.set_canonical_block(block_num, block_header.compute_block_hash());
        }
        let gas_price = GasPrice {};
        let response = gas_price.handle(store).unwrap();
        let parsed_result = parse_json_hex(&response).unwrap();
        assert_eq!(parsed_result, 2000000000);
    }

    #[test]
    fn test_for_eip_1559_txs() {
        let mut store = setup_store();
        for i in 1..100 {
            let mut txs = vec![];
            for nonce in 1..=3 {
                txs.push(eip1559_tx_for_test(nonce));
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
        let parsed_result = parse_json_hex(&response).unwrap();
        assert_eq!(parsed_result, 2000000000);
    }
    #[test]
    fn test_with_mixed_transactions() {
        let mut store = setup_store();
        for i in 1..100 {
            let mut txs = vec![];
            txs.push(legacy_tx_for_test(1));
            txs.push(eip1559_tx_for_test(2));
            txs.push(legacy_tx_for_test(3));
            txs.push(eip1559_tx_for_test(3));
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
        let parsed_result = parse_json_hex(&response).unwrap();
        assert_eq!(parsed_result, 2000000000);
    }
}
