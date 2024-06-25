use serde_json::{json, Value};
use tracing::info;

use crate::RpcErr;

pub type ExchangeCapabilitiesRequest = Vec<String>;

pub fn exchange_capabilities(capabilities: &ExchangeCapabilitiesRequest) -> Result<Value, RpcErr> {
    Ok(json!(capabilities))
}

pub fn forkchoice_updated_v3() -> Result<Value, RpcErr> {
    Ok(json!({
        "payloadId": null,
        "payloadStatus": {
            "latestValidHash": null,
            "status": "SYNCING",
            "validationError": null
        }
    }))
}

pub fn new_payload_v3(block: &Value) -> Result<Value, RpcErr> {
    info!(
        "Received new payload with block hash: {}",
        block["blockHash"]
    );

    Ok(json!({
        "latestValidHash": null,
        "status": "SYNCING",
        "validationError": null
    }))
}
