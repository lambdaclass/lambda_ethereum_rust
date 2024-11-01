/// Converts a slice of compact-encoded nibbles into a byte slice
/// If the nibble slice has odd-length (aka the last byte will be a half byte) returns true else false
pub fn compact_nibbles_to_bytes(compact: &[u8]) -> (Vec<u8>, bool) {
    // Convert compact nibbles to nibbles
    let nibbles = compact_to_hex(compact);
    // Convert nibbles to bytes, accouning for odd number of bytes
    let mut last_is_half = false;
    let bytes = nibbles
        .chunks(2)
        .map(|chunk| match chunk.len() {
            1 => {
                last_is_half = true;
                chunk[0] << 4
            }
            // 2
            _ => chunk[0] << 4 | chunk[1],
        })
        .collect::<Vec<_>>();
    (bytes, last_is_half)
}

// Code taken from https://github.com/ethereum/go-ethereum/blob/a1093d98eb3260f2abf340903c2d968b2b891c11/trie/encoding.go#L82
fn compact_to_hex(compact: &[u8]) -> Vec<u8> {
    if compact.is_empty() {
        return vec![];
    }
    let mut base = keybytes_to_hex(compact);
    // delete terminator flag
    if base[0] < 2 {
        base = base[..base.len() - 1].to_vec();
    }
    // apply odd flag
    let chop = 2 - (base[0] & 1) as usize;
    base[chop..].to_vec()
}

// Code taken from https://github.com/ethereum/go-ethereum/blob/a1093d98eb3260f2abf340903c2d968b2b891c11/trie/encoding.go#L96
fn keybytes_to_hex(keybytes: &[u8]) -> Vec<u8> {
    let l = keybytes.len() * 2 + 1;
    let mut nibbles = vec![0; l];
    for (i, b) in keybytes.iter().enumerate() {
        nibbles[i * 2] = b / 16;
        nibbles[i * 2 + 1] = b % 16;
    }
    nibbles[l - 1] = 16;
    nibbles
}
