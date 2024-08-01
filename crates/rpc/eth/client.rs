use ethereum_rust_storage::Store;
use serde_json::Value;
use tracing::info;

use crate::utils::RpcErr;

pub fn chain_id(storage: Store) -> Result<Value, RpcErr> {
    info!("Requested chain id");
    match storage.get_chain_id() {
        Ok(Some(chain_id)) => {
            serde_json::to_value(format!("{:#x}", chain_id)).map_err(|_| RpcErr::Internal)
        }
        // Treat missing value as internal error as we should have a chain id
        // loaded in the db from loading the genesis file
        _ => Err(RpcErr::Internal),
    }
}

pub fn syncing() -> Result<Value, RpcErr> {
    Ok(Value::Bool(false))
}
