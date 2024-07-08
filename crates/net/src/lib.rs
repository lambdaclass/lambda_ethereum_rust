pub(crate) mod discv4;
use discv4::{Endpoint, FindNodeMessage, Message, PingMessage, PongMessage};
use ethereum_rust_core::H512;
use k256::elliptic_curve::sec1::ToEncodedPoint;
use k256::elliptic_curve::PublicKey;
use k256::{ecdsa::SigningKey, elliptic_curve::rand_core::OsRng};
use keccak_hash::H256;
use std::{
    net::SocketAddr,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
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
    let signer = SigningKey::random(&mut OsRng);
    let bootnode = match bootnodes.first() {
        Some(b) => b,
        None => {
            return;
        }
    };

    ping(&udp_socket, udp_addr, bootnode.socket_address, &signer).await;

    let mut buf = vec![0; MAX_DISC_PACKET_SIZE];
    loop {
        let (read, from) = udp_socket.recv_from(&mut buf).await.unwrap();
        let packet_type = buf[32 + 65];
        match packet_type {
            0x04 => {
                info!("Received NEIGHBORS message from {from}");
            }
            _ => {
                let msg = Message::decode_with_header(&buf[..read]).unwrap();
                info!("Received {read} bytes from {from}");
                info!("Message: {:?}", msg);
                if let Message::Ping(_) = msg {
                    let ping_hash = H256::from_slice(Message::get_hash(&buf[..read]));
                    pong(&udp_socket, from, ping_hash, &signer).await;
                    find_node(&udp_socket, from, &signer).await;
                }
            }
        }
    }
}

async fn ping(
    socket: &UdpSocket,
    local_addr: SocketAddr,
    to_addr: SocketAddr,
    signer: &SigningKey,
) {
    let mut buf = Vec::new();

    let expiration: u64 = (SystemTime::now() + Duration::from_secs(10))
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

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

    let ping: discv4::Message = discv4::Message::Ping(PingMessage::new(from, to, expiration));

    ping.encode_with_header(&mut buf, signer);
    socket.send_to(&buf, to_addr).await.unwrap();
}

async fn find_node(socket: &UdpSocket, to_addr: SocketAddr, signer: &SigningKey) {
    let public_key = PublicKey::from(signer.verifying_key());
    let encoded = public_key.to_encoded_point(false);
    let bytes = encoded.as_bytes();
    debug_assert_eq!(bytes[0], 4);

    let target = H512::from_slice(&bytes[1..]);

    let expiration: u64 = (SystemTime::now() + Duration::from_secs(10))
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let msg: discv4::Message = discv4::Message::FindNode(FindNodeMessage::new(target, expiration));

    let mut buf = Vec::new();
    msg.encode_with_header(&mut buf, signer);

    socket.send_to(&buf, to_addr).await.unwrap();
}

async fn pong(socket: &UdpSocket, to_addr: SocketAddr, ping_hash: H256, signer: &SigningKey) {
    let mut buf = Vec::new();

    let expiration: u64 = (SystemTime::now() + Duration::from_secs(10))
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let to = Endpoint {
        ip: to_addr.ip(),
        udp_port: to_addr.port(),
        tcp_port: 0,
    };
    let pong: discv4::Message = discv4::Message::Pong(PongMessage::new(to, ping_hash, expiration));

    pong.encode_with_header(&mut buf, signer);
    socket.send_to(&buf, to_addr).await.unwrap();
}

async fn serve_requests(tcp_addr: SocketAddr) {
    let tcp_socket = TcpSocket::new_v4().unwrap();
    tcp_socket.bind(tcp_addr).unwrap();
}
