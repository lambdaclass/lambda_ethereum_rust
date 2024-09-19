use ethereum_rust_storage::Store;
use serde_json::Value;
use tracing::info;

use crate::types::block_identifier::BlockIdentifierOrHash;
use crate::{utils::RpcErr, RpcHandler};
use ethereum_rust_core::{Address, BigEndianHash, H256, U256};

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

pub struct GetProofRequest {
    pub address: Address,
    pub storage_keys: Vec<H256>,
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

        let Some(block_number) = self.block.resolve_block_number(&storage)? else {
            return Err(RpcErr::Internal); // Should we return Null here?
        };

        let account = storage.get_account_info(block_number, self.address)?;
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

        let Some(block_number) = self.block.resolve_block_number(&storage)? else {
            return Err(RpcErr::Internal); // Should we return Null here?
        };

        let code = storage
            .get_code_by_account_address(block_number, self.address)?
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
            block: BlockIdentifierOrHash::parse(params[2].clone(), 2)?,
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

        let Some(block_number) = self.block.resolve_block_number(&storage)? else {
            return Err(RpcErr::Internal); // Should we return Null here?
        };

        let nonce = storage
            .get_nonce_by_account_address(block_number, self.address)?
            .unwrap_or_default();

        serde_json::to_value(format!("0x{:x}", nonce)).map_err(|_| RpcErr::Internal)
    }
}

impl RpcHandler for GetProofRequest {
    fn parse(params: &Option<Vec<Value>>) -> Result<Self, RpcErr> {
        let params = params.as_ref().ok_or(RpcErr::BadParams)?;
        if params.len() != 3 {
            return Err(RpcErr::BadParams);
        };
        let storage_keys: Vec<U256> = serde_json::from_value(params[1].clone())?;
        let storage_keys = storage_keys
            .iter()
            .map(|key| H256::from_uint(key))
            .collect();
        Ok(GetProofRequest {
            address: serde_json::from_value(params[0].clone())?,
            storage_keys,
            block: BlockIdentifierOrHash::parse(params[2].clone(), 2)?,
        })
    }

    fn handle(&self, storage: Store) -> Result<Value, RpcErr> {
        todo!()
    }
}
