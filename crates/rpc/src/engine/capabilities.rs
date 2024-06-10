use serde_json::{json, Value};

use crate::RpcErr;

// type ExchangeCapabilities = Vec<String>;

pub fn exchange_capabilities() -> Result<Value, RpcErr> {
    Ok(json!([]))
}
