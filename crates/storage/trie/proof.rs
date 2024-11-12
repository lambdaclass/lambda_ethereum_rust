use std::collections::HashMap;

use ethereum_types::H256;
use sha3::{Digest, Keccak256};

use crate::{
    nibbles::Nibbles, node::Node, node_hash::NodeHash, state::TrieState, trie_iter::print_trie,
    Trie, TrieError, ValueRLP,
};

/// The boolean indicates if there is more state to be fetched
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

    // Special Case A) No proofs given, the range is expected to be the full set of leaves
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

    let last_key = *keys.last().unwrap();

    // Special Case B) One edge proof no range given, there are no more values in the trie
    if keys.is_empty() {
        let (has_right_element, value) =
            has_right_element(root, first_key.as_bytes(), &proof_nodes)?;
        if has_right_element || value.is_empty() {
            return Err(TrieError::Verify(format!(
                "no keys returned but more are available on the trie"
            )));
        }
    }

    // Special Case C) There is only one element and the two edge keys are the same
    if keys.len() == 1 && first_key == last_key {
        let (has_right_element, value) =
            has_right_element(root, first_key.as_bytes(), &proof_nodes)?;
        if first_key != keys[0] {
            return Err(TrieError::Verify(format!("correct proof but invalid key")));
        }
        if value != values[0] {
            return Err(TrieError::Verify(format!("correct proof but invalid data")));
        }
        return Ok(has_right_element);
    }

    // Regular Case
    // Here we will have two edge proofs
    if first_key >= last_key {
        return Err(TrieError::Verify(format!("invalid edge keys")));
    }
    let mut trie = Trie::stateless();
    trie.root = Some(NodeHash::from(root));
    let _ = fill_state(&mut trie.state, root, first_key, &proof_nodes)?;
    let _ = fill_state(&mut trie.state, root, last_key, &proof_nodes)?;
    println!("FILL STATE");
    print_trie(&trie);
    remove_internal_references(root, first_key, last_key, &mut trie.state);
    println!("REMOVE INTERNAL REFERENCES");
    print_trie(&trie);
    println!("KEY RANGE INSERT");
    for (i, key) in keys.iter().enumerate() {
        trie.insert(key.0.to_vec(), values[i].clone())?;
    }
    // TODO: has_right_element
    assert_eq!(trie.hash().unwrap(), root);

    // Use first proof to build node path
    // use first proof root + second proof to complete it
    // Remove internal references
    // Add keys & values from range
    // Check root

    Ok(true)
}

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

fn get_child<'a>(path: &'a mut Nibbles, node: &'a Node) -> Option<NodeHash> {
    match node {
        Node::Branch(n) => path.next_choice().map(|i| n.choices[i].clone()),
        Node::Extension(n) => path.skip_prefix(&n.prefix).then_some(n.child.clone()),
        Node::Leaf(_) => None,
    }
}

/// Fills up the TrieState with nodes from the proof traversing the path given by first_key
/// Also returns the value if it is part of the proof
fn fill_state(
    trie_state: &mut TrieState,
    root_hash: H256,
    first_key: H256,
    proof_nodes: &ProofNodeStorage,
) -> Result<Vec<u8>, TrieError> {
    let mut path = Nibbles::from_bytes(&first_key.0);
    fill_node(
        &mut path,
        &NodeHash::from(root_hash),
        trie_state,
        proof_nodes,
    )
}

fn fill_node(
    path: &mut Nibbles,
    node_hash: &NodeHash,
    trie_state: &mut TrieState,
    proof_nodes: &ProofNodeStorage,
) -> Result<Vec<u8>, TrieError> {
    let node = proof_nodes.get_node(node_hash)?;
    let child_hash = get_child(path, &node);
    if let Some(ref child_hash) = child_hash {
        trie_state.insert_node(node, node_hash.clone());
        fill_node(path, child_hash, trie_state, proof_nodes)
    } else {
        let value = match &node {
            Node::Branch(n) => n.value.clone(),
            Node::Extension(_) => vec![],
            Node::Leaf(n) => n.value.clone(),
        };
        trie_state.insert_node(node, node_hash.clone());
        Ok(value)
    }
}

/// Removes references to internal nodes not contained in the state
/// These should be reconstructed when verifying the proof
fn remove_internal_references(
    root_hash: H256,
    left_key: H256,
    right_key: H256,
    trie_state: &mut TrieState,
) {
    // First find the node at which the left and right path differ
    let left_path = Nibbles::from_bytes(&left_key.0);
    let right_path = Nibbles::from_bytes(&right_key.0);

    remove_internal_references_inner(NodeHash::from(root_hash), left_path, right_path, trie_state);
}

fn remove_internal_references_inner(
    node_hash: NodeHash,
    mut left_path: Nibbles,
    mut right_path: Nibbles,
    trie_state: &mut TrieState,
) {
    // We already looked up the nodes when filling the state so this shouldn't fail
    let node = trie_state.get_node(node_hash.clone()).unwrap().unwrap();
    match node {
        Node::Branch(mut n) => {
            let left_choice = left_path.next_choice().unwrap();
            let right_choice = right_path.next_choice().unwrap();
            if left_choice == right_choice && n.choices[left_choice].is_valid() {
                // Keep going
                return remove_internal_references_inner(
                    n.choices[left_choice].clone(),
                    left_path,
                    right_path,
                    trie_state,
                );
            }
            // We found our fork node, now we can remove the internal references
            for choice in &mut n.choices[left_choice..right_choice] {
                *choice = NodeHash::default()
            }
            // Remove nodes on the left and right choice's subtries
            remove_nodes(
                &node_hash,
                n.choices[left_choice].clone(),
                left_path,
                false,
                trie_state,
            );
            remove_nodes(
                &node_hash,
                n.choices[right_choice].clone(),
                right_path,
                true,
                trie_state,
            );
            // Update node in the state
            trie_state.insert_node(n.into(), node_hash);
        }
        Node::Extension(n) => todo!(),
        Node::Leaf(_) => todo!(),
    }
}

fn remove_nodes(
    parent_hash: &NodeHash,
    node_hash: NodeHash,
    mut path: Nibbles,
    remove_left: bool,
    trie_state: &mut TrieState,
) {
    let node = trie_state.get_node(node_hash.clone()).unwrap().unwrap();
    match node {
        Node::Branch(mut n) => {
            // Remove child nodes
            let choice = path.next_choice().unwrap();
            if remove_left {
                for child in &mut n.choices[..choice] {
                    *child = NodeHash::default()
                }
            } else {
                for child in &mut n.choices[choice..] {
                    *child = NodeHash::default()
                }
                // Remove nodes to the left/right of the choice's subtrie
                remove_nodes(
                    &node_hash,
                    n.choices[choice].clone(),
                    path,
                    remove_left,
                    trie_state,
                );
            }
            // Update node in the state
            trie_state.insert_node(n.into(), node_hash);
        }
        Node::Extension(extension_node) => todo!(),
        Node::Leaf(leaf_node) => todo!(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_range_proof_regular_case() {
        // The trie will have keys and values ranging from 25-100
        // We will prove the range from 50-75
        // Note values are written as hashes in the form i -> [i;32]
        let mut trie = Trie::new_temp();
        for k in 25..100_u8 {
            trie.insert([k; 32].to_vec(), [k; 32].to_vec()).unwrap()
        }
        let mut proof = trie.get_proof(&[50; 32].to_vec()).unwrap();
        proof.extend(trie.get_proof(&[75; 32].to_vec()).unwrap());
        let root = trie.hash().unwrap();
        let keys = (50_u8..=75).map(|i| H256([i; 32])).collect::<Vec<_>>();
        let values = (50_u8..=75).map(|i| [i; 32].to_vec()).collect::<Vec<_>>();
        verify_range_proof(root, keys[0], keys, values, proof).unwrap();
    }
}
