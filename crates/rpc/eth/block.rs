use std::fmt::Display;

use ethereum_rust_chain::find_parent_header;
use serde::Deserialize;
use serde_json::Value;
use tracing::info;

use crate::{
    types::{
        block::RpcBlock,
        receipt::{RpcReceipt, RpcReceiptBlockInfo, RpcReceiptTxInfo},
    },
    utils::RpcErr,
};
use ethereum_rust_core::{
    types::{calculate_base_fee_per_blob_gas, BlockBody, BlockHash, BlockHeader, BlockNumber},
    U256,
};
use ethereum_rust_storage::{error::StoreError, Store};

use super::account::BlockIdentifierOrHash;

pub struct GetBlockByNumberRequest {
    pub block: BlockIdentifier,
    pub hydrated: bool,
}

pub struct GetBlockByHashRequest {
    pub block: BlockHash,
    pub hydrated: bool,
}

pub struct GetBlockTransactionCountByNumberRequest {
    pub block: BlockIdentifier,
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

#[derive(Deserialize, Default, Clone, Debug)]
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

impl GetBlockByNumberRequest {
    pub fn parse(params: &Option<Vec<Value>>) -> Option<GetBlockByNumberRequest> {
        let params = params.as_ref()?;
        if params.len() != 2 {
            return None;
        };
        Some(GetBlockByNumberRequest {
            block: serde_json::from_value(params[0].clone()).ok()?,
            hydrated: serde_json::from_value(params[1].clone()).ok()?,
        })
    }
}

impl GetBlockByHashRequest {
    pub fn parse(params: &Option<Vec<Value>>) -> Option<GetBlockByHashRequest> {
        let params = params.as_ref()?;
        if params.len() != 2 {
            return None;
        };
        Some(GetBlockByHashRequest {
            block: serde_json::from_value(params[0].clone()).ok()?,
            hydrated: serde_json::from_value(params[1].clone()).ok()?,
        })
    }
}

impl GetBlockTransactionCountByNumberRequest {
    pub fn parse(params: &Option<Vec<Value>>) -> Option<GetBlockTransactionCountByNumberRequest> {
        let params = params.as_ref()?;
        if params.len() != 1 {
            return None;
        };
        Some(GetBlockTransactionCountByNumberRequest {
            block: serde_json::from_value(params[0].clone()).ok()?,
        })
    }
}

impl GetBlockReceiptsRequest {
    pub fn parse(params: &Option<Vec<Value>>) -> Option<GetBlockReceiptsRequest> {
        let params = params.as_ref()?;
        if params.len() != 1 {
            return None;
        };
        Some(GetBlockReceiptsRequest {
            block: serde_json::from_value(params[0].clone()).ok()?,
        })
    }
}

pub fn get_block_by_number(
    request: &GetBlockByNumberRequest,
    storage: Store,
) -> Result<Value, RpcErr> {
    info!("Requested block with number: {}", request.block);
    let block_number = match request.block.resolve_block_number(&storage)? {
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
    // TODO (#307): Remove TotalDifficulty.
    let total_difficulty = storage.get_block_total_difficulty(hash)?;
    let block = RpcBlock::build(
        header,
        body,
        hash,
        request.hydrated,
        total_difficulty.unwrap_or(U256::zero()),
    );

    serde_json::to_value(&block).map_err(|_| RpcErr::Internal)
}

pub fn get_block_by_hash(request: &GetBlockByHashRequest, storage: Store) -> Result<Value, RpcErr> {
    info!("Requested block with hash: {}", request.block);
    let block_number = match storage.get_block_number(request.block)? {
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
    // TODO (#307): Remove TotalDifficulty.
    let total_difficulty = storage.get_block_total_difficulty(hash)?;
    let block = RpcBlock::build(
        header,
        body,
        hash,
        request.hydrated,
        total_difficulty.unwrap_or(U256::zero()),
    );
    serde_json::to_value(&block).map_err(|_| RpcErr::Internal)
}

pub fn get_block_transaction_count_by_number(
    request: &GetBlockTransactionCountByNumberRequest,
    storage: Store,
) -> Result<Value, RpcErr> {
    info!(
        "Requested transaction count for block with number: {}",
        request.block
    );
    let block_number = match request.block.resolve_block_number(&storage)? {
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

pub fn get_block_receipts(
    request: &GetBlockReceiptsRequest,
    storage: Store,
) -> Result<Value, RpcErr> {
    info!(
        "Requested receipts for block with number: {}",
        request.block
    );
    let block_number = match request.block.resolve_block_number(&storage)? {
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
    let parent_header = match find_parent_header(&block.header, storage) {
        Ok(header) => header,
        _ => return Err(RpcErr::Internal),
    };
    let blob_gas_price = calculate_base_fee_per_blob_gas(parent_header);
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
