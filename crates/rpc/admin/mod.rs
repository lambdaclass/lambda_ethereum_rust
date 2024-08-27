use ethereum_rust_core::types::ChainConfig;
use serde_json::{json, Value};

use crate::utils::RpcErr;

pub struct NodeInfo {
    enode: String,
    id: String,
    name: String,
    discovery_port: i32,
    listener_port: i32,
}

//TODO: pass network config to function
pub fn node_info(chain_config: &ChainConfig) -> Result<Value, RpcErr> {
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
