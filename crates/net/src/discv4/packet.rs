use super::message::Message;

/// A discv4 packet.
#[derive(Debug)]
pub struct Packet {
    pub message: Message,
    // pub node_id:
    pub hash: [u8; 32],
}
