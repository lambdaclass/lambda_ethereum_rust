use ethereum_rust_storage::Store;
use serde_json::Value;
use tracing::info;

use crate::utils::RpcErr;
use ethereum_rust_core::Address;

use super::block::BlockIdentifier;

pub struct GetBalanceRequest {
    pub address: Address,
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

pub fn get_balance(request: &GetBalanceRequest, storage: Store) -> Result<Value, RpcErr> {
    info!(
        "Requested balance of account {} at block {}",
        request.address, request.block
    );
    let account = match storage.get_account_info(request.address) {
        Ok(Some(account)) => account,
        // Account not found
        Ok(_) => return Ok(Value::Null),
        // DB error
        _ => return Err(RpcErr::Internal),
    };

    serde_json::to_value(&format!("{:#x}", account.balance)).map_err(|_| RpcErr::Internal)
}
