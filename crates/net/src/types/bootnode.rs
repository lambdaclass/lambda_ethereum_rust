use std::net::SocketAddr;

#[derive(Debug, Clone)]
pub struct BootNode {
    pub node_id: Vec<u8>,
    pub socket_address: SocketAddr,
}
