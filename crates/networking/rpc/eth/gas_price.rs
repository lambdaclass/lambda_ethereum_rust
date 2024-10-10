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
pub const BLOCK_RANGE_LOWER_BOUND_DEC: u64 = 20;

impl RpcHandler for GasPrice {
    fn parse(params: &Option<Vec<Value>>) -> Result<Self, RpcErr> {
        Ok(GasPrice {})
    }

    // TODO: Calculating gas price involves querying multiple blocks
    // and doing some calculations with each of them, let's consider
    // caching this result.
    // FIXME: Check diffs between legacy transaction, eip2930... etc.
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
            let Some(block_body) = storage.get_block_body(latest_block_number)? else {
                error!("Block body for block number {block_num} is missing but is below the latest known block!");
                return Err(RpcErr::Internal("Error calculating gas price".to_string()));
            };
            let Some(block_header) = storage.get_block_header(latest_block_number)? else {
                error!("Block header for block number {block_num} is missing but is below the latest known block!");
                return Err(RpcErr::Internal("Error calculating gas price".to_string()));
            };
            let base_fee = block_header.base_fee_per_gas;
            let mut txs_tips = block_body
                .transactions
                .into_iter()
                // Every transaction here should have a gas tip
                // since they're already accepted in a block.
                .filter_map(|tx| tx.effective_gas_tip(base_fee))
                .collect::<Vec<u64>>();

            txs_tips.sort();

            results.extend(txs_tips.into_iter().take(TXS_SAMPLE_SIZE));
        }
        // FIXME: Check for overflow here.
        // FIXME: Check if we need to add the base fee to this.
        let sample_gas = results
            .get(((results.len() - 1) * (TXS_SAMPLE_PERCENTILE / 100)))
            .ok_or(RpcErr::Internal("Error calculating gas price".to_string()))?;

        // FIXME: Return proper default value here, investigate
        // which would be appropiate
        if (*sample_gas as usize) > DEFAULT_MAX_PRICE_IN_WEI {
            todo!("")
        }
        // FIXME: Check sample gas is under limit
        let gas_as_hex = format!("0x{:x}", sample_gas);
        // FIXME: Check gas price unit, should be wei according to the spec.
        return Ok(serde_json::Value::from_str(&gas_as_hex)?);
    }
}

// FIXME: Test this with different block configs
#[cfg(test)]
mod tests {}
