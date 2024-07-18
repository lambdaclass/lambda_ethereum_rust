use std::fmt::Display;

use ethereum_rust_storage::Store;
use serde::Deserialize;
use serde_json::Value;
use tracing::info;

use crate::utils::RpcErr;
use ethereum_rust_core::types::{BlockHash, BlockSerializable};

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

#[derive(Deserialize)]
pub enum BlockIdentifier {
    Number(u64),
    #[allow(unused)]
    Tag(BlockTag),
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BlockTag {
    Earliest,
    Finalized,
    Safe,
    Latest,
    Pending,
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

pub fn get_block_by_number(
    request: &GetBlockByNumberRequest,
    storage: Store,
) -> Result<Value, RpcErr> {
    info!("Requested block with number: {}", request.block);
    let block_number = match request.block {
        BlockIdentifier::Tag(_) => unimplemented!("Obtain block number from tag"),
        BlockIdentifier::Number(block_number) => block_number,
    };
    let header = storage.get_block_header(block_number);
    let body = storage.get_block_body(block_number);
    let (header, body) = match (header, body) {
        (Ok(Some(header)), Ok(Some(body))) => (header, body),
        // Block not found
        (Ok(_), Ok(_)) => return Ok(Value::Null),
        // DB error
        _ => return Err(RpcErr::Internal),
    };
    let block = BlockSerializable::from_block(header, body, request.hydrated);

    serde_json::to_value(&block).map_err(|_| RpcErr::Internal)
}

pub fn get_block_by_hash(request: &GetBlockByHashRequest, storage: Store) -> Result<Value, RpcErr> {
    info!("Requested block with hash: {}", request.block);
    let block_number = match storage.get_block_number(request.block) {
        Ok(Some(number)) => number,
        Ok(_) => return Ok(Value::Null),
        _ => return Err(RpcErr::Internal),
    };
    let header = storage.get_block_header(block_number);
    let body = storage.get_block_body(block_number);
    let (header, body) = match (header, body) {
        (Ok(Some(header)), Ok(Some(body))) => (header, body),
        // Block not found
        (Ok(_), Ok(_)) => return Ok(Value::Null),
        // DB error
        _ => return Err(RpcErr::Internal),
    };
    let block = BlockSerializable::from_block(header, body, request.hydrated);

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
    let block_number = match request.block {
        BlockIdentifier::Tag(_) => unimplemented!("Obtain block number from tag"),
        BlockIdentifier::Number(block_number) => block_number,
    };
    let block_body = match storage.get_block_body(block_number) {
        Ok(Some(block_body)) => block_body,
        Ok(_) => return Ok(Value::Null),
        _ => return Err(RpcErr::Internal),
    };
    let transaction_count = block_body.transactions.len();

    serde_json::to_value(format!("{:#x}", transaction_count)).map_err(|_| RpcErr::Internal)
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
