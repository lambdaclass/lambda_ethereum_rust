use ethereum_rust_storage::{error::StoreError, Store};
use serde_json::Value;
use std::{fmt::Display, str::FromStr};
use tracing::info;

use crate::{eth::block::BlockTag, utils::RpcErr, RpcHandler};
use ethereum_rust_core::{types::BlockNumber, Address, BigEndianHash, H256};

use super::block::BlockIdentifier;
use ethereum_rust_core::types::BlockHash;

#[derive(Clone, Debug)]
pub enum BlockIdentifierOrHash {
    Hash(BlockHash),
    Identifier(BlockIdentifier),
}

impl PartialEq<BlockTag> for BlockIdentifierOrHash {
    fn eq(&self, other: &BlockTag) -> bool {
        match self {
            BlockIdentifierOrHash::Identifier(BlockIdentifier::Tag(tag)) => tag == other,
            _ => false,
        }
    }
}

impl BlockIdentifierOrHash {
    #[allow(unused)]
    pub fn resolve_block_number(&self, storage: &Store) -> Result<Option<BlockNumber>, StoreError> {
        match self {
            BlockIdentifierOrHash::Identifier(id) => id.resolve_block_number(storage),
            BlockIdentifierOrHash::Hash(block_hash) => storage.get_block_number(*block_hash),
        }
    }

    pub fn is_latest(&self, storage: &Store) -> Result<bool, StoreError> {
        if self == &BlockTag::Latest {
            return Ok(true);
        }

        let result = self.resolve_block_number(storage)?;
        let latest = storage.get_latest_block_number()?;
        match (result, latest) {
            (Some(result), Some(latest)) => Ok(result == latest),
            _ => Ok(false),
        }
    }

    pub fn parse(serde_value: Value, arg_index: u64) -> Result<BlockIdentifierOrHash, RpcErr> {
        // Parse as BlockIdentifier
        if let Ok(block_identifier) = BlockIdentifier::parse(serde_value.clone(), arg_index) {
            return Ok(BlockIdentifierOrHash::Identifier(block_identifier));
        };
        // Parse as BlockHash
        let Ok(hex_str) = serde_json::from_value::<String>(serde_value) else {
            return Err(RpcErr::BadParams);
        };
        let Ok(block_hash) = H256::from_str(&hex_str) else {
            return Err(RpcErr::BadHexFormat(arg_index));
        };
        Ok(BlockIdentifierOrHash::Hash(block_hash))
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

pub struct GetTransactionCountRequest {
    pub address: Address,
    pub block: BlockIdentifierOrHash,
}

impl RpcHandler for GetBalanceRequest {
    fn parse(params: &Option<Vec<Value>>) -> Result<GetBalanceRequest, RpcErr> {
        let params = params.as_ref().ok_or(RpcErr::BadParams)?;
        if params.len() != 2 {
            return Err(RpcErr::BadParams);
        };
        Ok(GetBalanceRequest {
            address: serde_json::from_value(params[0].clone())?,
            block: BlockIdentifierOrHash::parse(params[1].clone(), 1)?,
        })
    }
    fn handle(&self, storage: Store) -> Result<Value, RpcErr> {
        info!(
            "Requested balance of account {} at block {}",
            self.address, self.block
        );

        // TODO: implement historical querying
        let is_latest = self.block.is_latest(&storage)?;
        if !is_latest {
            return Err(RpcErr::Internal);
        }

        let account = storage.get_account_info(self.address)?;
        let balance = account.map(|acc| acc.balance).unwrap_or_default();

        serde_json::to_value(format!("{:#x}", balance)).map_err(|_| RpcErr::Internal)
    }
}

impl RpcHandler for GetCodeRequest {
    fn parse(params: &Option<Vec<Value>>) -> Result<GetCodeRequest, RpcErr> {
        let params = params.as_ref().ok_or(RpcErr::BadParams)?;
        if params.len() != 2 {
            return Err(RpcErr::BadParams);
        };
        Ok(GetCodeRequest {
            address: serde_json::from_value(params[0].clone())?,
            block: BlockIdentifierOrHash::parse(params[1].clone(), 1)?,
        })
    }
    fn handle(&self, storage: Store) -> Result<Value, RpcErr> {
        info!(
            "Requested code of account {} at block {}",
            self.address, self.block
        );

        // TODO: implement historical querying
        let is_latest = self.block.is_latest(&storage)?;
        if !is_latest {
            return Err(RpcErr::Internal);
        }

        let code = storage
            .get_code_by_account_address(self.address)?
            .unwrap_or_default();

        serde_json::to_value(format!("0x{:x}", code)).map_err(|_| RpcErr::Internal)
    }
}

impl RpcHandler for GetStorageAtRequest {
    fn parse(params: &Option<Vec<Value>>) -> Result<GetStorageAtRequest, RpcErr> {
        let params = params.as_ref().ok_or(RpcErr::BadParams)?;
        if params.len() != 3 {
            return Err(RpcErr::BadParams);
        };
        Ok(GetStorageAtRequest {
            address: serde_json::from_value(params[0].clone())?,
            storage_slot: serde_json::from_value(params[1].clone())?,
            block: BlockIdentifierOrHash::parse(params[1].clone(), 1)?,
        })
    }
    fn handle(&self, storage: Store) -> Result<Value, RpcErr> {
        info!(
            "Requested storage sot {} of account {} at block {}",
            self.storage_slot, self.address, self.block
        );

        // TODO: implement historical querying
        let is_latest = self.block.is_latest(&storage)?;
        if !is_latest {
            return Err(RpcErr::Internal);
        }

        let storage_value = storage
            .get_storage_at(self.address, self.storage_slot)?
            .unwrap_or_default();
        let storage_value = H256::from_uint(&storage_value);
        serde_json::to_value(format!("{:#x}", storage_value)).map_err(|_| RpcErr::Internal)
    }
}

impl RpcHandler for GetTransactionCountRequest {
    fn parse(params: &Option<Vec<Value>>) -> Result<GetTransactionCountRequest, RpcErr> {
        let params = params.as_ref().ok_or(RpcErr::BadParams)?;
        if params.len() != 2 {
            return Err(RpcErr::BadParams);
        };
        Ok(GetTransactionCountRequest {
            address: serde_json::from_value(params[0].clone())?,
            block: BlockIdentifierOrHash::parse(params[1].clone(), 1)?,
        })
    }
    fn handle(&self, storage: Store) -> Result<Value, RpcErr> {
        info!(
            "Requested nonce of account {} at block {}",
            self.address, self.block
        );

        // TODO: implement historical querying
        let is_latest = self.block.is_latest(&storage)?;
        if !is_latest {
            return Err(RpcErr::Internal);
        }

        let nonce = storage
            .get_nonce_by_account_address(self.address)?
            .unwrap_or_default();

        serde_json::to_value(format!("0x{:x}", nonce)).map_err(|_| RpcErr::Internal)
    }
}

impl Display for BlockIdentifierOrHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BlockIdentifierOrHash::Identifier(id) => id.fmt(f),
            BlockIdentifierOrHash::Hash(hash) => hash.fmt(f),
        }
    }
}
