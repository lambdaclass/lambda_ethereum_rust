use std::net::SocketAddr;

#[derive(Debug)]
pub struct BootNode {
    pub node_id: [u8; 128],
    pub socket_address: SocketAddr,
}
