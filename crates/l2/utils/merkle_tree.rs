use keccak_hash::{keccak, H256};

pub fn merkelize(data: Vec<H256>) -> H256 {
    let mut data = data;
    while data.len() > 1 {
        data = data
            .chunks(2)
            .map(|chunk| {
                let left = chunk[0];
                let right = if chunk.len() == 2 { chunk[1] } else { left };
                keccak([keccak(left.0).as_bytes(), keccak(right.0).as_bytes()].concat())
            })
            .collect();
    }
    data[0]
}
