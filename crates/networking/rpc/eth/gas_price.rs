use std::{
    collections::HashMap,
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

pub const DEFAULT_MAX_PRICE_IN_WEI: usize = 500 * (10_usize.pow(9));
// The limit for a gas estimation
pub const DEFAULT_IGNORE_PRICE: usize = 500 * (10_usize.pow(9));
// How many transactions to take as a sample from a block
// to give a gas price estimation.
pub const TXS_SAMPLE_SIZE: usize = 3;
// Determines which transaction from the sample will
// be taken as a reference for the gas price.
pub const TXS_SAMPLE_PERCENTILE: usize = 60;

impl RpcHandler for GasPrice {
    fn parse(params: &Option<Vec<Value>>) -> Result<Self, RpcErr> {
        Ok(GasPrice {})
    }

    fn call(req: &RpcRequest, storage: Store) -> Result<Value, RpcErr> {
        let request = Self::parse(&req.params)?;
        request.handle(storage)
    }

    // TODO: Calculating gas price involves querying multiple blocks
    // and doing some calculations with each of them, let's consider
    // caching this result.
    // FIXME: Check diffs between legacy transaction, eip2930... etc.
    fn handle(&self, storage: Store) -> Result<Value, RpcErr> {
        // FIXME: Handle None values (i.e. remove unwraps before PR review)
        let latest_block_number = storage.get_latest_block_number()?.unwrap();
        let block_range = latest_block_number.wrapping_sub(20)..=latest_block_number;
        // FIXME: Handle this case before PR review
        if block_range.is_empty() {
            todo!("Block range is empty")
        }
        let mut results = vec![];
        for block_num in block_range {
            // FIXME: Handle None values (i.e. remove unwraps before PR review)
            let mut block_body = storage.get_block_body(latest_block_number)?.unwrap();
            let block_header = storage.get_block_header(latest_block_number)?.unwrap();
            let base_fee = latest_block_header.base_fee_per_gas;
            let mut txs_tips = block_body
                .transactions
                .into_iter()
                // Every transaction here should have a gas tip
                // since they're already accepted in a block.
                .filter_map(|tx| tx.effective_gas_tip(base_fee))
                .collect::<Vec<u64>>();

            txs_tips.sort();

            results.extend(txs_.into_iter().take(TXS_SAMPLE_SIZE));
        }
        // FIXME: Properly handle this error before PR review.
        // FIXME: Check for overflow here.
        let sample_gas = results
            .get(((results.len() - 1) * (TXS_SAMPLE_PERCENTILE / 100)))
            .ok_or(RpcErr::Internal("".to_string()))?;
        // FIXME: Check sample gas is under limit
        return Ok(format!("0x{:x}", sample_gas));
    }
}
