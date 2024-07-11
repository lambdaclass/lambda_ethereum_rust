use bytes::BufMut;
use ethereum_rust_core::rlp::{
    decode::RLPDecode,
    encode::RLPEncode,
    error::RLPDecodeError,
    structs::{self, Decoder, Encoder},
};
use ethereum_rust_core::{H256, H264, H512, H520};
use k256::ecdsa::SigningKey;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

#[allow(unused)]
pub struct Packet {
    hash: H256,
    signature: H520,
    message: Message,
}

impl Packet {
    pub fn decode(encoded_packet: &[u8]) -> Result<Packet, RLPDecodeError> {
        // the packet structure is
        // hash || signature || packet-type || packet-data
        let hash_len = 32;
        let signature_len = 65;
        let signature_bytes = &encoded_packet[hash_len..hash_len + signature_len];
        let packet_type = encoded_packet[hash_len + signature_len];
        let encoded_msg = &encoded_packet[hash_len + signature_len + 1..];

        // TODO: verify hash and signature
        let hash = H256::from_slice(&encoded_packet[..hash_len]);
        let signature = H520::from_slice(signature_bytes);
        let message = Message::decode_with_type(packet_type, encoded_msg)?;

        Ok(Self {
            hash,
            signature,
            message,
        })
    }

    pub fn get_hash(&self) -> H256 {
        self.hash
    }

    pub fn get_message(&self) -> &Message {
        &self.message
    }
}

#[derive(Debug, Eq, PartialEq)]
// NOTE: All messages could have more fields than specified by the spec.
// Those additional fields should be ignored, and the message must be accepted.
// TODO: remove when all variants are used
#[allow(dead_code)]
pub(crate) enum Message {
    /// A ping message. Should be responded to with a Pong message.
    Ping(PingMessage),
    Pong(PongMessage),
    FindNode(FindNodeMessage),
    Neighbors(NeighborsMessage),
    ENRRequest(ENRRequestMessage),
    ENRResponse(ENRResponseMessage),
}

impl Message {
    pub fn encode_with_header(&self, buf: &mut dyn BufMut, node_signer: &SigningKey) {
        let signature_size = 65_usize;
        let mut data: Vec<u8> = Vec::with_capacity(signature_size.next_power_of_two());
        data.resize(signature_size, 0);

        self.encode_with_type(&mut data);

        let digest = keccak_hash::keccak_buffer(&mut &data[signature_size..]).unwrap();

        let (signature, recovery_id) = node_signer
            .sign_prehash_recoverable(&digest.0)
            .expect("failed to sign");
        let b = signature.to_bytes();

        data[..signature_size - 1].copy_from_slice(&b);
        data[signature_size - 1] = recovery_id.to_byte();

        let hash = keccak_hash::keccak_buffer(&mut &data[..]).unwrap();
        buf.put_slice(&hash.0);
        buf.put_slice(&data[..]);
    }

    fn encode_with_type(&self, buf: &mut dyn BufMut) {
        buf.put_u8(self.packet_type());
        match self {
            Message::Ping(msg) => msg.encode(buf),
            Message::Pong(msg) => msg.encode(buf),
            Message::FindNode(msg) => msg.encode(buf),
            Message::ENRRequest(msg) => msg.encode(buf),
            _ => todo!(),
        }
    }

    pub fn decode_with_type(packet_type: u8, msg: &[u8]) -> Result<Message, RLPDecodeError> {
        // NOTE: extra elements inside the message should be ignored, along with extra data
        // after the message.
        match packet_type {
            0x01 => {
                let (ping, _rest) = PingMessage::decode_unfinished(msg)?;
                Ok(Message::Ping(ping))
            }
            0x02 => {
                let (pong, _rest) = PongMessage::decode_unfinished(msg)?;
                Ok(Message::Pong(pong))
            }
            0x03 => {
                let (find_node_msg, _rest) = FindNodeMessage::decode_unfinished(msg)?;
                Ok(Message::FindNode(find_node_msg))
            }
            0x04 => {
                let (neighbors_msg, _rest) = NeighborsMessage::decode_unfinished(msg)?;
                Ok(Message::Neighbors(neighbors_msg))
            }
            0x05 => {
                let (enr_request_msg, _rest) = ENRRequestMessage::decode_unfinished(msg)?;
                Ok(Message::ENRRequest(enr_request_msg))
            }
            0x06 => {
                let (enr_response_msg, _rest) = ENRResponseMessage::decode_unfinished(msg)?;
                Ok(Message::ENRResponse(enr_response_msg))
            }
            _ => Err(RLPDecodeError::MalformedData),
        }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Endpoint {
    pub ip: IpAddr,
    pub udp_port: u16,
    pub tcp_port: u16,
}

impl RLPEncode for Endpoint {
    fn encode(&self, buf: &mut dyn BufMut) {
        structs::Encoder::new(buf)
            .encode_field(&self.ip)
            .encode_field(&self.udp_port)
            .encode_field(&self.tcp_port)
            .finish();
    }
}

impl RLPDecode for Endpoint {
    fn decode_unfinished(rlp: &[u8]) -> Result<(Self, &[u8]), RLPDecodeError> {
        let decoder = Decoder::new(rlp)?;
        let (ip, decoder) = decoder.decode_field("ip")?;
        let (udp_port, decoder) = decoder.decode_field("udp_port")?;
        let (tcp_port, decoder) = decoder.decode_field("tcp_port")?;
        let remaining = decoder.finish()?;
        let endpoint = Endpoint {
            ip,
            udp_port,
            tcp_port,
        };
        Ok((endpoint, remaining))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PingMessage {
    /// The Ping message version. Should be set to 4, but mustn't be enforced.
    version: u8,
    /// The endpoint of the sender.
    pub from: Endpoint,
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

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct FindNodeMessage {
    /// The target is a 64-byte secp256k1 public key.
    target: H512,
    /// The expiration time of the message. If the message is older than this time,
    /// it shouldn't be responded to.
    expiration: u64,
}

impl FindNodeMessage {
    #[allow(unused)]
    pub fn new(target: H512, expiration: u64) -> Self {
        Self { target, expiration }
    }
}

impl RLPEncode for FindNodeMessage {
    fn encode(&self, buf: &mut dyn BufMut) {
        structs::Encoder::new(buf)
            .encode_field(&self.target)
            .encode_field(&self.expiration)
            .finish();
    }
}

impl RLPDecode for FindNodeMessage {
    fn decode_unfinished(rlp: &[u8]) -> Result<(Self, &[u8]), RLPDecodeError> {
        let decoder = Decoder::new(rlp)?;
        let (target, decoder) = decoder.decode_field("target")?;
        let (expiration, decoder) = decoder.decode_field("expiration")?;
        let remaining = decoder.finish_unchecked();
        let msg = FindNodeMessage { target, expiration };
        Ok((msg, remaining))
    }
}

impl RLPDecode for PingMessage {
    fn decode_unfinished(rlp: &[u8]) -> Result<(Self, &[u8]), RLPDecodeError> {
        let decoder = Decoder::new(rlp)?;
        let (version, decoder): (u8, Decoder) = decoder.decode_field("version")?;
        let (from, decoder) = decoder.decode_field("from")?;
        let (to, decoder) = decoder.decode_field("to")?;
        let (expiration, decoder) = decoder.decode_field("expiration")?;
        let (enr_seq, decoder) = decoder.decode_optional_field();

        let ping = PingMessage {
            version,
            from,
            to,
            expiration,
            enr_seq,
        };
        // NOTE: as per the spec, any additional elements should be ignored.
        let remaining = decoder.finish_unchecked();
        Ok((ping, remaining))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PongMessage {
    /// The endpoint of the receiver.
    to: Endpoint,
    /// The hash of the corresponding ping packet.
    ping_hash: H256,
    /// The expiration time of the message. If the message is older than this time,
    /// it shouldn't be responded to.
    expiration: u64,
    /// The ENR sequence number of the sender. This field is optional.
    enr_seq: Option<u64>,
}

impl PongMessage {
    #[allow(unused)]
    pub fn new(to: Endpoint, ping_hash: H256, expiration: u64) -> Self {
        Self {
            to,
            ping_hash,
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

impl RLPEncode for PongMessage {
    fn encode(&self, buf: &mut dyn BufMut) {
        Encoder::new(buf)
            .encode_field(&self.to)
            .encode_field(&self.ping_hash)
            .encode_field(&self.expiration)
            .encode_optional_field(&self.enr_seq)
            .finish();
    }
}

impl RLPDecode for PongMessage {
    fn decode_unfinished(rlp: &[u8]) -> Result<(Self, &[u8]), RLPDecodeError> {
        let decoder = Decoder::new(rlp)?;
        let (to, decoder) = decoder.decode_field("to")?;
        let (ping_hash, decoder) = decoder.decode_field("ping_hash")?;
        let (expiration, decoder) = decoder.decode_field("expiration")?;
        let (enr_seq, decoder) = decoder.decode_optional_field();

        let pong = PongMessage {
            to,
            ping_hash,
            expiration,
            enr_seq,
        };
        // NOTE: as per the spec, any additional elements should be ignored.
        let remaining = decoder.finish_unchecked();
        Ok((pong, remaining))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NeighborsMessage {
    // nodes is the list of neighbors
    pub nodes: Vec<Node>,
    pub expiration: u64,
}

impl NeighborsMessage {
    pub fn new(nodes: Vec<Node>, expiration: u64) -> Self {
        Self { nodes, expiration }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Node {
    pub ip: IpAddr,
    pub udp_port: u16,
    pub tcp_port: u16,
    pub node_id: H512,
}

impl RLPDecode for NeighborsMessage {
    fn decode_unfinished(rlp: &[u8]) -> Result<(Self, &[u8]), RLPDecodeError> {
        let decoder = Decoder::new(rlp)?;
        let (nodes, decoder) = decoder.decode_field("nodes")?;
        let (expiration, decoder) = decoder.decode_field("expiration")?;
        let remaining = decoder.finish_unchecked();

        let neighbors = NeighborsMessage::new(nodes, expiration);
        Ok((neighbors, remaining))
    }
}

impl RLPDecode for Node {
    fn decode_unfinished(rlp: &[u8]) -> Result<(Self, &[u8]), RLPDecodeError> {
        let decoder = Decoder::new(rlp)?;
        let (ip, decoder) = decoder.decode_field("ip")?;
        let (udp_port, decoder) = decoder.decode_field("upd_port")?;
        let (tcp_port, decoder) = decoder.decode_field("tcp_port")?;
        let (node_id, decoder) = decoder.decode_field("node_id")?;
        let remaining = decoder.finish_unchecked();

        let node = Node {
            ip,
            udp_port,
            tcp_port,
            node_id,
        };
        Ok((node, remaining))
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ENRResponseMessage {
    request_hash: H256,
    node_record: NodeRecord,
}

#[derive(Debug, PartialEq, Eq)]
pub struct NodeRecord {
    signature: H512,
    seq: u64,
    id: Option<String>,
    secp256k1: Option<H264>,
    ip: Option<Ipv4Addr>,
    tcp: Option<u16>,
    udp: Option<u16>,
    ip6: Option<Ipv6Addr>,
    tcp6: Option<u16>,
    udp6: Option<u16>,
}

impl RLPDecode for ENRResponseMessage {
    fn decode_unfinished(rlp: &[u8]) -> Result<(Self, &[u8]), RLPDecodeError> {
        let decoder = Decoder::new(rlp)?;
        let (request_hash, decoder) = decoder.decode_field("request_hash")?;
        let (node_record, decoder) = decoder.decode_field("node_record")?;
        let remaining = decoder.finish_unchecked();
        let response = ENRResponseMessage {
            request_hash,
            node_record,
        };
        Ok((response, remaining))
    }
}

impl RLPDecode for NodeRecord {
    fn decode_unfinished(rlp: &[u8]) -> Result<(Self, &[u8]), RLPDecodeError> {
        let decoder = Decoder::new(rlp)?;
        let (signature, decoder) = decoder.decode_field("signature")?;
        let (seq, decoder) = decoder.decode_field("seq")?;
        let node_record = NodeRecord {
            signature,
            seq,
            id: None,
            secp256k1: None,
            ip: None,
            tcp: None,
            udp: None,
            ip6: None,
            tcp6: None,
            udp6: None,
        };
        let (node_record, decoder) = decode_node_record_optional_fields(node_record, decoder);
        let remaining = decoder.finish_unchecked();
        Ok((node_record, remaining))
    }
}

/// The NodeRecord optional fields are encoded as key/value pairs, according to the documentation
/// <https://github.com/ethereum/devp2p/blob/master/enr.md#record-structure>
/// This function decodes each pair and set the values to the corresponding fields of the NodeRecord struct.
fn decode_node_record_optional_fields(
    mut node_record: NodeRecord,
    decoder: Decoder,
) -> (NodeRecord, Decoder) {
    let (key, decoder): (Option<String>, Decoder) = decoder.decode_optional_field();
    if let Some(k) = key {
        match k.as_str() {
            "id" => {
                let (id, decoder) = decoder.decode_optional_field();
                node_record.id = id;
                decode_node_record_optional_fields(node_record, decoder)
            }
            "secp256k1" => {
                let (secp256k1, decoder) = decoder.decode_optional_field();
                node_record.secp256k1 = secp256k1;
                decode_node_record_optional_fields(node_record, decoder)
            }
            "ip" => {
                let (ip, decoder) = decoder.decode_optional_field();
                node_record.ip = ip;
                decode_node_record_optional_fields(node_record, decoder)
            }
            "tcp" => {
                let (tcp, decoder) = decoder.decode_optional_field();
                node_record.tcp = tcp;
                if node_record.tcp6.is_none() {
                    node_record.tcp6 = tcp;
                }
                decode_node_record_optional_fields(node_record, decoder)
            }
            "udp" => {
                let (udp, decoder) = decoder.decode_optional_field();
                node_record.udp = udp;
                if node_record.udp6.is_none() {
                    node_record.udp6 = udp;
                }
                decode_node_record_optional_fields(node_record, decoder)
            }
            "ip6" => {
                let (ip6, decoder) = decoder.decode_optional_field();
                node_record.ip6 = ip6;
                decode_node_record_optional_fields(node_record, decoder)
            }
            "tcp6" => {
                let (tcp6, decoder) = decoder.decode_optional_field();
                node_record.tcp6 = tcp6;
                decode_node_record_optional_fields(node_record, decoder)
            }
            "udp6" => {
                let (udp6, decoder) = decoder.decode_optional_field();
                node_record.udp6 = udp6;
                decode_node_record_optional_fields(node_record, decoder)
            }
            _ => {
                // ignore the field
                let (_field, decoder): (Option<Vec<u8>>, Decoder) = decoder.decode_optional_field();
                decode_node_record_optional_fields(node_record, decoder)
            }
        }
    } else {
        (node_record, decoder)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ENRRequestMessage {
    expiration: u64,
}

impl RLPDecode for ENRRequestMessage {
    fn decode_unfinished(rlp: &[u8]) -> Result<(Self, &[u8]), RLPDecodeError> {
        let decoder = Decoder::new(rlp)?;
        let (expiration, decoder) = decoder.decode_field("expiration")?;
        let remaining = decoder.finish_unchecked();
        let enr_request = ENRRequestMessage { expiration };
        Ok((enr_request, remaining))
    }
}

impl RLPEncode for ENRRequestMessage {
    fn encode(&self, buf: &mut dyn BufMut) {
        structs::Encoder::new(buf)
            .encode_field(&self.expiration)
            .finish();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use keccak_hash::H256;
    use std::num::ParseIntError;
    use std::{fmt::Write, str::FromStr};

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

        msg.encode_with_header(&mut buf, &signer);
        let result = to_hex(&buf);
        let hash = "d2821841963050aa505c00d8e4fd2d016f95eff739b784e0e26587a58226738e";
        let signature = "8a73f13d613c0ba5148787bb52fd04eb984c3dae486bac19433adf658d29bbb352f3acf2d55f2bdae3afff5298723114581e3f34c37815b32b9195a3326dd68700";
        let pkt_type = "01";
        let encoded_message = "dd04cb840102030482064d8218dbc984ffff0205820bf780850400e78bba";
        let expected = [hash, signature, pkt_type, encoded_message].concat();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_encode_pong_message_with_enr_seq() {
        let to = Endpoint {
            ip: IpAddr::from_str("190.191.188.57").unwrap(),
            udp_port: 30303,
            tcp_port: 0,
        };
        let expiration: u64 = 1719507696;
        let ping_hash: H256 =
            H256::from_str("3e1bf98f025f98d54ed2f61bbef63b6b46f50e12d7b937d6bdea19afd640be23")
                .unwrap();
        let enr_seq = 1704896740573;
        let msg = Message::Pong(PongMessage::new(to, ping_hash, expiration).with_enr_seq(enr_seq));

        let key_bytes =
            H256::from_str("577d8278cc7748fad214b5378669b420f8221afb45ce930b7f22da49cbc545f3")
                .unwrap();
        let signer = SigningKey::from_slice(key_bytes.as_bytes()).unwrap();

        let mut buf = Vec::new();

        msg.encode_with_header(&mut buf, &signer);
        let result = to_hex(&buf);
        let hash = "9657e4e2db33b51cbbeb503bd195efcf081d6a83befbb42b4be95d0f7bf27ffe";
        let signature = "b1a91caa6105b941d3ecce052dcfea5e4f4290c9e6a89ff72707a8b5116ee87a1ea3fa0086990cd862a8a2347f346f1b118122a28bf2ed2ca371d2c493a86bde01";
        let pkt_type = "02";
        let msg = "f7c984bebfbc3982765f80a03e1bf98f025f98d54ed2f61bbef63b6b46f50e12d7b937d6bdea19afd640be2384667d9af086018cf3c3bcdd";
        let expected = [hash, signature, pkt_type, msg].concat();

        assert_eq!(result, expected);
    }

    #[test]
    fn test_encode_pong_message() {
        let to = Endpoint {
            ip: IpAddr::from_str("190.191.188.57").unwrap(),
            udp_port: 30303,
            tcp_port: 0,
        };
        let expiration: u64 = 1719507696;
        let ping_hash: H256 =
            H256::from_str("3e1bf98f025f98d54ed2f61bbef63b6b46f50e12d7b937d6bdea19afd640be23")
                .unwrap();
        let msg = Message::Pong(PongMessage::new(to, ping_hash, expiration));
        let key_bytes =
            H256::from_str("577d8278cc7748fad214b5378669b420f8221afb45ce930b7f22da49cbc545f3")
                .unwrap();
        let signer = SigningKey::from_slice(key_bytes.as_bytes()).unwrap();

        let mut buf = Vec::new();

        msg.encode_with_header(&mut buf, &signer);
        let result = to_hex(&buf);
        let hash = "58a1d0ea66afd9617c198b60a7417637ae27b847b004dbebc1e29d4067327e35";
        let signature = "e1988832d7d7b73925ec584ff818ff3a7bffe1a84fe3835923c3ab17af40071f7c9263176203c80c6ed77f0586479b78884e9e47fdb3287d2aafa92348e5c16700";
        let pkt_type = "02";
        let msg = "f0c984bebfbc3982765f80a03e1bf98f025f98d54ed2f61bbef63b6b46f50e12d7b937d6bdea19afd640be2384667d9af0";
        let expected = [hash, signature, pkt_type, msg].concat();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_encode_find_node_message() {
        let target: H512 = H512::from_str("d860a01f9722d78051619d1e2351aba3f43f943f6f00718d1b9baa4101932a1f5011f16bb2b1bb35db20d6fe28fa0bf09636d26a87d31de9ec6203eeedb1f666").unwrap();
        let expiration: u64 = 17195043770;

        let msg = Message::FindNode(FindNodeMessage::new(target, expiration));

        let key_bytes =
            H256::from_str("577d8278cc7748fad214b5378669b420f8221afb45ce930b7f22da49cbc545f3")
                .unwrap();
        let signer = SigningKey::from_slice(key_bytes.as_bytes()).unwrap();

        let mut buf = Vec::new();

        msg.encode_with_header(&mut buf, &signer);
        let result = to_hex(&buf);
        let hash = "23770430fc208bdc78bc77052bf7ec2e928b38c13c085b87491c15ebebb2050f";
        let signature = "7c98bb4759569117031a9fbbeb00314d018eba55135c65ee98dbf6871aaebe61225f36b36e4f60da5b5d6c917e3589dd235acfacc6de4dade116c4bb851b884b01";
        let pkt_type = "03";
        let encoded_message = "f848b840d860a01f9722d78051619d1e2351aba3f43f943f6f00718d1b9baa4101932a1f5011f16bb2b1bb35db20d6fe28fa0bf09636d26a87d31de9ec6203eeedb1f666850400e78bba";
        let expected = [hash, signature, pkt_type, encoded_message].concat();

        assert_eq!(result, expected);
    }

    #[test]
    fn test_encode_enr_request_message() {
        let expiration: u64 = 17195043770;
        let msg = Message::ENRRequest(ENRRequestMessage { expiration });
        let key_bytes =
            H256::from_str("577d8278cc7748fad214b5378669b420f8221afb45ce930b7f22da49cbc545f3")
                .unwrap();
        let signer = SigningKey::from_slice(key_bytes.as_bytes()).unwrap();
        let mut buf = Vec::new();
        msg.encode_with_header(&mut buf, &signer);
        let result = to_hex(&buf);
        let hash = "ddb4faf81ed7bee047e42088a0efd01650c2191988c08c71dd10635573bee31f";
        let signature = "ec86b35edf60470d81e9796bc4fad68c1d187266492662d91f56b7e42ed46b9317444a72172f13aa91af41ca7a4fec49d5619de9abc0be6c79da0d92bc1c9f3201";
        let pkt_type = "05";
        let encoded_message = "c6850400e78bba";
        let expected = [hash, signature, pkt_type, encoded_message].concat();

        assert_eq!(result, expected);
    }

    #[test]
    fn test_decode_pong_message_with_enr_seq() {
        let hash = "2e1fc2a02ad95a1742f6dd41fb7cbff1e08548ba87f63a72221e44026ab1c347";
        let signature = "34f486e4e92f2fdf592912aa77ad51db532dd7f9b426092384c9c2e9919414fd480d57f4f3b2b1964ed6eb1c94b1e4b9f6bfe9b44b1d1ac3d94c38c4cce915bc01";
        let pkt_type = "02";
        let msg = "f7c984bebfbc3982765f80a03e1bf98f025f98d54ed2f61bbef63b6b46f50e12d7b937d6bdea19afd640be2384667d9af086018cf3c3bcdd";
        let encoded_packet = [hash, signature, pkt_type, msg].concat();

        let decoded_packet = Packet::decode(&decode_hex(&encoded_packet).unwrap()).unwrap();
        let decoded_msg = decoded_packet.get_message();
        let to = Endpoint {
            ip: IpAddr::from_str("190.191.188.57").unwrap(),
            udp_port: 30303,
            tcp_port: 0,
        };
        let expiration: u64 = 1719507696;
        let ping_hash: H256 =
            H256::from_str("3e1bf98f025f98d54ed2f61bbef63b6b46f50e12d7b937d6bdea19afd640be23")
                .unwrap();
        let enr_seq = 1704896740573;
        let expected =
            Message::Pong(PongMessage::new(to, ping_hash, expiration).with_enr_seq(enr_seq));
        assert_eq!(decoded_msg, &expected);
    }

    #[test]
    fn test_decode_pong_message() {
        // in this case the pong message does not contain the `enr_seq` field
        let hash = "2e1fc2a02ad95a1742f6dd41fb7cbff1e08548ba87f63a72221e44026ab1c347";
        let signature = "34f486e4e92f2fdf592912aa77ad51db532dd7f9b426092384c9c2e9919414fd480d57f4f3b2b1964ed6eb1c94b1e4b9f6bfe9b44b1d1ac3d94c38c4cce915bc01";
        let pkt_type = "02";
        let msg = "f0c984bebfbc3982765f80a03e1bf98f025f98d54ed2f61bbef63b6b46f50e12d7b937d6bdea19afd640be2384667d9af0";
        let encoded_packet = [hash, signature, pkt_type, msg].concat();

        let decoded_packet = Packet::decode(&decode_hex(&encoded_packet).unwrap()).unwrap();
        let decoded_msg = decoded_packet.get_message();

        let to = Endpoint {
            ip: IpAddr::from_str("190.191.188.57").unwrap(),
            udp_port: 30303,
            tcp_port: 0,
        };
        let expiration: u64 = 1719507696;
        let ping_hash: H256 =
            H256::from_str("3e1bf98f025f98d54ed2f61bbef63b6b46f50e12d7b937d6bdea19afd640be23")
                .unwrap();
        let expected = Message::Pong(PongMessage::new(to, ping_hash, expiration));
        assert_eq!(decoded_msg, &expected);
    }

    #[test]
    fn test_decode_enr_response() {
        let encoded = "f8ada054e4aa2c131aa991ee21c6e72296205ddadc847cbd09e533c70de6b29b1a72ebf88ab840b77741277742abd4d7460c336a48df717130ef9f77e85d039948ca497831a5ea7e05b5a8a5c5979eaa49b3db0e5f98dbbf26343cef2905dd876279f8f7ce2a3e860190360f571a8269648276348269708436c2f50589736563703235366b31a1038729e0c825f3d9cad382555f3e46dcff21af323e89025a0e6312df541f4a9e738375647082765f";
        let decoded = Message::decode_with_type(0x06, &decode_hex(encoded).unwrap()).unwrap();
        let request_hash =
            H256::from_str("54e4aa2c131aa991ee21c6e72296205ddadc847cbd09e533c70de6b29b1a72eb")
                .unwrap();
        let signature = H512::from_str("b77741277742abd4d7460c336a48df717130ef9f77e85d039948ca497831a5ea7e05b5a8a5c5979eaa49b3db0e5f98dbbf26343cef2905dd876279f8f7ce2a3e").unwrap();
        let seq = 0x0190360f571a;
        let id = Some(String::from("v4"));
        let secp256k1 = Some(
            H264::from_str("038729e0c825f3d9cad382555f3e46dcff21af323e89025a0e6312df541f4a9e73")
                .unwrap(),
        );
        let ip = Some(Ipv4Addr::from_str("54.194.245.5").unwrap());
        let udp = Some(30303_u16);
        let node_record = NodeRecord {
            signature,
            seq,
            id,
            secp256k1,
            ip,
            tcp: None,
            udp,
            ip6: None,
            tcp6: None,
            udp6: udp,
        };
        let expected = Message::ENRResponse(ENRResponseMessage {
            request_hash,
            node_record,
        });
        assert_eq!(decoded, expected);
    }

    pub fn decode_hex(s: &str) -> Result<Vec<u8>, ParseIntError> {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
            .collect()
    }

    #[test]
    fn test_decode_ping_message() {
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

        msg.encode_with_header(&mut buf, &signer);
        let decoded_packet = Packet::decode(&buf).unwrap();
        let decoded_msg = decoded_packet.get_message();
        assert_eq!(decoded_msg, &msg);
    }

    #[test]
    fn test_decode_ping_message_with_enr_seq() {
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

        let enr_seq = 1704896740573;
        let msg = Message::Ping(PingMessage::new(from, to, expiration).with_enr_seq(enr_seq));

        let key_bytes =
            H256::from_str("577d8278cc7748fad214b5378669b420f8221afb45ce930b7f22da49cbc545f3")
                .unwrap();
        let signer = SigningKey::from_slice(key_bytes.as_bytes()).unwrap();

        let mut buf = Vec::new();

        msg.encode_with_header(&mut buf, &signer);
        let decoded_packet = Packet::decode(&buf).unwrap();
        let decoded_msg = decoded_packet.get_message();
        assert_eq!(decoded_msg, &msg);
    }

    #[test]
    fn test_decode_find_node_message() {
        let target: H512 = H512::from_str("d860a01f9722d78051619d1e2351aba3f43f943f6f00718d1b9baa4101932a1f5011f16bb2b1bb35db20d6fe28fa0bf09636d26a87d31de9ec6203eeedb1f666").unwrap();
        let expiration: u64 = 17195043770;
        let msg = Message::FindNode(FindNodeMessage::new(target, expiration));

        let key_bytes =
            H256::from_str("577d8278cc7748fad214b5378669b420f8221afb45ce930b7f22da49cbc545f3")
                .unwrap();
        let signer = SigningKey::from_slice(key_bytes.as_bytes()).unwrap();

        let mut buf = Vec::new();

        msg.encode_with_header(&mut buf, &signer);
        let decoded_packet = Packet::decode(&buf).unwrap();
        let decoded_msg = decoded_packet.get_message();
        assert_eq!(decoded_msg, &msg);
    }

    #[test]
    fn test_decode_neighbors_message() {
        let encoded = "f857f84ff84d847f00000182765f82765fb840d860a01f9722d78051619d1e2351aba3f43f943f6f00718d1b9baa4101932a1f5011f16bb2b1bb35db20d6fe28fa0bf09636d26a87d31de9ec6203eeedb1f666850400e78bba";
        let decoded = Message::decode_with_type(0x04, &decode_hex(encoded).unwrap()).unwrap();
        let expiration: u64 = 17195043770;
        let node_id = H512::from_str("d860a01f9722d78051619d1e2351aba3f43f943f6f00718d1b9baa4101932a1f5011f16bb2b1bb35db20d6fe28fa0bf09636d26a87d31de9ec6203eeedb1f666").unwrap();
        let node = Node {
            ip: "127.0.0.1".parse().unwrap(),
            udp_port: 30303,
            tcp_port: 30303,
            node_id,
        };

        let expected = Message::Neighbors(NeighborsMessage::new(vec![node], expiration));
        assert_eq!(decoded, expected);
    }

    #[test]
    fn test_decode_enr_request_message() {
        let encoded = "c6850400e78bba";
        let decoded = Message::decode_with_type(0x05, &decode_hex(encoded).unwrap()).unwrap();
        let expiration = 0x400E78BBA;
        let expected = Message::ENRRequest(ENRRequestMessage { expiration });
        assert_eq!(decoded, expected);
    }

    #[test]
    fn test_decode_endpoint() {
        let endpoint = Endpoint {
            ip: IpAddr::from_str("255.255.2.5").unwrap(),
            udp_port: 3063,
            tcp_port: 0,
        };

        let encoded = {
            let mut buf = vec![];
            endpoint.encode(&mut buf);
            buf
        };
        let decoded = Endpoint::decode(&encoded).expect("Failed decoding Endpoint");
        assert_eq!(endpoint, decoded);
    }
}
