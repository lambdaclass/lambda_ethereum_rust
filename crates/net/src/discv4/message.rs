use enr::{secp256k1::SecretKey, Enr};
use fastrlp::{BufMut, Decodable, Encodable, Header};
use std::net::IpAddr;

#[derive(Debug)]
pub enum Message {
    Ping {
        id: u64,
        enr_seq: u64,
    },
    Pong {
        enr_seq: u64,
    },
    FindNode {
        distance: u64,
        target: Ip,
    },
    Neighbours {
        neighbours: Vec<Ip>,
    },
    EnrRequest {
        expire: u64,
    },
    EnrResponse {
        request_hash: [u8; 32],
        enr: Enr<SecretKey>,
    },
}

impl Message {
    /// Returns the message id of the message.
    pub fn message_id(&self) -> u8 {
        match self {
            Message::Ping { .. } => 0x01,
            Message::Pong { .. } => 0x02,
            Message::FindNode { .. } => 0x03,
            Message::Neighbours { .. } => 0x04,
            Message::EnrRequest { .. } => 0x05,
            Message::EnrResponse { .. } => 0x06,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Ip(IpAddr);

impl Encodable for Ip {
    fn encode(&self, out: &mut dyn BufMut) {
        match self.0 {
            IpAddr::V4(address) => address.octets().encode(out),
            IpAddr::V6(address) => address.octets().encode(out),
        }
    }

    fn length(&self) -> usize {
        match self.0 {
            IpAddr::V4(address) => address.octets().length(),
            IpAddr::V6(address) => address.octets().length(),
        }
    }
}

impl Decodable for Ip {
    fn decode(buf: &mut &[u8]) -> Result<Self, fastrlp::DecodeError> {
        match Header::decode(buf)?.payload_length {
            0 => Err(fastrlp::DecodeError::Custom("Ip address cannot be empty")),
            4 => Ok(Self((<[u8; 4]>::decode(buf)?).into())),
            16 => Ok(Self((<[u8; 16]>::decode(buf)?).into())),
            _ => Err(fastrlp::DecodeError::Custom("Wrong IP address length")),
        }
    }
}
