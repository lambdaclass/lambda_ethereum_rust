pub use ethereum_types::*;
use std::net::SocketAddr;
use tokio::net::{TcpSocket, UdpSocket};
use tracing::info;

pub mod types;

pub async fn start_network(udp_addr: SocketAddr, tcp_addr: SocketAddr) {
    info!("Starting discovery service at {udp_addr}");
    info!("Listening for requests at {tcp_addr}");

    let _udp_socket = UdpSocket::bind(udp_addr).await.unwrap();

    let tcp_socket = TcpSocket::new_v4().unwrap();
    tcp_socket.bind(tcp_addr).unwrap();
}
