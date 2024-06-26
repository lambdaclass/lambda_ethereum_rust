use std::net::SocketAddr;

use tokio::net::{TcpSocket, UdpSocket};
use tracing::info;

const MAX_DISC_PACKET_SIZE: usize = 1280;

pub async fn start_network(udp_addr: SocketAddr, tcp_addr: SocketAddr) {
    info!("Starting discovery service at {udp_addr}");
    info!("Listening for requests at {tcp_addr}");

    let udp_socket = UdpSocket::bind(udp_addr).await.unwrap();

    udp_socket.send_to(&[], "51.141.78.53:30303").await.unwrap();

    let mut buf = [0; MAX_DISC_PACKET_SIZE];
    let (read, from) = udp_socket.recv_from(&mut buf).await.unwrap();
    info!("Received {} bytes from {}", read, from);

    let tcp_socket = TcpSocket::new_v4().unwrap();
    tcp_socket.bind(tcp_addr).unwrap();
}
