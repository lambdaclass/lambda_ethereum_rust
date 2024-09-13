use crate::{
    discv4::{time_now_unix, FindNodeRequest},
    types::Node,
};
use ethereum_rust_core::{H256, H512, U256};
use sha3::{Digest, Keccak256};

pub const MAX_NODES_PER_BUCKET: usize = 16;
const NUMBER_OF_BUCKETS: usize = 256;
const MAX_NUMBER_OF_REPLACEMENTS: usize = 10;

#[derive(Debug)]
pub struct KademliaTable {
    local_node_id: H512,
    buckets: Vec<Vec<PeerData>>,
    replacements: Vec<PeerData>,
}

impl KademliaTable {
    pub fn new(local_node_id: H512) -> Self {
        let buckets: Vec<Vec<PeerData>> = vec![vec![]; NUMBER_OF_BUCKETS];
        Self {
            local_node_id,
            buckets,
            replacements: Vec::with_capacity(MAX_NUMBER_OF_REPLACEMENTS),
        }
    }

    pub fn get_by_node_id(&self, node_id: H512) -> Option<&PeerData> {
        let bucket = &self.buckets[bucket_number(node_id, self.local_node_id)];
        bucket.iter().find(|entry| entry.node.node_id == node_id)
    }

    pub fn get_by_node_id_mut(&mut self, node_id: H512) -> Option<&mut PeerData> {
        let bucket = &mut self.buckets[bucket_number(node_id, self.local_node_id)];
        bucket
            .iter_mut()
            .find(|entry| entry.node.node_id == node_id)
    }

    /// Will try to insert a node into the table. If the table is full then it pushes it to the replacement list.
    /// # Returns
    /// A tuple containing:
    ///     1. PeerData
    ///     2. A bool indicating if the node was inserted to the table
    pub fn insert_node(&mut self, node: Node) -> (&mut PeerData, bool) {
        let node_id = node.node_id;
        let peer = PeerData::new(node, time_now_unix(), false);
        let bucket_idx = bucket_number(node_id, self.local_node_id);

        if self.buckets[bucket_idx].len() == MAX_NODES_PER_BUCKET {
            (self.insert_as_replacement(&peer), false)
        } else {
            self.remove_from_replacements(node_id);
            self.buckets[bucket_idx].push(peer);
            let peer_idx = self.buckets[bucket_idx].len() - 1;
            (&mut self.buckets[bucket_idx][peer_idx], true)
        }
    }

    fn insert_as_replacement(&mut self, node: &PeerData) -> &mut PeerData {
        if self.replacements.len() >= MAX_NUMBER_OF_REPLACEMENTS {
            self.replacements.pop();
        }
        self.replacements.insert(0, node.clone());
        &mut self.replacements[0]
    }

    fn remove_from_replacements(&mut self, node_id: H512) {
        self.replacements = self
            .replacements
            .drain(..)
            .filter(|r| r.node.node_id != node_id)
            .collect();
    }

    pub fn get_closest_nodes(&self, node_id: H512) -> Vec<Node> {
        let mut nodes: Vec<(Node, usize)> = vec![];

        // todo see if there is a more efficient way of doing this
        // though the bucket isn't that large and it shouldn't be an issue I guess
        for bucket in &self.buckets {
            for peer in bucket {
                let distance = bucket_number(node_id, peer.node.node_id);
                if nodes.len() < MAX_NODES_PER_BUCKET {
                    nodes.push((peer.node, distance));
                } else {
                    for (i, (_, dis)) in &mut nodes.iter().enumerate() {
                        if distance < *dis {
                            nodes[i] = (peer.node, distance);
                            break;
                        }
                    }
                }
            }
        }

        nodes.iter().map(|a| a.0).collect()
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
    distance.bits().saturating_sub(1)
}

#[derive(Debug, Clone)]
pub struct PeerData {
    pub node: Node,
    pub last_ping: u64,
    pub last_ping_hash: Option<H256>,
    pub is_proven: bool,
    pub find_node_request: Option<FindNodeRequest>,
}

impl PeerData {
    pub fn new(record: Node, last_ping: u64, is_proven: bool) -> Self {
        Self {
            node: record,
            last_ping,
            is_proven,
            last_ping_hash: None,
            find_node_request: None,
        }
    }

    pub fn new_find_node_request(&mut self) {
        self.find_node_request = Some(FindNodeRequest::default());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex_literal::hex;

    #[test]
    fn bucket_number_works_as_expected() {
        let node_id_1 = H512(hex!("4dc429669029ceb17d6438a35c80c29e09ca2c25cc810d690f5ee690aa322274043a504b8d42740079c4f4cef50777c991010208b333b80bee7b9ae8e5f6b6f0"));
        let node_id_2 = H512(hex!("034ee575a025a661e19f8cda2b6fd8b2fd4fe062f6f2f75f0ec3447e23c1bb59beb1e91b2337b264c7386150b24b621b8224180c9e4aaf3e00584402dc4a8386"));
        let expected_bucket = 255;
        let result = bucket_number(node_id_1, node_id_2);
        assert_eq!(result, expected_bucket);
    }
}
