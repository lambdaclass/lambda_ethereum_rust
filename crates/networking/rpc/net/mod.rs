use ethereum_rust_storage::Store;
use serde_json::Value;

use crate::utils::{RpcErr, RpcRequest};

pub fn version(_req: &RpcRequest, store: Store) -> Result<Value, RpcErr> {
    let chain_spec = store.get_chain_config()?;
    serde_json::to_value(format!("{}", chain_spec.chain_id))
        .map_err(|error| RpcErr::Internal(error.to_string()))
}
