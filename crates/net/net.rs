use std::{
    collections::HashSet,
    net::SocketAddr,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use bootnode::BootNode;
use discv4::{
    get_expiration, is_expired, time_since_in_hs, FindNodeMessage, Message, NeighborsMessage,
    Packet, PingMessage, PongMessage,
};
use ethereum_rust_core::{H256, H512};
use k256::{
    ecdsa::{RecoveryId, Signature, SigningKey, VerifyingKey},
    elliptic_curve::{sec1::ToEncodedPoint, PublicKey},
    SecretKey,
};
use kademlia::{KademliaTable, MAX_NODES_PER_BUCKET};
use rand::rngs::OsRng;
use rlpx::{
    connection::SUPPORTED_CAPABILITIES, handshake::RLPxLocalClient,
    message::Message as RLPxMessage, p2p,
};
use sha3::{Digest, Keccak256};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpSocket, UdpSocket},
    sync::Mutex,
    try_join,
};
use tracing::{debug, info, warn};
use types::{Endpoint, Node};

pub mod bootnode;
pub(crate) mod discv4;
pub(crate) mod kademlia;
pub mod rlpx;
pub mod types;

const MAX_DISC_PACKET_SIZE: usize = 1280;

pub async fn start_network(
    udp_addr: SocketAddr,
    tcp_addr: SocketAddr,
    bootnodes: Vec<BootNode>,
    signer: SigningKey,
) {
    info!("Starting discovery service at {udp_addr}");
    info!("Listening for requests at {tcp_addr}");

    let discovery_handle = tokio::spawn(discover_peers(udp_addr, signer.clone(), bootnodes));
    let server_handle = tokio::spawn(serve_requests(tcp_addr, signer));

    try_join!(discovery_handle, server_handle).unwrap();
}

async fn discover_peers(udp_addr: SocketAddr, signer: SigningKey, bootnodes: Vec<BootNode>) {
    let udp_socket = Arc::new(UdpSocket::bind(udp_addr).await.unwrap());
    let local_node_id = node_id_from_signing_key(&signer);
    let table = Arc::new(Mutex::new(KademliaTable::new(local_node_id)));

    // TODO implement this right
    if let Some(b) = bootnodes.first() {
        ping(&udp_socket, udp_addr, b.socket_address, &signer).await;
    };

    let server_handler = tokio::spawn(discover_peers_server(
        udp_addr,
        udp_socket.clone(),
        table.clone(),
        signer.clone(),
    ));
    let revalidation_handler = tokio::spawn(peers_revalidation(
        udp_addr,
        udp_socket.clone(),
        table.clone(),
        signer.clone(),
        REVALIDATION_INTERVAL_IN_MINUTES as u64 * 60,
    ));
    let lookup_handler = tokio::spawn(peers_lookup(
        udp_socket.clone(),
        table.clone(),
        signer,
        local_node_id,
        PEERS_RANDOM_LOOKUP_TIME_IN_MIN as u64 * 60,
    ));

    try_join!(server_handler, revalidation_handler, lookup_handler).unwrap();
}
async fn discover_peers_server(
    udp_addr: SocketAddr,
    udp_socket: Arc<UdpSocket>,
    table: Arc<Mutex<KademliaTable>>,
    signer: SigningKey,
) {
    let mut buf = vec![0; MAX_DISC_PACKET_SIZE];

    loop {
        let (read, from) = udp_socket.recv_from(&mut buf).await.unwrap();
        info!("Received {read} bytes from {from}");

        let packet = Packet::decode(&buf[..read]);
        if packet.is_err() {
            debug!("Could not decode packet: {:?}", packet.err().unwrap());
            continue;
        }
        let packet = packet.unwrap();

        let msg = packet.get_message();
        info!("Message: {:?} from {}", msg, packet.get_node_id());

        match msg {
            Message::Ping(msg) => {
                if is_expired(msg.expiration) {
                    debug!("Ignoring ping as it is expired.");
                    continue;
                };
                let ping_hash = packet.get_hash();
                pong(&udp_socket, from, ping_hash, &signer).await;
                let node = {
                    let table = table.lock().await;
                    table.get_by_node_id(packet.get_node_id()).cloned()
                };
                if let Some(peer) = node {
                    // send a a ping to get an endpoint proof
                    if time_since_in_hs(peer.last_ping) > 12 {
                        let hash = ping(&udp_socket, udp_addr, from, &signer).await;
                        if let Some(hash) = hash {
                            table
                                .lock()
                                .await
                                .update_peer_ping(peer.node.node_id, Some(hash));
                        }
                    }
                } else {
                    // send a ping to get the endpoint proof from our end
                    let (peer, inserted_to_table) = {
                        let mut table = table.lock().await;
                        table.insert_node(Node {
                            ip: from.ip(),
                            udp_port: from.port(),
                            tcp_port: 0,
                            node_id: packet.get_node_id(),
                        })
                    };
                    let hash = ping(&udp_socket, udp_addr, from, &signer).await;
                    if let Some(hash) = hash {
                        if inserted_to_table && peer.is_some() {
                            let peer = peer.unwrap();
                            table
                                .lock()
                                .await
                                .update_peer_ping(peer.node.node_id, Some(hash));
                        }
                    }
                }
            }
            Message::Pong(msg) => {
                if is_expired(msg.expiration) {
                    debug!("Ignoring pong as it is expired.");
                    continue;
                }
                let peer = {
                    let table = table.lock().await;
                    table.get_by_node_id(packet.get_node_id()).cloned()
                };
                if let Some(peer) = peer {
                    if peer.last_ping_hash.is_none() {
                        debug!("Discarding pong as the node did not send a previous ping");
                        continue;
                    }
                    if peer.last_ping_hash.unwrap() == msg.ping_hash {
                        table.lock().await.mark_peer_as_proven(peer.node.node_id);
                    } else {
                        debug!(
                            "Discarding pong as the hash did not match the last corresponding ping"
                        );
                    }
                } else {
                    debug!("Discarding pong as it is not a known node");
                }
            }
            Message::FindNode(msg) => {
                if is_expired(msg.expiration) {
                    debug!("Ignoring find node msg as it is expired.");
                    continue;
                };
                let node = {
                    let table = table.lock().await;
                    table.get_by_node_id(packet.get_node_id()).cloned()
                };
                if let Some(node) = node {
                    if node.is_proven {
                        let nodes = {
                            let table = table.lock().await;
                            table.get_closest_nodes(msg.target)
                        };
                        let nodes_chunks = nodes.chunks(4);
                        let expiration = get_expiration(20);
                        debug!("Sending neighbors!");
                        // we are sending the neighbors in 4 different messages as not to exceed the
                        // maximum packet size
                        for nodes in nodes_chunks {
                            let neighbors = discv4::Message::Neighbors(NeighborsMessage::new(
                                nodes.to_vec(),
                                expiration,
                            ));
                            let mut buf = Vec::new();
                            neighbors.encode_with_header(&mut buf, &signer);
                            // we are going to send the nodes in four request as not to
                            udp_socket.send_to(&buf, from).await.unwrap();
                        }
                    } else {
                        debug!("Ignoring find node message as the node isn't proven!");
                    }
                } else {
                    debug!("Ignoring find node message as it is not a known node");
                }
            }
            Message::Neighbors(neighbors_msg) => {
                if is_expired(neighbors_msg.expiration) {
                    debug!("Ignoring neighbor msg as it is expired.");
                    continue;
                };

                let mut nodes_to_insert = None;
                let mut table = table.lock().await;
                if let Some(node) = table.get_by_node_id_mut(packet.get_node_id()) {
                    if let Some(req) = &mut node.find_node_request {
                        let nodes = &neighbors_msg.nodes;
                        let nodes_sent = req.nodes_sent + nodes.len();

                        if nodes_sent <= MAX_NODES_PER_BUCKET {
                            debug!("Storing neighbors in our table!");
                            req.nodes_sent = nodes_sent;
                            nodes_to_insert = Some(nodes.clone());
                        } else {
                            debug!("Ignoring neighbors message as the client sent more than the allowed nodes");
                        }

                        if nodes_sent == MAX_NODES_PER_BUCKET {
                            debug!("Neighbors request has been fulfilled");
                            node.find_node_request = None;
                        }
                    }
                } else {
                    debug!("Ignoring neighbor msg as it is not a known node");
                }

                if let Some(nodes) = nodes_to_insert {
                    for node in nodes {
                        let (peer, inserted_to_table) = table.insert_node(node);
                        if inserted_to_table && peer.is_some() {
                            let peer = peer.unwrap();
                            let node_addr = SocketAddr::new(peer.node.ip, peer.node.udp_port);
                            let ping_hash = ping(&udp_socket, udp_addr, node_addr, &signer).await;
                            table.update_peer_ping(peer.node.node_id, ping_hash);
                        };
                    }
                }
            }
            _ => {}
        }
    }
}

const REVALIDATION_INTERVAL_IN_MINUTES: usize = 10; // this is just an arbitrary number, maybe we should get this from some kind of cfg
const PROOF_EXPIRATION_IN_HS: usize = 12;

/// Starts a tokio scheduler that:
/// - performs periodic revalidation of the current nodes (sends a ping to the old nodes). Currently this is configured to happen every [`REVALIDATION_INTERVAL_IN_MINUTES`]
///
/// **Peer revalidation**
///
/// Peers revalidation works in the following manner:
/// 1. If the last ping has happened `PROOF_EXPIRATION_TIME_IN_HS`hs ago, we invalidate and send a ping to re-validate the endpoint proof
/// 2. In the next iteration, we check if the node has responded. If not, then we delete it and insert a new one from the replacements table
async fn peers_revalidation(
    udp_addr: SocketAddr,
    udp_socket: Arc<UdpSocket>,
    table: Arc<Mutex<KademliaTable>>,
    signer: SigningKey,
    interval_time_in_seconds: u64,
) {
    let mut interval = tokio::time::interval(Duration::from_secs(interval_time_in_seconds));

    // peers whose proof expired and we pinged them to revalidate them
    // we expect them to be proven by the next iteration otherwise we remove them
    let mut previously_pinged_peers: HashSet<H512> = HashSet::default();

    // first tick starts immediately
    interval.tick().await;

    loop {
        interval.tick().await;
        debug!("Running peer revalidation");

        let peers = {
            let mut table = table.lock().await;
            table.get_pinged_peers_since(PROOF_EXPIRATION_IN_HS as u64 * 60 * 60)
        };
        let mut peers_pending_revalidation: HashSet<H512> = HashSet::default();

        for peer in peers {
            if previously_pinged_peers.contains(&peer.node.node_id) {
                // Peer did not respond, replace it with a new peer from the replacements table
                if !peer.is_proven {
                    let new_peer = {
                        let mut table = table.lock().await;
                        table.replace_peer(peer.node.node_id)
                    };
                    debug!(
                        "Replacing peer {} with {:?} from table as it hasn't pinged back!",
                        peer.node.node_id, new_peer
                    );
                    if let Some(peer) = new_peer {
                        let ping_hash = ping(
                            &udp_socket,
                            udp_addr,
                            SocketAddr::new(peer.node.ip, peer.node.udp_port),
                            &signer,
                        )
                        .await;
                        let mut table = table.lock().await;
                        table.update_peer_ping(peer.node.node_id, ping_hash);
                        peers_pending_revalidation.insert(peer.node.node_id);
                    }
                }
                continue;
            }

            let ping_hash = ping(
                &udp_socket,
                udp_addr,
                SocketAddr::new(peer.node.ip, peer.node.udp_port),
                &signer,
            )
            .await;
            let mut table = table.lock().await;
            table.update_peer_ping(peer.node.node_id, ping_hash);
            peers_pending_revalidation.insert(peer.node.node_id);
            debug!(
                "Pinging peer {:?} to re-validate endpoint proof!",
                peer.node.node_id
            );
        }

        previously_pinged_peers = peers_pending_revalidation;
        debug!("Peer revalidation finished");
    }
}

const PEERS_RANDOM_LOOKUP_TIME_IN_MIN: usize = 10;

/// Starts a tokio scheduler that:
/// - performs random lookups to discover new nodes. Currently this is configure to run every `PEERS_RANDOM_LOOKUP_TIME_IN_MIN`
///
/// **Random lookups**
///
/// Random lookups work in the following manner:
/// 1. We send a find_node that is closest to our public key
/// 2. We select the 3 closest peers that we haven't select previously
/// 3. We send a find_node request to each one of them concurrently
///
/// To each find_node request it will correspond (or it should) a neighbors response
/// where we will decide whether we insert the node or not (at least it will be added to the replacement list).
///
/// See more https://github.com/ethereum/devp2p/blob/master/discv4.md#recursive-lookup
async fn peers_lookup(
    udp_socket: Arc<UdpSocket>,
    table: Arc<Mutex<KademliaTable>>,
    signer: SigningKey,
    local_node_id: H512,
    interval_time_in_seconds: u64,
) {
    let mut interval = tokio::time::interval(Duration::from_secs(interval_time_in_seconds));

    // here we store the peers that we have already send a find_node request
    let mut asked_peers: HashSet<H512> = HashSet::default();

    interval.tick().await;
    loop {
        interval.tick().await;

        // lookup closest to our pub key
        lookup(
            udp_socket.clone(),
            table.clone(),
            &signer,
            local_node_id,
            &mut asked_peers,
        )
        .await;

        // lookup closest to 3 random keys
        for _ in 0..3 {
            let random_pub_key = &SigningKey::random(&mut OsRng);
            lookup(
                udp_socket.clone(),
                table.clone(),
                &signer,
                node_id_from_signing_key(random_pub_key),
                &mut asked_peers,
            )
            .await;
        }
    }
}

async fn lookup(
    udp_socket: Arc<UdpSocket>,
    table: Arc<Mutex<KademliaTable>>,
    signer: &SigningKey,
    target: H512,
    asked_peers: &mut HashSet<H512>,
) {
    let alpha = 3;
    let closest_nodes = table.lock().await.get_closest_nodes(target);
    let mut queries = 0;
    for node in closest_nodes {
        if !asked_peers.contains(&node.node_id) {
            find_node(
                &udp_socket,
                SocketAddr::new(node.ip, node.udp_port),
                signer,
                target,
            )
            .await;
            let mut table = table.lock().await;
            let peer = table.get_by_node_id_mut(node.node_id);
            if let Some(peer) = peer {
                peer.new_find_node_request();
            }
            asked_peers.insert(node.node_id);
            queries += 1;
        }

        if queries == alpha {
            break;
        }
    }
}

/// Sends a ping to the addr
/// # Returns
/// an optional hash corresponding to the message header hash to account if the send was successful
async fn ping(
    socket: &UdpSocket,
    local_addr: SocketAddr,
    to_addr: SocketAddr,
    signer: &SigningKey,
) -> Option<H256> {
    let mut buf = Vec::new();

    let expiration: u64 = (SystemTime::now() + Duration::from_secs(20))
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
    let res = socket.send_to(&buf, to_addr).await;

    if res.is_err() {
        return None;
    }
    let bytes_sent = res.unwrap();

    // sanity check to make sure the ping was well sent
    // though idk if this is actually needed or if it might break other stuff
    if bytes_sent == buf.len() {
        return Some(H256::from_slice(&buf[0..32]));
    }

    None
}

async fn find_node(
    socket: &UdpSocket,
    to_addr: SocketAddr,
    signer: &SigningKey,
    target_node_id: H512,
) {
    let expiration: u64 = (SystemTime::now() + Duration::from_secs(20))
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let msg: discv4::Message =
        discv4::Message::FindNode(FindNodeMessage::new(target_node_id, expiration));

    let mut buf = Vec::new();
    msg.encode_with_header(&mut buf, signer);

    socket.send_to(&buf, to_addr).await.unwrap();
}

async fn pong(socket: &UdpSocket, to_addr: SocketAddr, ping_hash: H256, signer: &SigningKey) {
    let mut buf = Vec::new();

    let expiration: u64 = (SystemTime::now() + Duration::from_secs(20))
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

async fn serve_requests(tcp_addr: SocketAddr, signer: SigningKey) {
    let secret_key: SecretKey = signer.clone().into();
    let tcp_socket = TcpSocket::new_v4().unwrap();
    tcp_socket.bind(tcp_addr).unwrap();

    let mut udp_addr = tcp_addr;
    udp_addr.set_port(tcp_addr.port() + 1);
    let udp_socket = UdpSocket::bind(udp_addr).await.unwrap();

    // Try contacting a known peer
    // TODO: this is just an example, and we should do this dynamically
    let str_tcp_addr = "127.0.0.1:30307";
    let str_udp_addr = "127.0.0.1:30307";

    let udp_addr: SocketAddr = str_udp_addr.parse().unwrap();

    let mut buf = vec![0; MAX_DISC_PACKET_SIZE];

    let (msg, sig_bytes, endpoint) = loop {
        ping(&udp_socket, tcp_addr, udp_addr, &signer).await;

        let (read, from) = udp_socket.recv_from(&mut buf).await.unwrap();
        info!("RLPx: Received {read} bytes from {from}");
        let packet = Packet::decode(&buf[..read]).unwrap();
        info!("RLPx: Message: {:?}", packet);

        match packet.get_message() {
            Message::Pong(pong) => {
                break (&buf[32 + 65..read], &buf[32..32 + 65], pong.to);
            }
            Message::Ping(ping) => {
                break (&buf[32 + 65..read], &buf[32..32 + 65], ping.from);
            }
            _ => {
                warn!("Unexpected message type");
            }
        };
    };

    let digest = Keccak256::digest(msg);
    let signature = &Signature::from_bytes(sig_bytes[..64].into()).unwrap();
    let rid = RecoveryId::from_byte(sig_bytes[64]).unwrap();

    let peer_pk = VerifyingKey::recover_from_prehash(&digest, signature, rid).unwrap();

    let mut client = RLPxLocalClient::random();
    let mut auth_message = vec![];
    client.encode_auth_message(&secret_key, &peer_pk.into(), &mut auth_message);

    // NOTE: for some reason kurtosis peers don't publish their active TCP port
    let tcp_addr = endpoint
        .tcp_address()
        .unwrap_or(str_tcp_addr.parse().unwrap());

    let mut stream = TcpSocket::new_v4()
        .unwrap()
        .connect(tcp_addr)
        .await
        .unwrap();

    stream.write_all(&auth_message).await.unwrap();
    info!("Sent auth message correctly!");
    // Read the ack message's size
    stream.read_exact(&mut buf[..2]).await.unwrap();
    let auth_data = buf[..2].try_into().unwrap();
    let msg_size = u16::from_be_bytes(auth_data) as usize;

    // Read the rest of the ack message
    stream.read_exact(&mut buf[2..msg_size + 2]).await.unwrap();

    let msg = &mut buf[2..msg_size + 2];
    let mut pending_conn = client.decode_ack_message(&secret_key, msg, auth_data);
    info!("Completed handshake!");

    let hello_msg = RLPxMessage::Hello(p2p::HelloMessage::new(
        SUPPORTED_CAPABILITIES
            .into_iter()
            .map(|(name, version)| (name.to_string(), version))
            .collect(),
        PublicKey::from(signer.verifying_key()),
    ));

    pending_conn.send(hello_msg, &mut stream).await;

    // Receive Hello message
    let mut conn = pending_conn.receive(&mut stream).await;

    info!("Completed Hello roundtrip!");

    // Send Ping
    conn.send(RLPxMessage::Ping(p2p::PingMessage::new()), &mut stream)
        .await;

    // Receive three messages
    // TODO implement listen loop instead
    conn.receive(&mut stream).await;

    // Testing disconnect message
    // conn.send(
    //     RLPxMessage::Disconnect(p2p::DisconnectMessage::new(Some(3))),
    //     &mut stream,
    // )
    // .await;

    conn.receive(&mut stream).await;
    conn.receive(&mut stream).await;
}

pub fn node_id_from_signing_key(signer: &SigningKey) -> H512 {
    let public_key = PublicKey::from(signer.verifying_key());
    let encoded = public_key.to_encoded_point(false);
    H512::from_slice(&encoded.as_bytes()[1..])
}

#[cfg(test)]
mod tests {
    use super::*;
    use discv4::time_now_unix;
    use k256::ecdsa::SigningKey;
    use kademlia::bucket_number;
    use rand::rngs::OsRng;
    use std::net::{IpAddr, Ipv4Addr};
    use tokio::time::sleep;

    async fn insert_random_node_on_custom_bucket(
        table: Arc<Mutex<KademliaTable>>,
        bucket_idx: usize,
    ) {
        let node_id = node_id_from_signing_key(&SigningKey::random(&mut OsRng));
        let node = Node {
            ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            tcp_port: 0,
            udp_port: 0,
            node_id,
        };
        table
            .lock()
            .await
            .insert_node_on_custom_bucket(node, bucket_idx);
    }

    async fn fill_table_with_random_nodes(table: Arc<Mutex<KademliaTable>>) {
        for i in 0..256 {
            for _ in 0..16 {
                insert_random_node_on_custom_bucket(table.clone(), i).await;
            }
        }
    }

    #[tokio::test]
    /** This is a end to end test on the discovery server, the idea is as follows:
     * - We'll start two discovery servers (`a` & `b`) to ping between each other
     * - We'll make `b` ping `a`, and validate that the connection is right
     * - Then we'll wait for a revalidation where we expect everything to be the same
     * - Then we'll forcedly change the last_pong of `a` peer in `b` table
     *   such that in the next revalidation `b` re-validates `a`
     * - Finally, we'll forcedly change the last_pong as before but this time we won't answer from `a`.
     *   In this case, we expect that `b` first tries to re-validate `a`
     *   but in the next as `a` does not respond, `b` should removes `a` from its bucket.
     *
     * To make this run faster, we'll change the revalidation time to be every 2secs
     */
    async fn discovery_server_e2e() {
        // start server `a`
        let addr_a = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8000);
        let signer_a = SigningKey::random(&mut OsRng);
        let node_id_a = node_id_from_signing_key(&signer_a);
        tokio::spawn(discover_peers(addr_a, signer_a.clone(), vec![]));

        // for server `b` we won't use discover_peers fn
        // since we want to have access to the table to force some changes
        let addr_b = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8001);
        let signer_b = SigningKey::random(&mut OsRng);
        let udp_socket = Arc::new(UdpSocket::bind(addr_b).await.unwrap());
        let local_node_id = node_id_from_signing_key(&signer_b);
        let table = Arc::new(Mutex::new(KademliaTable::new(local_node_id)));

        tokio::spawn(discover_peers_server(
            addr_b,
            udp_socket.clone(),
            table.clone(),
            signer_b.clone(),
        ));
        tokio::spawn(peers_revalidation(
            addr_b,
            udp_socket.clone(),
            table.clone(),
            signer_b.clone(),
            2,
        ));

        let ping_hash = ping(&udp_socket, addr_b, addr_a, &signer_b).await;
        {
            let mut table = table.lock().await;
            table.insert_node(Node {
                ip: addr_a.ip(),
                udp_port: addr_a.port(),
                tcp_port: 0,
                node_id: node_id_a,
            });
            let peer = table
                .get_by_node_id_mut(node_id_from_signing_key(&signer_a))
                .unwrap();
            peer.last_ping_hash = ping_hash;
            peer.last_ping = time_now_unix();
        }

        // allow some time for server `a` to respond
        sleep(Duration::from_secs(1)).await;

        // server_a should've received the ping, and now we expect a pong to be received
        // so it should be proven
        {
            let table = table.lock().await;
            let peer = table.get_by_node_id(node_id_a).unwrap();
            assert!(peer.is_proven);
            assert!(peer.last_ping_hash.is_none());
        }

        // now we wait 2 seconds, so that a revalidation runs, we expect everything to be the same
        sleep(Duration::from_secs(2)).await;
        {
            let table = table.lock().await;
            let peer = table.get_by_node_id(node_id_a).unwrap();
            assert!(peer.is_proven);
            assert!(peer.last_ping_hash.is_none());
        }

        // now we are going to change the last pong to be from more than 24hs ago
        // we expect everything to stay the same after the revalidation
        {
            let mut table = table.lock().await;
            let peer = table
                .get_by_node_id_mut(node_id_from_signing_key(&signer_a))
                .unwrap();
            peer.last_pong = (SystemTime::now() - Duration::from_secs(24 * 60 * 60))
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            peer.last_ping = (SystemTime::now() - Duration::from_secs(24 * 60 * 60))
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
        }

        sleep(Duration::from_secs(3)).await;
        {
            let table = table.lock().await;
            let peer = table.get_by_node_id(node_id_a).unwrap();
            assert!(peer.is_proven);
            assert!(peer.last_ping_hash.is_none());
        }

        // though we expect the revalidation, to send the ping
        // we can make sure it has run by checking that the last ping and pong are recent
        {
            let mut table = table.lock().await;
            let peer = table.get_by_node_id_mut(node_id_a).unwrap();

            assert!(
                time_now_unix().saturating_sub(peer.last_ping) <= Duration::from_secs(10).as_secs()
            );
            assert!(
                time_now_unix().saturating_sub(peer.last_pong) <= Duration::from_secs(10).as_secs()
            );
        }

        // finally, we'll change the port of the server `a` so that no one responds and the peer is removed
        {
            let mut table = table.lock().await;
            let peer = table
                .get_by_node_id_mut(node_id_from_signing_key(&signer_a))
                .unwrap();
            peer.node.udp_port = 0;
            peer.last_pong = (SystemTime::now() - Duration::from_secs(24 * 60 * 60))
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            peer.last_ping = (SystemTime::now() - Duration::from_secs(24 * 60 * 60))
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
        }

        // first it will try to send a revalidation
        sleep(Duration::from_secs(3)).await;
        {
            let table = table.lock().await;
            let peer = table.get_by_node_id(node_id_a).unwrap();
            assert!(!peer.is_proven);
            assert!(
                time_now_unix().saturating_sub(peer.last_ping) <= Duration::from_secs(10).as_secs()
            );
        }

        // but it won't respond, so it should not exist anymore
        sleep(Duration::from_secs(2)).await;
        {
            let table = table.lock().await;
            let peer = table.get_by_node_id(node_id_a);
            assert!(peer.is_none());
        }
    }

    #[tokio::test]
    /** This test tests the lookup function, the idea is as follows:
     * - We'll start two discovery servers (`a` & `b`) that will connect between each other
     * - We'll insert random nodes to the server `a`` to fill its table
     * - We'll forcedly run `lookup` and validate that a `find_node` request was sent
     *   by checking that new nodes have been inserted to the table
     */
    async fn discovery_server_lookup() {
        let addr_a = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8002);
        let signer_a = SigningKey::random(&mut OsRng);
        let udp_socket_a = Arc::new(UdpSocket::bind(addr_a).await.unwrap());
        let node_id_a = node_id_from_signing_key(&signer_a);
        let table_a = Arc::new(Mutex::new(KademliaTable::new(node_id_a)));
        fill_table_with_random_nodes(table_a.clone()).await;

        tokio::spawn(discover_peers_server(
            addr_a,
            udp_socket_a.clone(),
            table_a.clone(),
            signer_a.clone(),
        ));

        let addr_b = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8003);
        let signer_b = SigningKey::random(&mut OsRng);
        let udp_socket_b = Arc::new(UdpSocket::bind(addr_b).await.unwrap());
        let node_id_b = node_id_from_signing_key(&signer_b);
        let table_b = Arc::new(Mutex::new(KademliaTable::new(node_id_b)));

        tokio::spawn(discover_peers_server(
            addr_b,
            udp_socket_b.clone(),
            table_b.clone(),
            signer_b.clone(),
        ));

        // before making the connection, remove a node from the `b` bucket. Otherwise it won't be added
        let b_bucket = bucket_number(node_id_a, node_id_b);
        let node_id_to_remove = table_a.lock().await.buckets()[b_bucket].peers[0]
            .node
            .node_id;
        table_a
            .lock()
            .await
            .replace_peer_on_custom_bucket(node_id_to_remove, b_bucket);

        let ping_hash = ping(&udp_socket_b, addr_b, addr_a, &signer_b).await;
        {
            let mut table = table_b.lock().await;
            table.insert_node(Node {
                ip: addr_a.ip(),
                udp_port: addr_a.port(),
                tcp_port: 0,
                node_id: node_id_a,
            });
            table.update_peer_ping(node_id_a, ping_hash);
        }

        // allow some time for the handshake
        sleep(Duration::from_secs(1)).await;

        // now we are going to run a lookup with us as the target
        let closets_peers_to_b_from_a = table_a.lock().await.get_closest_nodes(node_id_b);
        let mut asked_peers = HashSet::default();
        lookup(
            udp_socket_b.clone(),
            table_b.clone(),
            &signer_b,
            node_id_b,
            &mut asked_peers,
        )
        .await;

        // find_node sent, allow some time for `a` to respond
        sleep(Duration::from_secs(2)).await;

        // now all peers should've been inserted
        for peer in closets_peers_to_b_from_a {
            let table = table_b.lock().await;
            assert!(table.get_by_node_id(peer.node_id).is_some());
        }
    }
}
