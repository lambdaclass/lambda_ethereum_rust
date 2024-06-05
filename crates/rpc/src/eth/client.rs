use serde_json::Value;

use crate::utils::RpcErr;

pub fn chain_id() -> Result<Value, RpcErr> {
    Ok(Value::String("0xaa36a7".to_string()))
}

pub fn syncing() -> Result<Value, RpcErr> {
    Ok(Value::Bool(false))
}
