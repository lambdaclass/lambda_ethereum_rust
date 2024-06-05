use serde_json::Value;

use crate::RpcErr;

// type ExchangeCapabilities = Vec<String>;

pub fn exchange_capabilities() -> Result<Value, RpcErr> {
    Ok(Value::Array(vec![]))
}
