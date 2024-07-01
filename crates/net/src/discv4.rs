use std::net::IpAddr;

use bytes::BufMut;
use ethrex_core::rlp::{encode::RLPEncode, structs};
use k256::ecdsa::{signature::Signer, SigningKey};

#[derive(Debug)]
// TODO: remove when all variants are used
// NOTE: All messages could have more fields than specified by the spec.
// Those additional fields should be ignored, and the message must be accepted.
#[allow(dead_code)]
pub(crate) enum Message {
    /// A ping message. Should be responded to with a Pong message.
    Ping(PingMessage),
    Pong(()),
    FindNode(()),
    Neighbors(()),
    ENRRequest(()),
    ENRResponse(()),
}

impl Message {
    pub fn encode_with_header(&self, buf: &mut dyn BufMut, node_signer: SigningKey) {
        let signature_size = 65_usize;
        let mut data: Vec<u8> = Vec::with_capacity(signature_size.next_power_of_two());
        data.resize(signature_size, 0);
        data.push(self.packet_type());
        match self {
            Message::Ping(msg) => msg.encode(&mut data),
            _ => todo!(),
        }

        let digest = keccak_hash::keccak_buffer(&mut &data[signature_size..]).unwrap();

        let (signature, recovery_id) = node_signer.try_sign(&digest.0).expect("failed to sign");
        let b = signature.to_bytes();

        data[..signature_size - 1].copy_from_slice(&b);
        data[signature_size - 1] = recovery_id.to_byte();

        let hash = keccak_hash::keccak_buffer(&mut &data[..]).unwrap();
        buf.put_slice(&hash.0);
        buf.put_slice(&data[..]);
    }

    fn packet_type(&self) -> u8 {
        match self {
            Message::Ping(_) => 0x01,
            Message::Pong(_) => 0x02,
            Message::FindNode(_) => 0x03,
            Message::Neighbors(_) => 0x04,
            Message::ENRRequest(_) => 0x05,
            Message::ENRResponse(_) => 0x06,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Endpoint {
    pub ip: IpAddr,
    pub udp_port: u16,
    pub tcp_port: u16,
}

impl RLPEncode for Endpoint {
    fn encode(&self, buf: &mut dyn BufMut) {
        (self.ip, self.udp_port, self.tcp_port).encode(buf);
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct PingMessage {
    /// The Ping message version. Should be set to 4, but mustn't be enforced.
    version: u8,
    /// The endpoint of the sender.
    from: Endpoint,
    /// The endpoint of the receiver.
    to: Endpoint,
    /// The expiration time of the message. If the message is older than this time,
    /// it shouldn't be responded to.
    expiration: u64,
    /// The ENR sequence number of the sender. This field is optional.
    enr_seq: Option<u64>,
}

impl PingMessage {
    pub fn new(from: Endpoint, to: Endpoint, expiration: u64) -> Self {
        Self {
            version: 4,
            from,
            to,
            expiration,
            enr_seq: None,
        }
    }

    // TODO: remove when used
    #[allow(unused)]
    pub fn with_enr_seq(self, enr_seq: u64) -> Self {
        Self {
            enr_seq: Some(enr_seq),
            ..self
        }
    }
}

impl RLPEncode for PingMessage {
    fn encode(&self, buf: &mut dyn BufMut) {
        structs::Encoder::new(buf)
            .encode_field(&self.version)
            .encode_field(&self.from)
            .encode_field(&self.to)
            .encode_field(&self.expiration)
            .encode_optional_field(&self.enr_seq)
            .finish();
    }
}

#[cfg(test)]
mod tests {
    use std::{fmt::Write, str::FromStr};

    use super::*;
    use keccak_hash::H256;

    fn to_hex(bytes: &[u8]) -> String {
        bytes.iter().fold(String::new(), |mut buf, b| {
            let _ = write!(&mut buf, "{b:02x}");
            buf
        })
    }

    #[test]
    fn test_encode_ping_message() {
        let expiration: u64 = 17195043770;

        let from = Endpoint {
            ip: IpAddr::from_str("1.2.3.4").unwrap(),
            udp_port: 1613,
            tcp_port: 6363,
        };
        let to = Endpoint {
            ip: IpAddr::from_str("255.255.2.5").unwrap(),
            udp_port: 3063,
            tcp_port: 0,
        };

        let msg = Message::Ping(PingMessage::new(from, to, expiration));

        let key_bytes =
            H256::from_str("577d8278cc7748fad214b5378669b420f8221afb45ce930b7f22da49cbc545f3")
                .unwrap();
        let signer = SigningKey::from_slice(key_bytes.as_bytes()).unwrap();

        let mut buf = Vec::new();

        msg.encode_with_header(&mut buf, signer);
        let result = to_hex(&buf);
        let hash = "d9b83d9701c6481a99db908b19551c6b082bcb28d5bef44cfa55256bc7977500";
        let signature = "f0bff907b5c432e623ba5d3803d6a405bdbaffdfc0373499ac2a243ef3ab52de3a5312c0a9a96593979b746a4cd37ebdf21cf6971cf8c10c94f4d45c1a0f90dd00";
        let pkt_type = "01";
        let encoded_message = "dd04cb840102030482064d8218dbc984ffff0205820bf780850400e78bba";
        let expected = [hash, signature, pkt_type, encoded_message].concat();

        assert_eq!(result, expected);
    }

    #[test]
    fn test_decode_pong_message() {
        let hash = "2e1fc2a02ad95a1742f6dd41fb7cbff1e08548ba87f63a72221e44026ab1c347";
        let signature = "34f486e4e92f2fdf592912aa77ad51db532dd7f9b426092384c9c2e9919414fd480d57f4f3b2b1964ed6eb1c94b1e4b9f6bfe9b44b1d1ac3d94c38c4cce915bc01";
        let pkt_type = "02";
        let msg = "f7c984bebfbc3982765f80a03e1bf98f025f98d54ed2f61bbef63b6b46f50e12d7b937d6bdea19afd640be2384667d9af086018cf3c3bcdd";
        let _encoded_packet = [hash, signature, pkt_type, msg].concat();
        // TODO
    }
}
