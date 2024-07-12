use crate::discv4::Node;
use ethereum_rust_core::{H256, H512, U256};
use sha3::{Digest, Keccak256};
use std::net::IpAddr;

const MAX_NODES_PER_BUCKET: usize = 16;
const NUMBER_OF_BUCKETS: usize = 256;

#[derive(Debug)]
pub struct KademliaTable {
    local_node_id: H512,
    buckets: Vec<Vec<PeerData>>,
}

impl KademliaTable {
    pub fn new(local_node_id: H512) -> Self {
        let buckets: Vec<Vec<PeerData>> = vec![vec![]; NUMBER_OF_BUCKETS];
        Self {
            local_node_id,
            buckets,
        }
    }

    pub fn insert(&mut self, peer: PeerData) {
        let bucket_number = bucket_number(self.local_node_id, peer.node_id);
        let bucket = &mut self.buckets[bucket_number];
        if bucket.len() == MAX_NODES_PER_BUCKET {
            // TODO: revalidate least recently seen node as described in
            // <https://github.com/ethereum/devp2p/blob/master/discv4.md#kademlia-table>
            bucket.pop();
        }
        bucket.push(peer);
    }
}

/// Computes the distance between two nodes according to the discv4 protocol
/// and returns the corresponding bucket number
/// <https://github.com/ethereum/devp2p/blob/master/discv4.md#node-identities>
pub fn bucket_number(node_id_1: H512, node_id_2: H512) -> usize {
    let hash_1 = Keccak256::digest(node_id_1);
    let hash_2 = Keccak256::digest(node_id_2);
    let xor = H256(hash_1.into()) ^ H256(hash_2.into());
    let distance = U256::from_big_endian(xor.as_bytes());
    distance.bits() - 1
}

#[derive(Clone, Debug)]
#[allow(unused)]
pub struct PeerData {
    pub ip: IpAddr,
    pub udp_port: u16,
    pub tcp_port: u16,
    pub node_id: H512,
}

impl From<Node> for PeerData {
    fn from(node: Node) -> Self {
        Self {
            ip: node.ip,
            udp_port: node.udp_port,
            tcp_port: node.tcp_port,
            node_id: node.node_id,
        }
    }
}
