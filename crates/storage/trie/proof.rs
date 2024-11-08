use ethereum_types::H256;

use crate::{Trie, TrieError, ValueRLP};

/// The boolead indicates if there is more state to be fetched
fn verify_range_proof(root: H256, first_key: H256, keys: Vec<H256>, values: Vec<ValueRLP>, proof: Vec<Vec<u8>>) -> Result<bool, TrieError> {
    if keys.len() != values.len() {
        return Err(TrieError::Verify(format!("inconsistent proof data, got {} keys and {} values", keys.len(), values.len())));
    }
    // Check that the key range is monotonically increasing
    for keys in keys.windows(2) {
        if keys[0] >= keys[1] {
            return Err(TrieError::Verify(String::from("key range is not monotonically increasing")));
        }
    }
    // Check for empty values
    if values.iter().find(|value| value.is_empty()).is_some() {
        return Err(TrieError::Verify(String::from("value range contains empty value")));
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
            return Err(TrieError::Verify(format!("invalid proof, expected root hash {}, got  {}", root, hash)));
        }
        return Ok(false)
    }

    // Case B) One edge proof no range given, there are no more values in the trie
    if keys.is_empty() {
        
    }


    Ok(true)
}