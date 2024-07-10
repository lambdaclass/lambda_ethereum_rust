const MAX_NODES_PER_BUCKET: usize = 16;
const NUMBER_OF_BUCKETS: usize = 256;
use crate::discv4::Node;
use ethereum_rust_core::{H512, U256};
use keccak_hash::keccak;

pub struct KademliaTable {
    node_id: H512,
    buckets: Vec<Vec<Node>>,
}

impl KademliaTable {
    pub fn new(node_id: H512) -> Self {
        let buckets: Vec<Vec<Node>> = vec![vec![]; NUMBER_OF_BUCKETS];
        Self { node_id, buckets }
    }

    pub fn insert_node(&mut self, node: Node) {
        let bucket_number = bucket_number(self.node_id, node.node_id);
        let bucket = &mut self.buckets[bucket_number];
        if bucket.len() == MAX_NODES_PER_BUCKET {
            // TODO: revalidate least recently seen node as described in
            // <https://github.com/ethereum/devp2p/blob/master/discv4.md#kademlia-table>
            bucket.pop();
        }
        bucket.push(node);
    }
}

/// Computes the distance between two nodes according to the discv4 protocol
/// and returns the corresponding bucket number
/// <https://github.com/ethereum/devp2p/blob/master/discv4.md#node-identities>
pub fn bucket_number(node_id_1: H512, node_id_2: H512) -> usize {
    let hash_1 = keccak(node_id_1);
    let hash_2 = keccak(node_id_2);
    let xor = hash_1 ^ hash_2;
    let distance = U256::from_big_endian(xor.as_bytes());
    distance.bits() - 1
}
