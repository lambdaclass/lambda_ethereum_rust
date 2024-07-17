use std::pin::pin;

use aes::{
    cipher::{BlockEncrypt, KeyInit, KeyIvInit, StreamCipher},
    Aes256Enc,
};
use bytes::{BufMut, Bytes};
use ethereum_rust_core::{
    rlp::{
        decode::RLPDecode,
        encode::RLPEncode,
        structs::{Decoder, Encoder},
    },
    H128, H256, H512,
};
use k256::PublicKey;
use sha3::{Digest, Keccak256};
use tokio::io::{AsyncRead, AsyncReadExt};
use utils::pubkey2id;

pub mod handshake;
pub mod utils;

const SUPPORTED_CAPABILITIES: [(&str, u8); 1] = [("p2p", 5)];

type Aes256Ctr64BE = ctr::Ctr64BE<aes::Aes256>;

// TODO: move to connection.rs
// TODO: make state diagram
/// Fully working RLPx connection.
pub(crate) struct RLPxConnection {
    #[allow(unused)]
    state: RLPxState,
    // ...capabilities information
}

// TODO: move to connection.rs
/// RLPx connection which is pending the receive of a Hello message.
pub(crate) struct RLPxConnectionPending {
    // TODO: make private
    state: RLPxState,
}

impl RLPxConnectionPending {
    pub fn send_hello(&mut self, node_pk: &PublicKey, buf: &mut dyn BufMut) {
        let egress_aes = &mut self.state.egress_aes;
        let egress_mac = &mut self.state.egress_mac;

        let mac_aes_cipher = Aes256Enc::new_from_slice(&self.state.mac_key.0).unwrap();

        // Generate Hello message
        // [protocolVersion: P, clientId: B, capabilities, listenPort: P, nodeKey: B_64, ...]
        let msg_id = 1_u8;
        let protocol_version = 5_u8;
        let client_id = "Ethereum(++)/1.0.0";
        let capabilities = SUPPORTED_CAPABILITIES.to_vec();
        let listen_port = 0_u8; // This one is ignored
        let node_id = pubkey2id(node_pk);
        let mut frame_data = vec![];
        msg_id.encode(&mut frame_data);
        Encoder::new(&mut frame_data)
            .encode_field(&protocol_version)
            .encode_field(&client_id)
            .encode_field(&capabilities)
            .encode_field(&listen_port)
            .encode_field(&node_id)
            .finish();

        // header = frame-size || header-data || header-padding
        let mut header = Vec::with_capacity(32);
        let frame_size = frame_data.len().to_be_bytes();
        header.extend_from_slice(&frame_size[5..8]);
        // header-data = [capability-id, context-id]  (both always zero)
        let header_data = (0_u8, 0_u8);
        header_data.encode(&mut header);

        header.resize(16, 0);
        egress_aes.apply_keystream(&mut header[..16]);

        let header_mac_seed = {
            let mac_digest: [u8; 16] = egress_mac.clone().finalize()[..16].try_into().unwrap();
            let mut seed = mac_digest.into();
            mac_aes_cipher.encrypt_block(&mut seed);
            H128(seed.into()) ^ H128(header[..16].try_into().unwrap())
        };
        egress_mac.update(header_mac_seed);
        let header_mac = egress_mac.clone().finalize();
        header.extend_from_slice(&header_mac[..16]);

        // Write header
        buf.put_slice(&header);

        // Pad to next multiple of 16
        frame_data.resize(frame_data.len().next_multiple_of(16), 0);
        egress_aes.apply_keystream(&mut frame_data);
        let frame_ciphertext = frame_data;

        // Send frame
        buf.put_slice(&frame_ciphertext);

        // Compute frame-mac
        egress_mac.update(&frame_ciphertext);

        // frame-mac-seed = aes(mac-secret, keccak256.digest(egress-mac)[:16]) ^ keccak256.digest(egress-mac)[:16]
        let frame_mac_seed = {
            let mac_digest: [u8; 16] = egress_mac.clone().finalize()[..16].try_into().unwrap();
            let mut seed = mac_digest.into();
            mac_aes_cipher.encrypt_block(&mut seed);
            (H128(seed.into()) ^ H128(mac_digest)).0
        };
        egress_mac.update(frame_mac_seed);
        let frame_mac = egress_mac.clone().finalize();

        // Send frame-mac
        buf.put_slice(&frame_mac[..16]);
    }

    pub async fn receive_hello<S: AsyncRead>(self, stream: S) -> RLPxConnection {
        let mut stream = pin!(stream);

        let Self { mut state } = self;

        let ingress_aes = &mut state.ingress_aes;
        let ingress_mac = &mut state.ingress_mac;

        let mac_aes_cipher = Aes256Enc::new_from_slice(&state.mac_key.0).unwrap();

        // Receive the hello message's frame header
        let mut frame_header = [0; 32];
        stream.read_exact(&mut frame_header).await.unwrap();
        // Both are padded to the block's size (16 bytes)
        let (header_ciphertext, header_mac) = frame_header.split_at_mut(16);

        // Validate MAC header
        // header-mac-seed = aes(mac-secret, keccak256.digest(egress-mac)[:16]) ^ header-ciphertext
        let header_mac_seed = {
            let mac_digest: [u8; 16] = ingress_mac.clone().finalize()[..16].try_into().unwrap();
            let mut seed = mac_digest.into();
            mac_aes_cipher.encrypt_block(&mut seed);
            (H128(seed.into()) ^ H128(header_ciphertext.try_into().unwrap())).0
        };

        // ingress-mac = keccak256.update(ingress-mac, header-mac-seed)
        ingress_mac.update(header_mac_seed);

        // header-mac = keccak256.digest(egress-mac)[:16]
        let expected_header_mac = H128(ingress_mac.clone().finalize()[..16].try_into().unwrap());

        assert_eq!(header_mac, expected_header_mac.0);

        let header_text = header_ciphertext;
        ingress_aes.apply_keystream(header_text);

        // header-data = [capability-id, context-id]
        // Both are unused, and always zero
        assert_eq!(&header_text[3..6], &(0_u8, 0_u8).encode_to_vec());

        let frame_size: usize =
            u32::from_be_bytes([0, header_text[0], header_text[1], header_text[2]])
                .try_into()
                .unwrap();
        // Receive the hello message
        let padded_size = frame_size.next_multiple_of(16);
        let mut frame_data = vec![0; padded_size + 16];
        stream.read_exact(&mut frame_data).await.unwrap();
        let (frame_ciphertext, frame_mac) = frame_data.split_at_mut(padded_size);

        // check MAC
        ingress_mac.update(&frame_ciphertext);
        let frame_mac_seed = {
            let mac_digest: [u8; 16] = ingress_mac.clone().finalize()[..16].try_into().unwrap();
            let mut seed = mac_digest.into();
            mac_aes_cipher.encrypt_block(&mut seed);
            (H128(seed.into()) ^ H128(mac_digest)).0
        };
        ingress_mac.update(frame_mac_seed);
        let expected_frame_mac: [u8; 16] = ingress_mac.clone().finalize()[..16].try_into().unwrap();

        assert_eq!(frame_mac, expected_frame_mac);

        // decrypt frame
        ingress_aes.apply_keystream(frame_ciphertext);

        let (frame_data, _padding) = frame_ciphertext.split_at(frame_size);

        let (msg_id, msg_data): (u8, _) = RLPDecode::decode_unfinished(frame_data).unwrap();

        // Hello has an ID of 0x00
        assert_eq!(msg_id, 0);

        // decode hello message: [protocolVersion: P, clientId: B, capabilities, listenPort: P, nodeId: B_64, ...]
        let decoder = Decoder::new(msg_data).unwrap();
        let (protocol_version, decoder): (u64, _) =
            decoder.decode_field("protocolVersion").unwrap();

        assert_eq!(protocol_version, 5, "only protocol version 5 is supported");

        let (_client_id, decoder): (String, _) = decoder.decode_field("clientId").unwrap();
        // TODO: store client id for debugging purposes

        // [[cap1, capVersion1], [cap2, capVersion2], ...]
        let (_capabilities, decoder): (Vec<(Bytes, u64)>, _) =
            decoder.decode_field("capabilities").unwrap();
        // TODO: derive shared capabilities for further communication

        // This field should be ignored
        let (_listen_port, decoder): (u16, _) = decoder.decode_field("listenPort").unwrap();

        let (_node_id, decoder): (H512, _) = decoder.decode_field("nodeId").unwrap();
        // TODO: check node id is the one we expect

        // Implementations must ignore any additional list elements
        let _padding = decoder.finish_unchecked();

        RLPxConnection { state }
    }
}

/// The current state of an RLPx connection
#[derive(Clone)]
pub(crate) struct RLPxState {
    // TODO: maybe discard aes_key, since we only need the cipher
    // TODO: maybe precompute some values that are used more than once
    #[allow(unused)]
    aes_key: H256,
    mac_key: H256,
    ingress_mac: Keccak256,
    egress_mac: Keccak256,
    ingress_aes: Aes256Ctr64BE,
    egress_aes: Aes256Ctr64BE,
}

impl RLPxState {
    pub fn new(
        aes_key: H256,
        mac_key: H256,
        local_nonce: H256,
        local_init_message: &[u8],
        remote_nonce: H256,
        remote_init_message: &[u8],
    ) -> Self {
        // egress-mac = keccak256.init((mac-secret ^ remote-nonce) || auth)
        let egress_mac = Keccak256::default()
            .chain_update(mac_key ^ remote_nonce)
            .chain_update(local_init_message);

        // ingress-mac = keccak256.init((mac-secret ^ initiator-nonce) || ack)
        let ingress_mac = Keccak256::default()
            .chain_update(mac_key ^ local_nonce)
            .chain_update(remote_init_message);

        let ingress_aes = <Aes256Ctr64BE as KeyIvInit>::new(&aes_key.0.into(), &[0; 16].into());
        let egress_aes = ingress_aes.clone();

        Self {
            aes_key,
            mac_key,
            ingress_mac,
            egress_mac,
            ingress_aes,
            egress_aes,
        }
    }
}
