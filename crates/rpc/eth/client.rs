use ethereum_rust_storage::Store;
use serde_json::Value;
use tracing::info;

use crate::utils::RpcErr;

pub fn chain_id(storage: Store) -> Result<Value, RpcErr> {
    info!("Requested chain id");
    let chain_spec = storage.get_chain_config().map_err(|_| RpcErr::Internal)?;
    serde_json::to_value(format!("{:#x}", chain_spec.chain_id)).map_err(|_| RpcErr::Internal)
}

pub fn syncing() -> Result<Value, RpcErr> {
    Ok(Value::Bool(false))
}
