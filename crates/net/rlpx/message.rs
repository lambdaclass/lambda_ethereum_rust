use bytes::BufMut;
use ethereum_rust_core::rlp::error::RLPDecodeError;

use super::p2p::{DisconnectMessage, HelloMessage, PingMessage, PongMessage, RLPxMessage as _};

pub(crate) enum Message {
    Hello(HelloMessage),
    Disconnect(DisconnectMessage),
    Ping(PingMessage),
    Pong(PongMessage),
}

impl Message {
    pub fn decode(msg_id: u8, msg_data: &[u8]) -> Result<Message, RLPDecodeError> {
        match msg_id {
            0x00 => Ok(Message::Hello(HelloMessage::decode(msg_data)?)),
            0x01 => Ok(Message::Disconnect(DisconnectMessage::decode(msg_data)?)),
            0x02 => Ok(Message::Ping(PingMessage::decode(msg_data)?)),
            0x03 => Ok(Message::Pong(PongMessage::decode(msg_data)?)),
            0x10 => Ok(Message::Pong(PongMessage::decode(msg_data)?)),
            _ => Err(RLPDecodeError::MalformedData),
        }
    }

    pub fn encode(&self, buf: &mut dyn BufMut) {
        match self {
            Message::Hello(msg) => msg.encode(buf),
            Message::Disconnect(msg) => msg.encode(buf),
            Message::Ping(msg) => msg.encode(buf),
            Message::Pong(msg) => msg.encode(buf),
        }
    }
}
