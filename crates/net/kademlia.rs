use crate::{discv4::time_now_unix, types::Node};
use ethereum_rust_core::{H256, H512, U256};
use sha3::{Digest, Keccak256};

const MAX_NODES_PER_BUCKET: usize = 16;
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

    #[allow(unused)]
    pub fn get_by_node_id(&self, node_id: H512) -> Option<&PeerData> {
        let bucket = &self.buckets[bucket_number(node_id, self.local_node_id)];
        for entry in bucket {
            if entry.node.node_id == node_id {
                return Some(entry);
            }
        }

        return None;
    }

    pub fn get_by_node_id_mut(&mut self, node_id: H512) -> Option<&mut PeerData> {
        let bucket = &mut self.buckets[bucket_number(node_id, self.local_node_id)];
        for entry in bucket {
            if entry.node.node_id == node_id {
                return Some(entry);
            }
        }

        return None;
    }

    /// Will try to insert a node into the table:
    /// - If the node is already inserted then it updates it
    /// - If the none is not inserted it will try to create a new entry
    /// - If the bucket is full then it adds it to the possible replacements table
    pub fn insert_node(&mut self, peer: Node) {
        let node_id = peer.node_id;
        if let Some(node) = self.get_by_node_id_mut(peer.node_id) {
            node.is_proven = true;
            node.last_ping = time_now_unix();
            node.last_ping_hash = None;
            // we also want to update the peer data, for the node might have changed its ports or ip
            node.node = peer;
            return;
        }
        let node = PeerData::new(peer, time_now_unix(), false);
        let bucket_idx = bucket_number(node_id, self.local_node_id);

        if self.buckets[bucket_idx].len() == MAX_NODES_PER_BUCKET {
            self.insert_as_replacement(&node);
        } else {
            self.remove_from_replacements(node_id);
            self.buckets[bucket_idx].push(node);
        }
    }

    fn insert_as_replacement(&mut self, node: &PeerData) {
        let entry = self
            .replacements
            .iter_mut()
            .find(|e| e.node.node_id == node.node.node_id);

        if let Some(entry) = entry {
            entry.is_proven = true;
            entry.last_ping = time_now_unix();
            entry.node = node.node.clone();
        } else {
            if self.replacements.len() >= MAX_NUMBER_OF_REPLACEMENTS {
                self.replacements.pop();
            }
            self.replacements.insert(0, node.clone());
        }
    }

    fn remove_from_replacements(&mut self, node_id: H512) {
        self.replacements = self
            .replacements
            .drain(..)
            .filter(|r| r.node.node_id != node_id)
            .collect();
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
}

impl PeerData {
    pub fn new(record: Node, last_ping: u64, is_proven: bool) -> Self {
        Self {
            node: record,
            last_ping,
            is_proven,
            last_ping_hash: None,
        }
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
