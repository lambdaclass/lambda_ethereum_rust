pub(crate) mod discv4;

use std::{
    net::SocketAddr,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use discv4::{Endpoint, Message, PingMessage};
use k256::{
    ecdsa::{RecoveryId, Signature, SigningKey, VerifyingKey},
    elliptic_curve::rand_core::OsRng,
};
use rlpx::ecies::RLPxConnection;
use tokio::{
    io::AsyncWriteExt,
    net::{TcpSocket, UdpSocket},
    try_join,
};
use tracing::{info, warn};
pub mod rlpx;
pub mod types;

const MAX_DISC_PACKET_SIZE: usize = 1280;

pub async fn start_network(udp_addr: SocketAddr, tcp_addr: SocketAddr) {
    info!("Starting discovery service at {udp_addr}");
    info!("Listening for requests at {tcp_addr}");
    let signer = SigningKey::random(&mut OsRng);

    let discovery_handle = tokio::spawn(discover_peers(udp_addr, signer.clone()));
    let server_handle = tokio::spawn(serve_requests(tcp_addr, signer));
    try_join!(discovery_handle, server_handle).unwrap();
}

async fn discover_peers(socket_addr: SocketAddr, signer: SigningKey) {
    let udp_socket = UdpSocket::bind(socket_addr).await.unwrap();
    // This is just a placeholder example. The address is a known bootnode.
    // let receiver_addr: SocketAddr = ("138.197.51.181:30303").parse().unwrap();
    let mut buf = vec![0; MAX_DISC_PACKET_SIZE];

    // ping(&udp_socket, &signer, socket_addr, receiver_addr).await;

    // let (read, from) = udp_socket.recv_from(&mut buf).await.unwrap();
    // info!("Received {read} bytes from {from}");
    // let msg = Message::decode_with_header(&buf[..read]).unwrap();
    // info!("Message: {:?}", msg);

    // BEGIN EXAMPLE
    // Try contacting a known peer
    // TODO: do this dynamically
    let str_udp_addr = "127.0.0.1:57978";

    let udp_addr: SocketAddr = str_udp_addr.parse().unwrap();

    let (read, endpoint) = loop {
        ping(&udp_socket, &signer, socket_addr, udp_addr).await;

        let (read, from) = udp_socket.recv_from(&mut buf).await.unwrap();
        info!("Received {read} bytes from {from}");
        let msg = Message::decode_with_header(&buf[..read]).unwrap();
        info!("Message: {:?}", msg);

        match msg {
            Message::Pong(pong) => {
                break (read, pong.to);
            }
            // TODO: geth seems to respond with Ping instead of Pong
            Message::Ping(ping) => {
                break (read, ping.from);
            }
            _ => {
                warn!("Unexpected message type");
            }
        };
    };

    let digest = keccak_hash::keccak_buffer(&mut &buf[..read]).unwrap();
    let sig_bytes = &buf[32..32 + 65];
    let signature = &Signature::from_bytes(sig_bytes[..64].into()).unwrap();
    let rid = RecoveryId::from_byte(sig_bytes[64]).unwrap();

    let peer_pk = VerifyingKey::recover_from_prehash(&digest.0, signature, rid).unwrap();

    let conn = RLPxConnection::random();
    let mut auth_message = vec![];
    conn.encode_auth_message(&signer.into(), &peer_pk.into(), &mut auth_message);

    let tcp_addr = "127.0.0.1:59903";
    let tcp_addr = endpoint
        .to_tcp_address()
        .unwrap_or(tcp_addr.parse().unwrap());

    let mut stream = TcpSocket::new_v4()
        .unwrap()
        .connect(tcp_addr)
        .await
        .unwrap();

    stream.write_all(&auth_message).await.unwrap();
    info!("Sent auth message correctly!");
    // END EXAMPLE
}

async fn ping(
    socket: &UdpSocket,
    signer: &SigningKey,
    local_addr: SocketAddr,
    to_addr: SocketAddr,
) {
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

    let msg = discv4::Message::Ping(PingMessage::new(from, to, expiration));

    msg.encode_with_header(&mut buf, signer);
    socket.send_to(&buf, to_addr).await.unwrap();
}

async fn serve_requests(tcp_addr: SocketAddr, _signer: SigningKey) {
    let tcp_socket = TcpSocket::new_v4().unwrap();
    tcp_socket.bind(tcp_addr).unwrap();
}
