use std::collections::HashMap;

use ethereum_types::H256;
use sha3::{Digest, Keccak256};

use crate::{nibbles::Nibbles, node::Node, node_hash::NodeHash, Trie, TrieError, ValueRLP};

/// The boolead indicates if there is more state to be fetched
fn verify_range_proof(
    root: H256,
    first_key: H256,
    keys: Vec<H256>,
    values: Vec<ValueRLP>,
    proof: Vec<Vec<u8>>,
) -> Result<bool, TrieError> {
    // Store proof nodes by hash
    let proof_nodes = ProofNodeStorage::from_proof(&proof);
    if keys.len() != values.len() {
        return Err(TrieError::Verify(format!(
            "inconsistent proof data, got {} keys and {} values",
            keys.len(),
            values.len()
        )));
    }
    // Check that the key range is monotonically increasing
    for keys in keys.windows(2) {
        if keys[0] >= keys[1] {
            return Err(TrieError::Verify(String::from(
                "key range is not monotonically increasing",
            )));
        }
    }
    // Check for empty values
    if values.iter().find(|value| value.is_empty()).is_some() {
        return Err(TrieError::Verify(String::from(
            "value range contains empty value",
        )));
    }

    // Verify ranges depending on the given proof

    // Case A) No proofs given, the range is expected to be the full set of leaves
    if proof.is_empty() {
        let mut trie = Trie::stateless();
        for (index, key) in keys.iter().enumerate() {
            // Ignore the error as we don't rely on a DB
            let _ = trie.insert(key.0.to_vec(), values[index].clone());
        }
        let hash = trie.hash().unwrap_or_default();
        if hash != root {
            return Err(TrieError::Verify(format!(
                "invalid proof, expected root hash {}, got  {}",
                root, hash
            )));
        }
        return Ok(false);
    }

    // Case B) One edge proof no range given, there are no more values in the trie
    if keys.is_empty() {
        let (has_right_element, value) =
            has_right_element(root, first_key.as_bytes(), &proof_nodes)?;
        if has_right_element || value.is_empty() {
            return Err(TrieError::Verify(format!(
                "no keys returned but more are available on the trie"
            )));
        }
    }

    Ok(true)
}

// Traverses the path till the last node is reached
// Check weather there are no more values in the trie

// Indicates where there exist more elements to the right side of the given path
// Also returns the value (or an empty value if it is not present on the trie)
fn has_right_element(
    root_hash: H256,
    key: &[u8],
    proof_nodes: &ProofNodeStorage,
) -> Result<(bool, Vec<u8>), TrieError> {
    let path = Nibbles::from_bytes(key);
    let node = proof_nodes.get_node(&root_hash.into())?;
    has_right_element_inner(&node, path, proof_nodes)
}

fn has_right_element_inner(
    node: &Node,
    mut path: Nibbles,
    proof_nodes: &ProofNodeStorage,
) -> Result<(bool, Vec<u8>), TrieError> {
    match node {
        Node::Branch(ref n) => {
            // Check if there are children to the right side
            if let Some(choice) = path.next_choice() {
                if n.choices[choice..].iter().any(|child| child.is_valid()) {
                    Ok((true, vec![]))
                } else {
                    let node = proof_nodes.get_node(&n.choices[choice])?;
                    has_right_element_inner(&node, path, proof_nodes)
                }
            } else {
                Ok((false, n.value.clone()))
            }
        }
        Node::Extension(n) => {
            if path.skip_prefix(&n.prefix) {
                let node = proof_nodes.get_node(&n.child)?;
                has_right_element_inner(&node, path, proof_nodes)
            } else {
                Ok((n.prefix.as_ref() > path.as_ref(), vec![]))
            }
        }
        // We reached the end of the path
        Node::Leaf(ref n) => {
            let value = (path == n.partial)
                .then_some(n.value.clone())
                .unwrap_or_default();
            Ok((false, value))
        }
    }
}

fn get_child<'a>(path: &'a mut Nibbles, node: &'a Node) -> Option<&'a NodeHash> {
    match node {
        Node::Branch(n) => path.next_choice().map(|i| &n.choices[i]),
        Node::Extension(n) => path.skip_prefix(&n.prefix).then_some(&n.child),
        Node::Leaf(_) => None,
    }
}

struct ProofNodeStorage<'a> {
    nodes: HashMap<Vec<u8>, &'a Vec<u8>>,
}

impl<'a> ProofNodeStorage<'a> {
    fn from_proof(proof: &'a Vec<Vec<u8>>) -> Self {
        Self {
            nodes: proof
                .iter()
                .map(|node| (Keccak256::new_with_prefix(node).finalize().to_vec(), node))
                .collect::<HashMap<_, _>>(),
        }
    }

    fn get_node(&self, hash: &NodeHash) -> Result<Node, TrieError> {
        let encoded = match hash {
            NodeHash::Hashed(hash) => {
                let Some(encoded) = self.nodes.get(hash.as_bytes()) else {
                    return Err(TrieError::Verify(format!("proof node missing: {hash}")));
                };
                *encoded
            }

            NodeHash::Inline(ref encoded) => encoded,
        };
        Ok(Node::decode_raw(encoded)?)
    }
}
