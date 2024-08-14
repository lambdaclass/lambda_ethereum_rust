use ethereum_rust_storage::{error::StoreError, Store};
use serde_json::Value;
use std::fmt::Display;
use tracing::info;

use crate::utils::RpcErr;
use ethereum_rust_core::{types::BlockNumber, Address, H256};

use super::block::BlockIdentifier;
use ethereum_rust_core::types::BlockHash;
use serde::Deserialize;

#[derive(Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum BlockIdentifierOrHash {
    Identifier(BlockIdentifier),
    Hash(BlockHash),
}

impl BlockIdentifierOrHash {
    #[allow(unused)]
    pub fn resolve_block_number(&self, storage: &Store) -> Result<Option<BlockNumber>, StoreError> {
        match self {
            BlockIdentifierOrHash::Identifier(id) => id.resolve_block_number(storage),
            BlockIdentifierOrHash::Hash(block_hash) => storage.get_block_number(*block_hash),
        }
    }
}

pub struct GetBalanceRequest {
    pub address: Address,
    pub block: BlockIdentifierOrHash,
}

pub struct GetCodeRequest {
    pub address: Address,
    pub block: BlockIdentifierOrHash,
}

pub struct GetStorageAtRequest {
    pub address: Address,
    pub storage_slot: H256,
    pub block: BlockIdentifierOrHash,
}

impl GetBalanceRequest {
    pub fn parse(params: &Option<Vec<Value>>) -> Option<GetBalanceRequest> {
        let params = params.as_ref()?;
        if params.len() != 2 {
            return None;
        };
        Some(GetBalanceRequest {
            address: serde_json::from_value(params[0].clone()).ok()?,
            block: serde_json::from_value(params[1].clone()).ok()?,
        })
    }
}

impl GetCodeRequest {
    pub fn parse(params: &Option<Vec<Value>>) -> Option<GetCodeRequest> {
        let params = params.as_ref()?;
        if params.len() != 2 {
            return None;
        };
        Some(GetCodeRequest {
            address: serde_json::from_value(params[0].clone()).ok()?,
            block: serde_json::from_value(params[1].clone()).ok()?,
        })
    }
}

impl GetStorageAtRequest {
    pub fn parse(params: &Option<Vec<Value>>) -> Option<GetStorageAtRequest> {
        let params = params.as_ref()?;
        if params.len() != 3 {
            return None;
        };
        Some(GetStorageAtRequest {
            address: serde_json::from_value(params[0].clone()).ok()?,
            storage_slot: serde_json::from_value(params[1].clone()).ok()?,
            block: serde_json::from_value(params[2].clone()).ok()?,
        })
    }
}

pub fn get_balance(request: &GetBalanceRequest, storage: Store) -> Result<Value, RpcErr> {
    info!(
        "Requested balance of account {} at block {}",
        request.address, request.block
    );
    let account = match storage.get_account_info(request.address)? {
        Some(account) => account,
        // Account not found
        _ => return Ok(Value::Null),
    };

    serde_json::to_value(format!("{:#x}", account.balance)).map_err(|_| RpcErr::Internal)
}

pub fn get_code(request: &GetCodeRequest, storage: Store) -> Result<Value, RpcErr> {
    info!(
        "Requested code of account {} at block {}",
        request.address, request.block
    );
    let code = match storage.get_code_by_account_address(request.address)? {
        Some(code) => code,
        // Account not found
        _ => return Ok(Value::Null),
    };

    serde_json::to_value(format!("0x{:x}", code)).map_err(|_| RpcErr::Internal)
}

pub fn get_storage_at(request: &GetStorageAtRequest, storage: Store) -> Result<Value, RpcErr> {
    info!(
        "Requested storage sot {} of account {} at block {}",
        request.storage_slot, request.address, request.block
    );
    let storage_value = match storage.get_storage_at(request.address, request.storage_slot)? {
        Some(storage_value) => storage_value,
        // Account not found
        _ => return Ok(Value::Null),
    };

    serde_json::to_value(format!("{:#x}", storage_value)).map_err(|_| RpcErr::Internal)
}

impl Display for BlockIdentifierOrHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BlockIdentifierOrHash::Identifier(id) => id.fmt(f),
            BlockIdentifierOrHash::Hash(hash) => hash.fmt(f),
        }
    }
}
