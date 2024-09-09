use ethereum_rust_core::{
    rlp::{
        error::RLPDecodeError,
        structs::{Decoder, Encoder},
    },
    H512,
};
use k256::PublicKey;
use snap::raw::Decoder as SnappyDecoder;

use crate::rlpx::utils::id2pubkey;

use super::utils::pubkey2id;

pub(crate) enum Message {
    Hello(Vec<(String, u8)>, PublicKey),
    // TODO
    // Disconnect(),
    Ping(),
    Pong(),
}

impl Message {
    pub fn decode(msg_id: u8, msg_data: &[u8]) -> Result<Message, RLPDecodeError> {
        match msg_id {
            0x00 => {
                // decode hello message: [protocolVersion: P, clientId: B, capabilities, listenPort: P, nodeId: B_64, ...]
                let decoder = Decoder::new(msg_data).unwrap();
                let (protocol_version, decoder): (u64, _) =
                    decoder.decode_field("protocolVersion").unwrap();

                assert_eq!(protocol_version, 5, "only protocol version 5 is supported");

                let (_client_id, decoder): (String, _) = decoder.decode_field("clientId").unwrap();
                // TODO: store client id for debugging purposes

                // [[cap1, capVersion1], [cap2, capVersion2], ...]
                let (capabilities, decoder): (Vec<(String, u8)>, _) =
                    decoder.decode_field("capabilities").unwrap();

                // TODO: derive shared capabilities for further communication

                // This field should be ignored
                let (_listen_port, decoder): (u16, _) = decoder.decode_field("listenPort").unwrap();

                let (node_id, decoder): (H512, _) = decoder.decode_field("nodeId").unwrap();
                // TODO: check node id is the one we expect

                // Implementations must ignore any additional list elements
                let _padding = decoder.finish_unchecked();

                Ok(Message::Hello(capabilities, id2pubkey(node_id).unwrap()))
            }
            0x02 => {
                // decode ping message: data is empty list [] but it is snappy compressed
                let mut snappy_decoder = SnappyDecoder::new();
                // TODO deal with error
                let decompressed_data = snappy_decoder.decompress_vec(msg_data).unwrap();
                let decoder = Decoder::new(&decompressed_data)?;
                let result = decoder.finish_unchecked();
                let empty: &[u8] = &[];
                assert_eq!(result, empty, "Pong msg_data should be &[]");
                Ok(Message::Ping())
            }
            0x03 => {
                // decode pong message: data is empty list [] but it is snappy compressed
                let mut snappy_decoder = SnappyDecoder::new();
                // TODO deal with error
                let decompressed_data = snappy_decoder.decompress_vec(msg_data).unwrap();
                let decoder = Decoder::new(&decompressed_data)?;
                let result = decoder.finish_unchecked();
                let empty: &[u8] = &[];
                assert_eq!(result, empty, "Pong msg_data should be &[]");
                Ok(Message::Pong())
            }
            0x10 => Ok(Message::Ping()),
            _ => Err(RLPDecodeError::MalformedData),
        }
    }

    pub fn msg_id(&self) -> u8 {
        match self {
            Message::Hello(_, _) => 0_u8,
            // Message::Disconnect() => 1_u8,
            Message::Ping() => 2_u8,
            Message::Pong() => 3_u8,
        }
    }

    pub fn msg_data(&self) -> Vec<u8> {
        match self {
            Message::Hello(capabilities, node_pk) => {
                // [protocolVersion: P, clientId: B, capabilities, listenPort: P, nodeKey: B_64, ...]
                let mut msg_data: Vec<u8> = vec![];
                Encoder::new(&mut msg_data)
                    .encode_field(&5_u8) // protocolVersion
                    .encode_field(&"Ethereum(++)/1.0.0") // clientId
                    .encode_field(capabilities) // capabilities
                    .encode_field(&0u8) // listenPort (ignored)
                    .encode_field(&pubkey2id(node_pk)) // nodeKey
                    .finish();
                msg_data
            }
            // Message::Disconnect() => todo!(),
            Message::Ping() => Vec::<u8>::new(), // msg_data is empty for ping
            Message::Pong() => Vec::<u8>::new(), // msg_data is empty for pong
        }
    }

    pub fn is_compressed(&self) -> bool {
        !matches!(self, Message::Hello(_, _))
        // !matches!(self, Message::Hello(_, _) | Message::Disconnect())
    }
}
