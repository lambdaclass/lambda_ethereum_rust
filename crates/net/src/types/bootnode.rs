use ethrex_core::H512;
use std::{net::SocketAddr, num::ParseIntError, str::FromStr};

#[derive(Debug, Clone)]
pub struct BootNode {
    pub node_id: H512,
    pub socket_address: SocketAddr,
}

impl FromStr for BootNode {
    type Err = ParseIntError;
    /// Takes a str with the format "enode://<node ID>@<IP address>:<port>" and
    /// parses it to a BootNode
    fn from_str(input: &str) -> Result<BootNode, ParseIntError> {
        // TODO: error handling
        let node_id_as_bytes = decode_hex(&input[8..136])?;
        let node_id = H512::from_slice(&node_id_as_bytes);
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
