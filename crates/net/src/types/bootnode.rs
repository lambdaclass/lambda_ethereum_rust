use std::{net::SocketAddr, num::ParseIntError, str::FromStr};

#[derive(Debug, Clone)]
pub struct BootNode {
    pub node_id: Vec<u8>,
    pub socket_address: SocketAddr,
}

impl FromStr for BootNode {
    type Err = ParseIntError;
    fn from_str(input: &str) -> Result<BootNode, ParseIntError> {
        // TODO: error handling
        let node_id = decode_hex(&input[8..136])?;
        let socket_address: SocketAddr = input[137..]
            .parse()
            .expect("Failed to parse bootnode address and port");
        Ok(BootNode {
            node_id,
            socket_address,
        })
    }
}

pub fn decode_hex(s: &str) -> Result<Vec<u8>, ParseIntError> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
        .collect()
}
