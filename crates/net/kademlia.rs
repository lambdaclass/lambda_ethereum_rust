use crate::{
    discv4::{time_now_unix, FindNodeRequest},
    types::Node,
};
use ethereum_rust_core::{H256, H512, U256};
use sha3::{Digest, Keccak256};

pub const MAX_NODES_PER_BUCKET: usize = 16;
const NUMBER_OF_BUCKETS: usize = 256;
const MAX_NUMBER_OF_REPLACEMENTS: usize = 10;

#[derive(Clone, Debug, Default)]
pub struct Bucket {
    pub peers: Vec<PeerData>,
    pub replacements: Vec<PeerData>,
}

#[derive(Debug)]
pub struct KademliaTable {
    local_node_id: H512,
    buckets: Vec<Bucket>,
}

impl KademliaTable {
    pub fn new(local_node_id: H512) -> Self {
        let buckets: Vec<Bucket> = vec![Bucket::default(); NUMBER_OF_BUCKETS];
        Self {
            local_node_id,
            buckets,
        }
    }

    #[allow(unused)]
    pub fn buckets(&self) -> &Vec<Bucket> {
        &self.buckets
    }

    pub fn buckets_mut(&mut self) -> &mut Vec<Bucket> {
        &mut self.buckets
    }

    pub fn get_by_node_id(&self, node_id: H512) -> Option<&PeerData> {
        let bucket = &self.buckets[bucket_number(node_id, self.local_node_id)];
        bucket
            .peers
            .iter()
            .find(|entry| entry.node.node_id == node_id)
    }

    pub fn get_by_node_id_mut(&mut self, node_id: H512) -> Option<&mut PeerData> {
        let bucket = &mut self.buckets[bucket_number(node_id, self.local_node_id)];
        bucket
            .peers
            .iter_mut()
            .find(|entry| entry.node.node_id == node_id)
    }

    /// Will try to insert a node into the table. If the table is full then it pushes it to the replacement list.
    /// # Returns
    /// A tuple containing:
    ///     1. PeerData
    ///     2. A bool indicating if the node was inserted to the table
    pub fn insert_node(&mut self, node: Node) -> (PeerData, bool) {
        let node_id = node.node_id;
        let bucket_idx = bucket_number(node_id, self.local_node_id);

        self.insert_node_inner(node, bucket_idx)
    }

    #[cfg(test)]
    fn insert_node_on_custom_bucket(&mut self, node: Node, bucket_idx: usize) -> (PeerData, bool) {
        self.insert_node_inner(node, bucket_idx)
    }

    fn insert_node_inner(&mut self, node: Node, bucket_idx: usize) -> (PeerData, bool) {
        let node_id = node.node_id;
        let peer = PeerData::new(node, time_now_unix(), 0, false);

        if self.buckets[bucket_idx].peers.len() == MAX_NODES_PER_BUCKET {
            self.insert_as_replacement(&peer, bucket_idx);
            (peer, false)
        } else {
            self.remove_from_replacements(node_id, bucket_idx);
            self.buckets[bucket_idx].peers.push(peer.clone());
            (peer, true)
        }
    }

    fn insert_as_replacement(&mut self, node: &PeerData, bucket_idx: usize) {
        let bucket = &mut self.buckets[bucket_idx];
        if bucket.replacements.len() >= MAX_NUMBER_OF_REPLACEMENTS {
            bucket.replacements.remove(0);
        }
        bucket.replacements.push(node.clone());
    }

    fn remove_from_replacements(&mut self, node_id: H512, bucket_idx: usize) {
        let bucket = &mut self.buckets[bucket_idx];

        bucket.replacements = bucket
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
            for peer in &bucket.peers {
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

    pub fn pong_answered(&mut self, node_id: H512) {
        let peer = self.get_by_node_id_mut(node_id);
        if peer.is_none() {
            return;
        }

        let peer = peer.unwrap();
        peer.is_proven = true;
        peer.last_pong = time_now_unix();
        peer.last_ping_hash = None;
        if peer.revalidation.is_some() {
            peer.revalidation = Some(true);
        }
    }

    pub fn update_peer_ping(&mut self, node_id: H512, ping_hash: Option<H256>) {
        let peer = self.get_by_node_id_mut(node_id);
        if peer.is_none() {
            return;
        }

        let peer = peer.unwrap();
        peer.last_ping_hash = ping_hash;
        peer.last_ping = time_now_unix();
    }

    pub fn update_peer_ping_with_revalidation(&mut self, node_id: H512, ping_hash: Option<H256>) {
        let peer = self.get_by_node_id_mut(node_id);
        if peer.is_none() {
            return;
        }

        let peer = peer.unwrap();
        peer.last_ping_hash = ping_hash;
        peer.last_ping = time_now_unix();
        peer.revalidation = Some(false);
    }

    #[allow(unused)]
    /// ## Returns
    /// The peers whose pong back is past the since. Since is given in seconds and it is not an actual data
    /// but a Duration.
    ///
    /// ### Example
    /// table.get_pinged_peers_since(24 * 60 * 60) // this would return the peers that haven't ponged for 24hs
    pub fn get_pinged_peers_since(&mut self, since: u64) -> Vec<PeerData> {
        let mut peers = vec![];

        for bucket in &self.buckets {
            for peer in &bucket.peers {
                if time_now_unix().saturating_sub(peer.last_pong) >= since {
                    peers.push(peer.clone());
                }
            }
        }

        peers
    }

    /// Replaces the peer with the given id with the latest replacement stored.
    /// If there are no replacements, it simply remove it
    ///
    /// # Returns
    ///
    /// A mutable reference to the inserted peer or None in case there was no replacement
    pub fn replace_peer(&mut self, node_id: H512) -> Option<PeerData> {
        let bucket_idx = bucket_number(self.local_node_id, node_id);
        self.replace_peer_inner(node_id, bucket_idx)
    }

    #[cfg(test)]
    fn replace_peer_on_custom_bucket(
        &mut self,
        node_id: H512,
        bucket_idx: usize,
    ) -> Option<PeerData> {
        self.replace_peer_inner(node_id, bucket_idx)
    }

    fn replace_peer_inner(&mut self, node_id: H512, bucket_idx: usize) -> Option<PeerData> {
        let idx_to_remove = self.buckets[bucket_idx]
            .peers
            .iter()
            .position(|peer| peer.node.node_id == node_id);

        if let Some(idx) = idx_to_remove {
            let bucket = &mut self.buckets[bucket_idx];
            let new_peer = bucket.replacements.pop();

            if let Some(new_peer) = new_peer {
                bucket.peers[idx] = new_peer.clone();
                return Some(new_peer);
            } else {
                bucket.peers.remove(idx);
                return None;
            }
        };

        None
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
    pub last_pong: u64,
    pub last_ping_hash: Option<H256>,
    pub is_proven: bool,
    pub find_node_request: Option<FindNodeRequest>,
    /// a ration to track the peers's ping responses
    pub liveness: u16,
    /// if a revalidation was sent to the peer, the bool marks if it has answered
    pub revalidation: Option<bool>,
}

impl PeerData {
    pub fn new(record: Node, last_ping: u64, last_pong: u64, is_proven: bool) -> Self {
        Self {
            node: record,
            last_ping,
            last_pong,
            is_proven,
            liveness: 1,
            last_ping_hash: None,
            find_node_request: None,
            revalidation: None,
        }
    }

    pub fn new_find_node_request(&mut self) {
        self.find_node_request = Some(FindNodeRequest::default());
    }

    pub fn increment_liveness(&mut self) {
        self.liveness += 1;
    }

    pub fn decrement_liveness(&mut self) {
        self.liveness /= 3;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{node_id_from_signing_key, PROOF_EXPIRATION_IN_HS};
    use hex_literal::hex;
    use k256::{ecdsa::SigningKey, elliptic_curve::rand_core::OsRng};
    use std::{
        net::{IpAddr, Ipv4Addr},
        time::{Duration, SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn bucket_number_works_as_expected() {
        let node_id_1 = H512(hex!("4dc429669029ceb17d6438a35c80c29e09ca2c25cc810d690f5ee690aa322274043a504b8d42740079c4f4cef50777c991010208b333b80bee7b9ae8e5f6b6f0"));
        let node_id_2 = H512(hex!("034ee575a025a661e19f8cda2b6fd8b2fd4fe062f6f2f75f0ec3447e23c1bb59beb1e91b2337b264c7386150b24b621b8224180c9e4aaf3e00584402dc4a8386"));
        let expected_bucket = 255;
        let result = bucket_number(node_id_1, node_id_2);
        assert_eq!(result, expected_bucket);
    }

    fn insert_random_node_on_custom_bucket(
        table: &mut KademliaTable,
        bucket_idx: usize,
    ) -> (PeerData, bool) {
        let node_id = node_id_from_signing_key(&SigningKey::random(&mut OsRng));
        let node = Node {
            ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            tcp_port: 0,
            udp_port: 0,
            node_id,
        };
        table.insert_node_on_custom_bucket(node, bucket_idx)
    }

    fn fill_table_with_random_nodes(table: &mut KademliaTable) {
        for i in 0..256 {
            for _ in 0..16 {
                insert_random_node_on_custom_bucket(table, i);
            }
        }
    }

    fn get_test_table() -> KademliaTable {
        let signer = SigningKey::random(&mut OsRng);
        let local_node_id = node_id_from_signing_key(&signer);

        KademliaTable::new(local_node_id)
    }

    #[test]
    fn get_peers_since_should_return_the_right_peers() {
        let mut table = get_test_table();
        let node_1_id = node_id_from_signing_key(&SigningKey::random(&mut OsRng));
        {
            table.insert_node(Node {
                ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                tcp_port: 0,
                udp_port: 0,
                node_id: node_1_id,
            });
            table.get_by_node_id_mut(node_1_id).unwrap().last_pong = (SystemTime::now()
                - Duration::from_secs(12 * 60 * 60))
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        }

        let node_2_id = node_id_from_signing_key(&SigningKey::random(&mut OsRng));
        {
            table.insert_node(Node {
                ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                tcp_port: 0,
                udp_port: 0,
                node_id: node_2_id,
            });
            table.get_by_node_id_mut(node_2_id).unwrap().last_pong = (SystemTime::now()
                - Duration::from_secs(36 * 60 * 60))
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        }

        let node_3_id = node_id_from_signing_key(&SigningKey::random(&mut OsRng));
        {
            table.insert_node(Node {
                ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                tcp_port: 0,
                udp_port: 0,
                node_id: node_3_id,
            });
            table.get_by_node_id_mut(node_3_id).unwrap().last_pong = (SystemTime::now()
                - Duration::from_secs(10 * 60 * 60))
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        }

        // we expect the node_1 & node_2 to be returned here
        let peers: Vec<H512> = table
            .get_pinged_peers_since(PROOF_EXPIRATION_IN_HS as u64 * 60 * 60)
            .iter()
            .map(|p| p.node.node_id)
            .collect();

        assert!(peers.contains(&node_1_id));
        assert!(peers.contains(&node_2_id));
        assert!(!peers.contains(&node_3_id));
    }

    #[test]
    fn insert_peer_should_remove_first_replacement_when_list_is_full() {
        let mut table = get_test_table();
        fill_table_with_random_nodes(&mut table);
        let bucket_idx = 0;

        let (first_node, inserted_to_table) =
            insert_random_node_on_custom_bucket(&mut table, bucket_idx);
        assert!(!inserted_to_table);

        // here we are forcingly pushing to the first bucket, that is, the distance might
        // not be in accordance with the bucket index
        // but we don't care about that here, we just want to check if the replacement works as expected
        for _ in 1..MAX_NUMBER_OF_REPLACEMENTS {
            let (_, inserted_to_table) =
                insert_random_node_on_custom_bucket(&mut table, bucket_idx);
            assert!(!inserted_to_table);
        }

        {
            let bucket = &table.buckets[bucket_idx];
            assert_eq!(first_node.node.node_id, bucket.replacements[0].node.node_id);
        }

        // push one more element, this should replace the first one pushed
        let (last, inserted_to_table) = insert_random_node_on_custom_bucket(&mut table, bucket_idx);
        assert!(!inserted_to_table);

        let bucket = &table.buckets[bucket_idx];
        assert_ne!(first_node.node.node_id, bucket.replacements[0].node.node_id);
        assert_eq!(
            last.node.node_id,
            bucket.replacements[MAX_NUMBER_OF_REPLACEMENTS - 1]
                .node
                .node_id
        );
    }

    #[test]
    fn replace_peer_should_replace_peer() {
        let mut table = get_test_table();
        let bucket_idx = 0;
        fill_table_with_random_nodes(&mut table);

        let (replacement_peer, inserted_to_table) =
            insert_random_node_on_custom_bucket(&mut table, bucket_idx);
        assert!(!inserted_to_table);

        let node_id_to_replace = table.buckets[bucket_idx].peers[0].node.node_id;
        let replacement = table.replace_peer_on_custom_bucket(node_id_to_replace, bucket_idx);

        assert_eq!(
            replacement.unwrap().node.node_id,
            replacement_peer.node.node_id
        );
        assert_eq!(
            table.buckets[bucket_idx].peers[0].node.node_id,
            replacement_peer.node.node_id
        );
    }
    #[test]
    fn replace_peer_should_remove_peer_but_not_replace() {
        // here, we will remove the peer, but with no replacements peers available
        let mut table = get_test_table();
        let bucket_idx = 0;
        fill_table_with_random_nodes(&mut table);

        let node_id_to_replace = table.buckets[bucket_idx].peers[0].node.node_id;
        let len_before = table.buckets[bucket_idx].peers.len();
        let replacement = table.replace_peer_on_custom_bucket(node_id_to_replace, bucket_idx);
        let len_after = table.buckets[bucket_idx].peers.len();

        assert!(replacement.is_none());
        assert!(len_before - 1 == len_after);
    }
}
