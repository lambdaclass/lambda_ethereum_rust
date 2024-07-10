pub(crate) mod discv4;
pub(crate) mod kademlia;
use discv4::{Endpoint, FindNodeMessage, Message, Packet, PingMessage, PongMessage};
use ethereum_rust_core::H512;
use k256::elliptic_curve::sec1::ToEncodedPoint;
use k256::elliptic_curve::PublicKey;
use k256::{ecdsa::SigningKey, elliptic_curve::rand_core::OsRng};
use keccak_hash::H256;

use kademlia::{KademliaTable, PeerData};
use std::vec;
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
    let public_key = PublicKey::from(signer.verifying_key());
    let encoded = public_key.to_encoded_point(false);
    let local_node_id = H512::from_slice(&encoded.as_bytes()[1..]);

    let bootnode = match bootnodes.first() {
        Some(b) => b,
        None => {
            return;
        }
    };

    ping(&udp_socket, udp_addr, bootnode.socket_address, &signer).await;

    let mut buf = vec![0; MAX_DISC_PACKET_SIZE];
    let mut table = KademliaTable::new(local_node_id);
    loop {
        let (read, from) = udp_socket.recv_from(&mut buf).await.unwrap();
        let packet = Packet::decode(&buf[..read]).unwrap();
        let msg = packet.get_message();
        info!("Received {read} bytes from {from}");
        info!("Message: {:?}", msg);

        match msg {
            Message::Ping(_) => {
                let ping_hash = packet.get_hash();
                pong(&udp_socket, from, ping_hash, &signer).await;
                find_node(&udp_socket, from, &signer).await;
            }
            Message::Neighbors(neighbors_msg) => {
                let nodes = &neighbors_msg.nodes;
                for node in nodes {
                    let peer_data = PeerData {
                        ip: node.ip,
                        udp_port: node.udp_port,
                        tcp_port: node.tcp_port,
                        node_id: node.node_id,
                    };
                    table.insert(peer_data);
                    let node_addr = SocketAddr::new(node.ip, node.udp_port);
                    ping(&udp_socket, udp_addr, node_addr, &signer).await;
                }
            }
            _ => {}
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

#[cfg(test)]
mod tests {
    use super::*;
    use kademlia::bucket_number;
    use std::str::FromStr;
    #[test]
    fn bucket_number_works_as_expected() {
        let node_id_1 = H512::from_str("4dc429669029ceb17d6438a35c80c29e09ca2c25cc810d690f5ee690aa322274043a504b8d42740079c4f4cef50777c991010208b333b80bee7b9ae8e5f6b6f0").unwrap();
        let node_id_2 = H512::from_str("034ee575a025a661e19f8cda2b6fd8b2fd4fe062f6f2f75f0ec3447e23c1bb59beb1e91b2337b264c7386150b24b621b8224180c9e4aaf3e00584402dc4a8386").unwrap();
        let expected_bucket = 255;
        let result = bucket_number(node_id_1, node_id_2);
        assert_eq!(result, expected_bucket);
    }
}
