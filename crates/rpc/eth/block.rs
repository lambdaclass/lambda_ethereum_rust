use std::fmt::Display;

use ethereum_rust_chain::{constants::MAX_BLOB_GAS_PER_BLOCK, find_parent_header};
use ethereum_rust_core::types::{Block, Transaction};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::info;

use crate::{
    types::{
        block::RpcBlock,
        receipt::{RpcReceipt, RpcReceiptBlockInfo, RpcReceiptTxInfo},
    },
    utils::RpcErr,
    RpcHandler,
};
use ethereum_rust_core::types::{
    calculate_base_fee_per_blob_gas, BlockBody, BlockHash, BlockHeader, BlockNumber,
};
use ethereum_rust_storage::{error::StoreError, Store};

use super::account::BlockIdentifierOrHash;

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FeeHistoryRequest {
    #[serde(with = "ethereum_rust_core::serde_utils::u64::hex_str")]
    pub block_count: u64,
    pub newest_block: BlockIdentifier,
    pub reward_percentiles: Option<Vec<f32>>,
}

#[derive(Serialize, Default, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FeeHistoryResponse {
    pub oldest_block: String,
    pub base_fee_per_gas: Vec<String>,
    pub base_fee_per_blob_gas: Vec<String>,
    pub gas_used_ratio: Vec<f64>,
    pub blob_gas_used_ratio: Vec<f64>,
    pub reward: Vec<Vec<String>>,
}

pub struct GetBlockByNumberRequest {
    pub block: BlockIdentifier,
    pub hydrated: bool,
}

pub struct GetBlockByHashRequest {
    pub block: BlockHash,
    pub hydrated: bool,
}

pub struct GetBlockTransactionCountRequest {
    pub block: BlockIdentifierOrHash,
}

pub struct GetBlockReceiptsRequest {
    pub block: BlockIdentifierOrHash,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum BlockIdentifier {
    #[serde(with = "ethereum_rust_core::serde_utils::u64::hex_str")]
    Number(BlockNumber),
    Tag(BlockTag),
}

#[derive(Deserialize, Default, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum BlockTag {
    Earliest,
    Finalized,
    Safe,
    #[default]
    Latest,
    Pending,
}

impl BlockIdentifier {
    pub fn resolve_block_number(&self, storage: &Store) -> Result<Option<BlockNumber>, StoreError> {
        match self {
            BlockIdentifier::Number(num) => Ok(Some(*num)),
            BlockIdentifier::Tag(tag) => match tag {
                BlockTag::Earliest => storage.get_earliest_block_number(),
                BlockTag::Finalized => storage.get_finalized_block_number(),
                BlockTag::Safe => storage.get_safe_block_number(),
                BlockTag::Latest => storage.get_latest_block_number(),
                BlockTag::Pending => storage.get_pending_block_number(),
            },
        }
    }
}

impl RpcHandler for GetBlockByNumberRequest {
    fn parse(params: &Option<Vec<Value>>) -> Option<GetBlockByNumberRequest> {
        let params = params.as_ref()?;
        if params.len() != 2 {
            return None;
        };
        Some(GetBlockByNumberRequest {
            block: serde_json::from_value(params[0].clone()).ok()?,
            hydrated: serde_json::from_value(params[1].clone()).ok()?,
        })
    }
    fn handle(&self, storage: Store) -> Result<Value, RpcErr> {
        info!("Requested block with number: {}", self.block);
        let block_number = match self.block.resolve_block_number(&storage)? {
            Some(block_number) => block_number,
            _ => return Ok(Value::Null),
        };
        let header = storage.get_block_header(block_number)?;
        let body = storage.get_block_body(block_number)?;
        let (header, body) = match (header, body) {
            (Some(header), Some(body)) => (header, body),
            // Block not found
            _ => return Ok(Value::Null),
        };
        let hash = header.compute_block_hash();
        let block = RpcBlock::build(header, body, hash, self.hydrated);

        serde_json::to_value(&block).map_err(|_| RpcErr::Internal)
    }
}

impl RpcHandler for FeeHistoryRequest {
    fn parse(params: &Option<Vec<Value>>) -> Option<FeeHistoryRequest> {
        let params = params.as_ref()?;
        if params.len() < 2 || params.len() > 3 {
            return None;
        };

        let reward_percentiles = match params.get(2).cloned() {
            Some(rp) => {
                let rp: Vec<f32> = serde_json::from_value(rp).ok()?;
                let all_ok = rp
                    .windows(2)
                    .all(|w| w[0] <= w[1] || w[0] >= 0.0 && w[0] <= 100.0);
                // We want to return None if any value is wrong
                Some(all_ok.then_some(rp)?)
            }
            None => None,
        };

        let block_count_str: String = serde_json::from_value(params[0].clone()).ok()?;
        let block_count_str = block_count_str.strip_prefix("0x")?;

        Some(FeeHistoryRequest {
            block_count: u64::from_str_radix(block_count_str, 16).ok()?,
            newest_block: serde_json::from_value(params[1].clone()).ok()?,
            reward_percentiles,
        })
    }

    fn handle(&self, storage: Store) -> Result<Value, RpcErr> {
        info!(
            "Requested fee history for {} blocks starting from {}",
            self.block_count, self.newest_block
        );

        if self.block_count == 0 {
            return serde_json::to_value(FeeHistoryResponse::default())
                .map_err(|_| RpcErr::Internal);
        }

        let (start_block, end_block) =
            Self::get_range(&storage, self.block_count, &self.newest_block)?;
        let oldest_block = start_block;
        let block_count = (end_block - start_block) as usize;
        let mut base_fee_per_gas = Vec::<u64>::with_capacity(block_count + 1);
        let mut base_fee_per_blob_gas = Vec::<u64>::with_capacity(block_count + 1);
        let mut gas_used_ratio = Vec::<f64>::with_capacity(block_count);
        let mut blob_gas_used_ratio = Vec::<f64>::with_capacity(block_count);
        let mut reward = Vec::<Vec<u64>>::with_capacity(block_count);

        for block_number in start_block..end_block {
            let header = storage
                .get_block_header(block_number)?
                .ok_or(RpcErr::Internal)?;
            let body = storage
                .get_block_body(block_number)?
                .ok_or(RpcErr::Internal)?;
            let blob_base_fee = calculate_base_fee_per_blob_gas(&header);

            base_fee_per_gas.push(header.base_fee_per_gas);
            base_fee_per_blob_gas.push(blob_base_fee);
            gas_used_ratio.push(header.gas_used as f64 / header.gas_limit as f64);
            blob_gas_used_ratio.push(
                header.blob_gas_used.unwrap_or_default() as f64 / MAX_BLOB_GAS_PER_BLOCK as f64,
            );

            if let Some(percentiles) = &self.reward_percentiles {
                let block = Block { header, body };
                reward.push(Self::calculate_percentiles_for_block(block, percentiles));
            }
        }

        // Now we project base_fee_per_gas and base_fee_per_blob_gas from last block
        let header = storage
            .get_block_header(end_block)?
            .ok_or(RpcErr::Internal)?;

        let blob_base_fee = calculate_base_fee_per_blob_gas(&header);
        base_fee_per_gas.push(header.base_fee_per_gas);
        base_fee_per_blob_gas.push(blob_base_fee);

        let u64_to_hex_str = |x: u64| format!("0x{:x}", x);
        let response = FeeHistoryResponse {
            oldest_block: u64_to_hex_str(oldest_block),
            base_fee_per_gas: base_fee_per_gas.into_iter().map(u64_to_hex_str).collect(),
            base_fee_per_blob_gas: base_fee_per_blob_gas
                .into_iter()
                .map(u64_to_hex_str)
                .collect(),
            gas_used_ratio,
            blob_gas_used_ratio,
            reward: reward
                .into_iter()
                .map(|v| v.into_iter().map(u64_to_hex_str).collect())
                .collect(),
        };

        serde_json::to_value(response).map_err(|_| RpcErr::Internal)
    }
}

impl FeeHistoryRequest {
    fn get_range(
        storage: &Store,
        block_num: u64,
        finish_block: &BlockIdentifier,
    ) -> Result<(u64, u64), RpcErr> {
        // TODO: We should probably restrict how many blocks we are fetching to a certain limit

        // Get earliest block
        let earliest_block_num = storage
            .get_earliest_block_number()?
            .ok_or(RpcErr::Internal)?;

        // Get latest block
        let latest_block_num = storage.get_latest_block_number()?.ok_or(RpcErr::Internal)?;

        // Get finish_block number
        let finish_block = finish_block
            .resolve_block_number(storage)?
            .ok_or(RpcErr::Internal)?;

        // finish block has to be included in the range
        let finish_block = finish_block + 1;

        // Acomodate finish_block to be <= latest_block
        let finish_block = finish_block.min(latest_block_num);

        // Acomodate start_block to be >= earliest_block
        let start_block = earliest_block_num.max(finish_block.saturating_sub(block_num));

        Ok((start_block, finish_block))
    }

    fn calculate_percentiles_for_block(block: Block, percentiles: &[f32]) -> Vec<u64> {
        let base_fee_per_gas = block.header.base_fee_per_gas;
        let mut effective_priority_fees: Vec<u64> = block
            .body
            .transactions
            .into_iter()
            .map(|t: Transaction| match t {
                Transaction::LegacyTransaction(_) | Transaction::EIP2930Transaction(_) => 0,
                Transaction::EIP1559Transaction(t) => t
                    .max_priority_fee_per_gas
                    .min(t.max_fee_per_gas.saturating_sub(base_fee_per_gas)),
                Transaction::EIP4844Transaction(t) => t
                    .max_priority_fee_per_gas
                    .min(t.max_fee_per_gas.saturating_sub(base_fee_per_gas)),
            })
            .collect();

        effective_priority_fees.sort();
        let t_len = effective_priority_fees.len() as f32;

        percentiles
            .iter()
            .map(|x: &f32| {
                let i = (x * t_len / 100_f32) as usize;
                effective_priority_fees.get(i).cloned().unwrap_or_default()
            })
            .collect()
    }
}

impl RpcHandler for GetBlockByHashRequest {
    fn parse(params: &Option<Vec<Value>>) -> Option<GetBlockByHashRequest> {
        let params = params.as_ref()?;
        if params.len() != 2 {
            return None;
        };
        Some(GetBlockByHashRequest {
            block: serde_json::from_value(params[0].clone()).ok()?,
            hydrated: serde_json::from_value(params[1].clone()).ok()?,
        })
    }
    fn handle(&self, storage: Store) -> Result<Value, RpcErr> {
        info!("Requested block with hash: {}", self.block);
        let block_number = match storage.get_block_number(self.block)? {
            Some(number) => number,
            _ => return Ok(Value::Null),
        };
        let header = storage.get_block_header(block_number)?;
        let body = storage.get_block_body(block_number)?;
        let (header, body) = match (header, body) {
            (Some(header), Some(body)) => (header, body),
            // Block not found
            _ => return Ok(Value::Null),
        };
        let hash = header.compute_block_hash();
        let block = RpcBlock::build(header, body, hash, self.hydrated);
        serde_json::to_value(&block).map_err(|_| RpcErr::Internal)
    }
}

impl RpcHandler for GetBlockTransactionCountRequest {
    fn parse(params: &Option<Vec<Value>>) -> Option<GetBlockTransactionCountRequest> {
        let params = params.as_ref()?;
        if params.len() != 1 {
            return None;
        };
        Some(GetBlockTransactionCountRequest {
            block: serde_json::from_value(params[0].clone()).ok()?,
        })
    }

    fn handle(&self, storage: Store) -> Result<Value, RpcErr> {
        info!(
            "Requested transaction count for block with number: {}",
            self.block
        );
        let block_number = match self.block.resolve_block_number(&storage)? {
            Some(block_number) => block_number,
            _ => return Ok(Value::Null),
        };
        let block_body = match storage.get_block_body(block_number)? {
            Some(block_body) => block_body,
            _ => return Ok(Value::Null),
        };
        let transaction_count = block_body.transactions.len();

        serde_json::to_value(format!("{:#x}", transaction_count)).map_err(|_| RpcErr::Internal)
    }
}

impl RpcHandler for GetBlockReceiptsRequest {
    fn parse(params: &Option<Vec<Value>>) -> Option<GetBlockReceiptsRequest> {
        let params = params.as_ref()?;
        if params.len() != 1 {
            return None;
        };
        Some(GetBlockReceiptsRequest {
            block: serde_json::from_value(params[0].clone()).ok()?,
        })
    }

    fn handle(&self, storage: Store) -> Result<Value, RpcErr> {
        info!("Requested receipts for block with number: {}", self.block);
        let block_number = match self.block.resolve_block_number(&storage)? {
            Some(block_number) => block_number,
            _ => return Ok(Value::Null),
        };
        let header = storage.get_block_header(block_number)?;
        let body = storage.get_block_body(block_number)?;
        let (header, body) = match (header, body) {
            (Some(header), Some(body)) => (header, body),
            // Block not found
            _ => return Ok(Value::Null),
        };
        let receipts = get_all_block_receipts(block_number, header, body, &storage)?;

        serde_json::to_value(&receipts).map_err(|_| RpcErr::Internal)
    }
}

pub fn get_all_block_receipts(
    block_number: BlockNumber,
    header: BlockHeader,
    body: BlockBody,
    storage: &Store,
) -> Result<Vec<RpcReceipt>, RpcErr> {
    let mut receipts = Vec::new();
    // Check if this is the genesis block
    if header.parent_hash.is_zero() {
        return Ok(receipts);
    }
    let parent_header = match find_parent_header(&header, storage) {
        Ok(header) => header,
        _ => return Err(RpcErr::Internal),
    };
    let blob_gas_price = calculate_base_fee_per_blob_gas(&parent_header);
    // Fetch receipt info from block
    let block_info = RpcReceiptBlockInfo::from_block_header(header);
    // Fetch receipt for each tx in the block and add block and tx info
    let mut last_cumulative_gas_used = 0;
    let mut current_log_index = 0;
    for (index, tx) in body.transactions.iter().enumerate() {
        let index = index as u64;
        let receipt = match storage.get_receipt(block_number, index)? {
            Some(receipt) => receipt,
            _ => return Err(RpcErr::Internal),
        };
        let gas_used = receipt.cumulative_gas_used - last_cumulative_gas_used;
        let tx_info =
            RpcReceiptTxInfo::from_transaction(tx.clone(), index, gas_used, blob_gas_price);
        let receipt = RpcReceipt::new(
            receipt.clone(),
            tx_info,
            block_info.clone(),
            current_log_index,
        );
        last_cumulative_gas_used += gas_used;
        current_log_index += receipt.logs.len() as u64;
        receipts.push(receipt);
    }
    Ok(receipts)
}

pub fn block_number(storage: Store) -> Result<Value, RpcErr> {
    info!("Requested latest block number");
    match storage.get_latest_block_number() {
        Ok(Some(block_number)) => {
            serde_json::to_value(format!("{:#x}", block_number)).map_err(|_| RpcErr::Internal)
        }
        _ => Err(RpcErr::Internal),
    }
}

pub fn get_blob_base_fee(storage: &Store) -> Result<Value, RpcErr> {
    info!("Requested blob gas price");
    match storage.get_latest_block_number() {
        Ok(Some(block_number)) => {
            let header = match storage.get_block_header(block_number)? {
                Some(header) => header,
                _ => return Err(RpcErr::Internal),
            };
            let parent_header = match find_parent_header(&header, storage) {
                Ok(header) => header,
                _ => return Err(RpcErr::Internal),
            };
            let blob_base_fee = calculate_base_fee_per_blob_gas(&parent_header);
            serde_json::to_value(format!("{:#x}", blob_base_fee)).map_err(|_| RpcErr::Internal)
        }
        _ => Err(RpcErr::Internal),
    }
}

impl Display for BlockIdentifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BlockIdentifier::Number(num) => num.fmt(f),
            BlockIdentifier::Tag(tag) => match tag {
                BlockTag::Earliest => "Earliest".fmt(f),
                BlockTag::Finalized => "Finalized".fmt(f),
                BlockTag::Safe => "Safe".fmt(f),
                BlockTag::Latest => "Latest".fmt(f),
                BlockTag::Pending => "Pending".fmt(f),
            },
        }
    }
}

impl Default for BlockIdentifier {
    fn default() -> BlockIdentifier {
        BlockIdentifier::Tag(BlockTag::default())
    }
}
