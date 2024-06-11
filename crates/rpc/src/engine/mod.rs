use serde_json::{json, Value};
use tracing::info;

use crate::RpcErr;

type ExchangeCapabilitiesRequest = Vec<String>;

pub fn exchange_capabilities(params: Option<Value>) -> Result<Value, RpcErr> {
    if let Some(params) = params {
        let params: ExchangeCapabilitiesRequest = serde_json::from_value(params).unwrap();
        Ok(json!(params))
    } else {
        Ok(json!([]))
    }
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

pub fn new_payload_v3(params: Option<Value>) -> Result<Value, RpcErr> {
    let block = params.unwrap();

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
