pub(crate) mod discv4;

use std::{
    net::SocketAddr,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use discv4::{Endpoint, Message, PingMessage, PongMessage};
use k256::{ecdsa::SigningKey, elliptic_curve::rand_core::OsRng};
use keccak_hash::H256;
use tokio::{
    net::{TcpSocket, UdpSocket},
    try_join,
};
use tracing::info;
use types::BootNode;
pub mod types;

const MAX_DISC_PACKET_SIZE: usize = 1280;

pub async fn start_network(udp_addr: SocketAddr, tcp_addr: SocketAddr, bootnodes: Vec<BootNode>) {
    info!("Starting discovery service at {udp_addr}");
    info!("Listening for requests at {tcp_addr}");

    let discovery_handle = tokio::spawn(discover_peers(udp_addr, bootnodes));
    let server_handle = tokio::spawn(serve_requests(tcp_addr));
    try_join!(discovery_handle, server_handle).unwrap();
}

async fn discover_peers(udp_addr: SocketAddr, bootnodes: Vec<BootNode>) {
    let udp_socket = UdpSocket::bind(udp_addr).await.unwrap();

    for bootnode in &bootnodes {
        ping(&udp_socket, udp_addr, bootnode.socket_address).await;
    }

    let mut buf = vec![0; MAX_DISC_PACKET_SIZE];
    // for each `Ping` we send we are receiving a `Pong` and a `Ping`
    for _ in 0..bootnodes.len() * 2 {
        let (read, from) = udp_socket.recv_from(&mut buf).await.unwrap();
        let msg = Message::decode_with_header(&buf[..read]).unwrap();
        info!("Received {read} bytes from {from}");
        info!("Message: {:?}", msg);
        match msg {
            Message::Ping(_) => {
                let ping_hash = H256::from_slice(Message::get_hash(&buf[..read]));
                pong(&udp_socket, from, ping_hash).await;
                info!("Sending Pong");
            }
            _ => {
                dbg!(msg);
            }
        }
    }
}

async fn ping(socket: &UdpSocket, local_addr: SocketAddr, to_addr: SocketAddr) {
    let mut buf = Vec::new();

    let expiration: u64 = (SystemTime::now() + Duration::from_secs(10))
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
        .try_into()
        .unwrap();

    // TODO: this should send our advertised TCP port
    let from = Endpoint {
        ip: local_addr.ip(),
        udp_port: local_addr.port(),
        tcp_port: 0,
    };
    let to = Endpoint {
        ip: to_addr.ip(),
        udp_port: to_addr.port(),
        tcp_port: 0,
    };

    let msg: discv4::Message = discv4::Message::Ping(PingMessage::new(from, to, expiration));
    let signer = SigningKey::random(&mut OsRng);

    msg.encode_with_header(&mut buf, signer);
    socket.send_to(&buf, to_addr).await.unwrap();
}

async fn pong(socket: &UdpSocket, to_addr: SocketAddr, ping_hash: H256) {
    let mut buf = Vec::new();

    let expiration: u64 = (SystemTime::now() + Duration::from_secs(10))
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
        .try_into()
        .unwrap();

    let to = Endpoint {
        ip: to_addr.ip(),
        udp_port: to_addr.port(),
        tcp_port: 0,
    };

    let msg: discv4::Message = discv4::Message::Pong(PongMessage::new(to, ping_hash, expiration));
    let signer = SigningKey::random(&mut OsRng);

    msg.encode_with_header(&mut buf, signer);
    socket.send_to(&buf, to_addr).await.unwrap();
}

async fn serve_requests(tcp_addr: SocketAddr) {
    let tcp_socket = TcpSocket::new_v4().unwrap();
    tcp_socket.bind(tcp_addr).unwrap();
}
