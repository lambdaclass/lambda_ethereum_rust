use serde::Deserialize;
use serde_json::Value;

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

pub fn get_block_by_number(
    request: &GetBlockByNumberRequest,
    storage: Store,
) -> Result<Value, RpcErr> {
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
