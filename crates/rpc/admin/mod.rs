use serde_json::{json, Value};

use crate::utils::RpcErr;

pub fn node_info() -> Result<Value, RpcErr> {
    Ok(json!({
        "enode": "enode://pubkey@ip:port",
        "id": "pubkey",
        "name": "node",
        "ports": {
            "discovery": 1234,
            "listener": 1234,
        },
        "protocols": {
            "eth": {
                "network": 1234,
                "version": 1234,
            },
        },
    }))
}
