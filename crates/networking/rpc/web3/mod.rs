use ethereum_rust_storage::Store;
use serde_json::Value;

use crate::utils::{RpcErr, RpcRequest};

pub fn client_version(_req: &RpcRequest, _store: Store) -> Result<Value, RpcErr> {
    Ok(Value::String("ethereum_rust@0.1.0".to_owned()))
}
