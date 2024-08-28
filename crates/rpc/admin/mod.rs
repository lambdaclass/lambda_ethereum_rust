use ethereum_rust_core::types::ChainConfig;
use ethereum_rust_net::types::Node;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;

use crate::utils::RpcErr;

#[derive(Serialize, Debug)]
struct NodeInfo {
    enode: String,
    id: String,
    ip: String,
    name: String,
    ports: Ports,
    protocols: HashMap<String, Protocol>,
}

#[derive(Serialize, Debug)]
struct Ports {
    discovery: u16,
    listener: u16,
}

#[derive(Serialize, Debug)]
#[serde(untagged)]
enum Protocol {
    Eth(ChainConfig),
}

pub fn enode_url(
    node_id: &String,
    node_ip: &String,
    discovery_port: &u16,
    listener_port: &u16,
) -> String {
    format!("enode://{node_id}@{node_ip}:{listener_port}?discport={discovery_port}")
}

//TODO: pass network config to function
pub fn node_info(chain_config: ChainConfig, local_node: Node) -> Result<Value, RpcErr> {
    let node_ip = local_node.ip.to_string();
    let node_id = hex::encode(local_node.node_id);
    let discovery_port = local_node.udp_port;
    let listener_port = local_node.tcp_port;
    let enode_url = enode_url(&node_id, &node_ip, &listener_port, &discovery_port);
    let mut protocols = HashMap::new();
    protocols.insert("eth".to_string(), Protocol::Eth(chain_config));

    let node_info = NodeInfo {
        enode: enode_url,
        id: node_id,
        name: "ethereum_rust/0.1.0/rust1.80".to_string(),
        ip: node_ip,
        ports: Ports {
            discovery: discovery_port,
            listener: listener_port,
        },
        protocols,
    };
    serde_json::to_value(node_info).map_err(|_| RpcErr::Internal)
}
