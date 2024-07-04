use ethereum_rust_core::{
    types::{ExecutionPayloadV3, PayloadStatus, PayloadValidationStatus},
    H256,
};
use serde_json::{json, Value};
use tracing::info;

use crate::RpcErr;

pub type ExchangeCapabilitiesRequest = Vec<String>;

pub struct NewPayloadV3Request {
    pub payload: ExecutionPayloadV3,
    pub expected_blob_versioned_hashes: Vec<H256>,
    pub parent_beacon_block_root: H256,
}

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

pub fn new_payload_v3(request: NewPayloadV3Request) -> Result<PayloadStatus, RpcErr> {
    let block_hash = request.payload.block_hash;

    info!("Received new payload with block hash: {}", block_hash);

    let (block_header, block_body) =
        match request.payload.into_block(request.parent_beacon_block_root) {
            Ok(block) => block,
            Err(error) => {
                return Ok(PayloadStatus {
                    status: PayloadValidationStatus::Invalid,
                    latest_valid_hash: H256::zero(),
                    validation_error: Some(error.to_string()),
                })
            }
        };

    Ok(PayloadStatus {
        status: PayloadValidationStatus::Valid,
        latest_valid_hash: block_hash,
        validation_error: None,
    })
}
