use std::{
    cmp::{self, Ordering},
    collections::HashMap,
};

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
/// Returns true if the trie is left empty (rootless) as a result of this process
fn remove_internal_references(
    root_hash: H256,
    left_key: H256,
    right_key: H256,
    trie_state: &mut TrieState,
) -> bool {
    // First find the node at which the left and right path differ
    let left_path = Nibbles::from_bytes(&left_key.0);
    let right_path = Nibbles::from_bytes(&right_key.0);

    remove_internal_references_inner(NodeHash::from(root_hash), left_path, right_path, trie_state)
}

// Return = true -> child should be removed
fn remove_internal_references_inner(
    node_hash: NodeHash,
    mut left_path: Nibbles,
    mut right_path: Nibbles,
    trie_state: &mut TrieState,
) -> bool {
    // We already looked up the nodes when filling the state so this shouldn't fail
    let node = trie_state.get_node(node_hash.clone()).unwrap().unwrap();
    match node {
        Node::Branch(mut n) => {
            let left_choice = left_path.next_choice().unwrap();
            let right_choice = right_path.next_choice().unwrap();
            if left_choice == right_choice && n.choices[left_choice].is_valid() {
                // Keep going
                // Check if the child extension node should be removed as a result of this process
                let should_remove = remove_internal_references_inner(
                    n.choices[left_choice].clone(),
                    left_path,
                    right_path,
                    trie_state,
                );
                if should_remove {
                    n.choices[left_choice] = NodeHash::default();
                    trie_state.insert_node(n.into(), node_hash);
                }
            } else {
                // We found our fork node, now we can remove the internal references
                for choice in &mut n.choices[left_choice + 1..right_choice] {
                    *choice = NodeHash::default()
                }
                // Remove nodes on the left and right choice's subtries
                let should_remove_left =
                    remove_node(n.choices[left_choice].clone(), left_path, false, trie_state);
                let should_remove_right = remove_node(
                    n.choices[right_choice].clone(),
                    right_path,
                    true,
                    trie_state,
                );
                if should_remove_left {
                    n.choices[left_choice] = NodeHash::default();
                }
                if should_remove_right {
                    n.choices[right_choice] = NodeHash::default();
                }
                // Update node in the state
                trie_state.insert_node(n.into(), node_hash);
            }
        }
        Node::Extension(n) => {
            // Compare left and right paths against prefix
            let compare_path = |path: &Nibbles, prefix: &Nibbles| -> Ordering {
                if path.len() > prefix.len() {
                    path.as_ref()[..prefix.len()].cmp(prefix.as_ref())
                } else {
                    path.as_ref().cmp(prefix.as_ref())
                }
            };

            let left_fork = compare_path(&left_path, &n.prefix);
            let right_fork = compare_path(&right_path, &n.prefix);

            if left_fork.is_eq() && right_fork.is_eq() {
                // Keep going
                return remove_internal_references_inner(
                    n.child,
                    left_path.offset(n.prefix.len()),
                    right_path.offset(n.prefix.len()),
                    trie_state,
                );
            }
            // We found our fork node, now we can remove the internal references
            match (left_fork, right_fork) {
                // If both paths are greater or lesser than the node's prefix then the range is empty
                // TODO: return the error instead of panicking here
                (Ordering::Greater, Ordering::Greater) | (Ordering::Less, Ordering::Less) => {
                    panic!("empty range")
                }
                // None of the paths fit the prefix, remove the entire subtrie
                (left, right) if left.is_ne() && right.is_ne() => {
                    // Return true so that the parent node knows they need to remove this node
                    return true;
                }
                // One path fits the prefix, the other one doesn't
                (left, right) => {
                    // If the child is a leaf node, tell parent to remove the node -> we will let the child handle this
                    let path = if left.is_eq() { left_path } else { right_path };
                    // If the child node is removed then this node will be removed too so we will leave that to the parent
                    return remove_node(node_hash, path, right.is_eq(), trie_state);
                }
            }
        }
        Node::Leaf(_) => todo!(),
    }
    false
}

// Removes all nodes in the node's subtrie to the left or right of the path (given by the `remove_left` flag)
// If the whole subtrie is removed in the process this function will return true, in which case
// the caller must remove the reference to this node from it's parent node
fn remove_node(
    node_hash: NodeHash,
    mut path: Nibbles,
    remove_left: bool,
    trie_state: &mut TrieState,
) -> bool {
    // Node doesn't exist already, no need to remove it
    if !node_hash.is_valid() {
        return false;
    }
    // We already checked the canonical proof path when filling the state so this case should be unreachable
    let Ok(Some(node)) = trie_state.get_node(node_hash.clone()) else {
        return false;
    };
    match node {
        Node::Branch(mut n) => {
            // Remove child nodes
            let choice = path.next_choice().unwrap();
            if remove_left {
                for child in &mut n.choices[..choice] {
                    *child = NodeHash::default()
                }
            } else {
                for child in &mut n.choices[choice + 1..] {
                    *child = NodeHash::default()
                }
            }
            // Remove nodes to the left/right of the choice's subtrie
            let should_remove =
                remove_node(n.choices[choice].clone(), path, remove_left, trie_state);
            if should_remove {
                n.choices[choice] = NodeHash::default();
            }
            // Update node in the state
            trie_state.insert_node(n.into(), node_hash);
        }
        Node::Extension(n) => {
            // If no child subtrie would result from this process remove the node entirely
            // (Such as removing the left side of a trie with no right side)
            if !path.skip_prefix(&n.prefix) {
                if (remove_left && n.prefix.as_ref() < path.as_ref())
                    || (!remove_left && n.prefix.as_ref() > path.as_ref())
                {
                    return true;
                }
            } else {
                // Remove left/right side of the child subtrie
                return remove_node(n.child, path, remove_left, trie_state);
            }
        }
        Node::Leaf(_) => return true,
    }
    false
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
    use proptest::collection::{btree_set, vec};
    use proptest::prelude::any;
    use proptest::{bool, proptest};
    use std::str::FromStr;

    #[test]
    fn verify_range_proof_regular_case_only_branch_nodes() {
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

    #[test]
    fn verify_range_proof_regular_case() {
        // The account ranges were taken form a hive test state, but artificially modified
        // so that the resulting trie has a wide variety of different nodes (and not only branches)
        let account_addresses: [&str; 26] = [
            "0xaa56789abcde80cde11add7d3447cd4ca93a5f2205d9874261484ae180718bd6",
            "0xaa56789abcdeda9ae19dd26a33bd10bbf825e28b3de84fc8fe1d15a21645067f",
            "0xaa56789abc39a8284ef43790e3a511b2caa50803613c5096bc782e8de08fa4c5",
            "0xaa5678931f4754834b0502de5b0342ceff21cde5bef386a83d2292f4445782c2",
            "0xaa567896492bfe767f3d18be2aab96441c449cd945770ef7ef8555acc505b2e4",
            "0xaa5f478d53bf78add6fa3708d9e061d59bfe14b21329b2a4cf1156d4f81b3d2d",
            "0xaa67c643f67b47cac9efacf6fcf0e4f4e1b273a727ded155db60eb9907939eb6",
            "0xaa04d8eaccf0b942c468074250cbcb625ec5c4688b6b5d17d2a9bdd8dd565d5a",
            "0xaa63e52cda557221b0b66bd7285b043071df4c2ab146260f4e010970f3a0cccf",
            "0xaad9aa4f67f8b24d70a0ffd757e82456d9184113106b7d9e8eb6c3e8a8df27ee",
            "0xaa3df2c3b574026812b154a99b13b626220af85cd01bb1693b1d42591054bce6",
            "0xaa79e46a5ed8a88504ac7d579b12eb346fbe4fd7e281bdd226b891f8abed4789",
            "0xbbf68e241fff876598e8e01cd529bd76416b248caf11e0552047c5f1d516aab6",
            "0xbbf68e241fff876598e8e01cd529c908cdf0d646049b5b83629a70b0117e2957",
            "0xbbf68e241fff876598e8e0180b89744abb96f7af1171ed5f47026bdf01df1874",
            "0xbbf68e241fff876598e8a4cd8e43f08be4715d903a0b1d96b3d9c4e811cbfb33",
            "0xbbf68e241fff8765182a510994e2b54d14b731fac96b9c9ef434bc1924315371",
            "0xbbf68e241fff87655379a3b66c2d8983ba0b2ca87abaf0ca44836b2a06a2b102",
            "0xbbf68e241fffcbcec8301709a7449e2e7371910778df64c89f48507390f2d129",
            "0xbbf68e241ffff228ed3aa7a29644b1915fde9ec22e0433808bf5467d914e7c7a",
            "0xbbf68e24190b881949ec9991e48dec768ccd1980896aefd0d51fd56fd5689790",
            "0xbbf68e2419de0a0cb0ff268c677aba17d39a3190fe15aec0ff7f54184955cba4",
            "0xbbf68e24cc6cbd96c1400150417dd9b30d958c58f63c36230a90a02b076f78b5",
            "0xbbf68e2490f33f1d1ba6d1521a00935630d2c81ab12fa03d4a0f4915033134f3",
            "0xc017b10a7cc3732d729fe1f71ced25e5b7bc73dc62ca61309a8c7e5ac0af2f72",
            "0xc098f06082dc467088ecedb143f9464ebb02f19dc10bd7491b03ba68d751ce45",
        ];
        let mut account_addresses = account_addresses
            .iter()
            .map(|addr| H256::from_str(addr).unwrap())
            .collect::<Vec<_>>();
        account_addresses.sort();
        let trie_values = account_addresses
            .iter()
            .map(|addr| addr.0.to_vec())
            .collect::<Vec<_>>();
        let key_range = account_addresses[7..=17].to_vec();
        let value_range = account_addresses[7..=17]
            .iter()
            .map(|v| v.0.to_vec())
            .collect::<Vec<_>>();
        let mut trie = Trie::new_temp();
        for val in trie_values.iter() {
            trie.insert(val.clone(), val.clone()).unwrap()
        }
        let mut proof = trie.get_proof(&trie_values[7]).unwrap();
        proof.extend(trie.get_proof(&trie_values[17]).unwrap());
        let root = trie.hash().unwrap();
        verify_range_proof(root, key_range[0], key_range, value_range, proof).unwrap();
    }

    proptest! {

        #[test]
        // Regular Case: Two Edge Proofs, both keys exist
        fn proptest_verify_range_regular_case(data in btree_set(vec(any::<u8>(), 32), 200), start in 1_usize..=100_usize, end in 101..200_usize) {
            // Build trie
            let mut trie = Trie::new_temp();
            for val in data.iter() {
                trie.insert(val.clone(), val.clone()).unwrap()
            }
            let root = trie.hash().unwrap();
            // Select range to prove
            let values = data.into_iter().collect::<Vec<_>>()[start..=end].to_vec();
            let keys = values.iter().map(|a| H256::from_slice(a)).collect::<Vec<_>>();
            // Generate proofs
            let mut proof = trie.get_proof(&values[0]).unwrap();
            proof.extend(trie.get_proof(&values.last().unwrap()).unwrap());
            // Verify the range proof
            verify_range_proof(root, keys[0], keys, values, proof).unwrap();
        }

        #[test]
        // Two Edge Proofs, first and last keys dont exist
        fn proptest_verify_range_nonexistant_edge_keys(data in btree_set(vec(1..u8::MAX-1, 32), 200), start in 1_usize..=100_usize, end in 101..199_usize) {
            let data = data.into_iter().collect::<Vec<_>>();
            // Build trie
            let mut trie = Trie::new_temp();
            for val in data.iter() {
                trie.insert(val.clone(), val.clone()).unwrap()
            }
            let root = trie.hash().unwrap();
            // Select range to prove
            let values = data[start..=end].to_vec();
            let keys = values.iter().map(|a| H256::from_slice(a)).collect::<Vec<_>>();
            // Select the first and last keys
            // As we will be using non-existant keys we will choose values that are `just` higer/lower than
            // the first and last values in our key range
            // Skip the test entirely in the unlucky case that the values just next to the edge keys are also part of the trie
            let mut first_key = data[start].clone();
            first_key[31] -=1;
            if first_key == data[start -1] {
                // Skip test
                return Ok(());
            }
            let mut last_key = data[end].clone();
            last_key[31] +=1;
            if last_key == data[end +1] {
                // Skip test
                return Ok(());
            }
            // Generate proofs
            let mut proof = trie.get_proof(&first_key).unwrap();
            proof.extend(trie.get_proof(&last_key).unwrap());
            // Verify the range proof
            verify_range_proof(root, H256::from_slice(&first_key), keys, values, proof).unwrap();
        }

        #[test]
        // Two Edge Proofs, one key doesn't exist
        fn proptest_verify_range_one_key_doesnt_exist(data in btree_set(vec(1..u8::MAX-1, 32), 200), start in 1_usize..=100_usize, end in 101..199_usize, first_key_exists in bool::ANY) {
            let data = data.into_iter().collect::<Vec<_>>();
            // Build trie
            let mut trie = Trie::new_temp();
            for val in data.iter() {
                trie.insert(val.clone(), val.clone()).unwrap()
            }
            let root = trie.hash().unwrap();
            // Select range to prove
            let values = data[start..=end].to_vec();
            let keys = values.iter().map(|a| H256::from_slice(a)).collect::<Vec<_>>();
            // Select the first and last keys
            // As we will be using non-existant keys we will choose values that are `just` higer/lower than
            // the first and last values in our key range
            // Skip the test entirely in the unlucky case that the values just next to the edge keys are also part of the trie
            let mut first_key = data[start].clone();
            let mut last_key = data[end].clone();
            if first_key_exists {
                last_key[31] +=1;
                if last_key == data[end +1] {
                    // Skip test
                    return Ok(());
                }
            } else {
            first_key[31] -=1;
            if first_key == data[start -1] {
                // Skip test
                return Ok(());
            }
            }
            // Generate proofs
            let mut proof = trie.get_proof(&first_key).unwrap();
            proof.extend(trie.get_proof(&last_key).unwrap());
            // Verify the range proof
            verify_range_proof(root, H256::from_slice(&first_key), keys, values, proof).unwrap();
        }
    }
}
