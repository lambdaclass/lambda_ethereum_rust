use ethereum_rust_storage::Store;
use serde_json::Value;
use tracing::info;

use crate::utils::RpcErr;
use ethereum_rust_core::{Address, H256};

use super::block::BlockIdentifier;

pub struct GetBalanceRequest {
    pub address: Address,
    pub block: BlockIdentifier,
}

pub struct GetCodeRequest {
    pub address: Address,
    pub block: BlockIdentifier,
}

pub struct GetStorageAtRequest {
    pub address: Address,
    pub storage_slot: H256,
    pub block: BlockIdentifier,
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
    let account = storage.get_account_info(request.address)?;
    let balance = account.map(|acc| acc.balance).unwrap_or_default();

    serde_json::to_value(format!("{:#x}", balance)).map_err(|_| RpcErr::Internal)
}

pub fn get_code(request: &GetCodeRequest, storage: Store) -> Result<Value, RpcErr> {
    info!(
        "Requested code of account {} at block {}",
        request.address, request.block
    );
    let code = storage
        .get_code_by_account_address(request.address)?
        .unwrap_or_default();

    serde_json::to_value(format!("0x{:x}", code)).map_err(|_| RpcErr::Internal)
}

pub fn get_storage_at(request: &GetStorageAtRequest, storage: Store) -> Result<Value, RpcErr> {
    info!(
        "Requested storage sot {} of account {} at block {}",
        request.storage_slot, request.address, request.block
    );
    let storage_value = storage
        .get_storage_at(request.address, request.storage_slot)?
        .unwrap_or_default();

    serde_json::to_value(format!("{:#x}", storage_value)).map_err(|_| RpcErr::Internal)
}
