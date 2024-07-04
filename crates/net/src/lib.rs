pub(crate) mod discv4;

use discv4::{Endpoint, FindNodeMessage, Message, PingMessage, PongMessage};
use ethereum_rust_core::H512;
use k256::Secp256k1;
use k256::{ecdsa::SigningKey, elliptic_curve::rand_core::OsRng};
use keccak_hash::H256;
use std::str::FromStr;
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
    let bootnode = bootnodes[0];

    ping(&udp_socket, udp_addr, bootnode.socket_address, &signer).await;

    let mut buf = vec![0; MAX_DISC_PACKET_SIZE];
    // for each `Ping` we send we are receiving a `Pong` and a `Ping`
    for _ in 0..2 {
        let (read, from) = udp_socket.recv_from(&mut buf).await.unwrap();
        let msg = Message::decode_with_header(&buf[..read]).unwrap();
        info!("Received {read} bytes from {from}");
        info!("Message: {:?}", msg);
        match msg {
            Message::Ping(_) => {
                let ping_hash = H256::from_slice(Message::get_hash(&buf[..read]));
                pong(&udp_socket, bootnode.socket_address, ping_hash, &signer).await;
                info!("Sending Pong");
            }
            _ => {
                dbg!(msg);
            }
        }
    }

    find_node(&udp_socket, bootnode.socket_address, &signer).await;
    info!("Sending FindNode");
    let (read, from) = udp_socket.recv_from(&mut buf).await.unwrap();
    let msg = Message::decode_with_header(&buf[..read]).unwrap();
    info!("Received {read} bytes from {from}");
    info!("Message: {:?}", msg);
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
        .as_secs()
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

    let ping: discv4::Message = discv4::Message::Ping(PingMessage::new(from, to, expiration));

    ping.encode_with_header(&mut buf, signer.clone());
    socket.send_to(&buf, to_addr).await.unwrap();
}

async fn find_node(socket: &UdpSocket, to_addr: SocketAddr, signer: &SigningKey) {
    let target = H512::from_str("764dd14179894fde2996d44fc9e91e3dc271a1733a1d79a22658e9154c9a949a2df86142be669295d36bb16ca7f1ef4c5b0e9dc30a08d0e1f9598d7e080ec3af").unwrap();

    let expiration: u64 = (SystemTime::now() + Duration::from_secs(10))
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .try_into()
        .unwrap();

    let msg: discv4::Message = discv4::Message::FindNode(FindNodeMessage::new(target, expiration));

    let mut buf = Vec::new();
    msg.encode_with_header(&mut buf, signer.clone());
    socket.send_to(&buf, to_addr).await.unwrap();
}

async fn pong(socket: &UdpSocket, to_addr: SocketAddr, ping_hash: H256, signer: &SigningKey) {
    let mut buf = Vec::new();

    let expiration: u64 = (SystemTime::now() + Duration::from_secs(10))
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .try_into()
        .unwrap();
    let to = Endpoint {
        ip: to_addr.ip(),
        udp_port: to_addr.port(),
        tcp_port: 0,
    };
    let pong: discv4::Message = discv4::Message::Pong(
        PongMessage::new(to, ping_hash, expiration).with_enr_seq(0x01907e144b64),
    );

    pong.encode_with_header(&mut buf, signer.clone());
    socket.send_to(&buf, to_addr).await.unwrap();
}

async fn serve_requests(tcp_addr: SocketAddr) {
    let tcp_socket = TcpSocket::new_v4().unwrap();
    tcp_socket.bind(tcp_addr).unwrap();
}
