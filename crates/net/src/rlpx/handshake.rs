//! # RLPx Handshake
//!
//! This state diagram shows the RLPx handshake process.
//!
//! ```mermaid
//! flowchart TD
//! Start --> |sends auth| AuthSent
//! Start --> |receives auth| AuthReceived
//! AuthSent --> |receives ack| CompletedHandshake
//! AuthReceived --> |sends ack| CompletedHandshake
//! CompletedHandshake --> |sends and receives Hello| ConnectionCompleted
//! ```

#![allow(unused)]

use super::utils::{id2pubkey, pubkey2id};
use crate::rlpx::utils::{ecdh_xchng, kdf, sha256, sha256_hmac};

use aes::{
    cipher::{BlockEncrypt, KeyInit, KeyIvInit, StreamCipher},
    Aes256Enc,
};
use bytes::{BufMut, Bytes};
use ethereum_rust_core::{
    rlp::{
        decode::RLPDecode,
        encode::RLPEncode,
        error::RLPDecodeError,
        structs::{Decoder, Encoder},
    },
    Signature, H128, H256, H512,
};
use k256::{ecdsa::SigningKey, elliptic_curve::sec1::ToEncodedPoint, PublicKey, SecretKey};
use rand::Rng;
use sha3::{Digest, Keccak256};
use std::pin::pin;
use tokio::io::{AsyncRead, AsyncReadExt};

const SUPPORTED_CAPABILITIES: [(&str, u8); 1] = [("p2p", 5)];

// TODO: check if these are the same
type Aes128Ctr64BE = ctr::Ctr64BE<aes::Aes128>;
type Aes256Ctr64BE = ctr::Ctr64BE<aes::Aes256>;

// TODO: move to connection.rs
// TODO: make state diagram
/// Fully working RLPx connection.
pub(crate) struct RLPxConnection {
    state: RLPxState,
    // ...capabilities information
}

// TODO: move to connection.rs
/// RLPx connection which is pending the receive of a Hello message.
pub(crate) struct RLPxConnectionPending {
    // TODO: make private
    pub state: RLPxState,
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

        let mut ingress_aes = &mut state.ingress_aes;
        let mut ingress_mac = &mut state.ingress_mac;

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

        //   check MAC
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

        assert_eq!(msg_id, 0);

        // decode hello message: [protocolVersion: P, clientId: B, capabilities, listenPort: P, nodeId: B_64, ...]
        let decoder = Decoder::new(msg_data).unwrap();
        let (protocol_version, decoder): (u64, _) =
            decoder.decode_field("protocolVersion").unwrap();
        let (client_id, decoder): (String, _) = decoder.decode_field("clientId").unwrap();

        // [[cap1, capVersion1], [cap2, capVersion2], ...]
        let (capabilities, decoder): (Vec<(Bytes, u64)>, _) =
            decoder.decode_field("capabilities").unwrap();

        // This field should be ignored
        let (_listen_port, decoder): (u16, _) = decoder.decode_field("listenPort").unwrap();
        let (node_id, decoder): (H512, _) = decoder.decode_field("nodeId").unwrap();

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
    // TODO: make private
    pub aes_key: H256,
    pub mac_key: H256,
    pub ingress_mac: Keccak256,
    pub egress_mac: Keccak256,
    pub ingress_aes: Aes256Ctr64BE,
    pub egress_aes: Aes256Ctr64BE,
}

/// RLPx local client for initiating or accepting connections.
/// Use [`RLPxLocalClient::encode_auth_message`] to initiate a connection,
/// or [`RLPxLocalClient::decode_auth_message_and_encode_ack`] to accept a connection.
#[derive(Debug)]
pub(crate) struct RLPxLocalClient {
    nonce: H256,
    ephemeral_key: SecretKey,
    auth_message: Option<Vec<u8>>,
}

impl RLPxLocalClient {
    pub fn new(nonce: H256, ephemeral_key: SecretKey) -> Self {
        Self {
            nonce,
            ephemeral_key,
            auth_message: None,
        }
    }

    pub fn random() -> Self {
        let mut rng = rand::thread_rng();
        Self::new(H256::random_using(&mut rng), SecretKey::random(&mut rng))
    }

    pub fn encode_auth_message(
        &mut self,
        static_key: &SecretKey,
        remote_static_pubkey: &PublicKey,
        buf: &mut dyn BufMut,
    ) {
        const SIGNATURE_SIZE: usize = 65;
        const IV_SIZE: usize = 16;
        const MAC_FOOTER_SIZE: usize = 32;

        let mut rng = rand::thread_rng();
        let node_id = pubkey2id(&static_key.public_key());

        // Derive a shared secret from the static keys.
        let static_shared_secret = ecdh_xchng(static_key, remote_static_pubkey);

        // Create the signature included in the message.
        let signature = self.sign_shared_secret(static_shared_secret.into());

        // Compose the auth message.
        let auth = AuthMessage::new(signature, node_id, self.nonce);

        // RLP-encode the message.
        let mut encoded_auth_msg = auth.encode_to_vec();

        // Pad with random amount of data. the amount needs to be at least 100 bytes to make
        // the message distinguishable from pre-EIP-8 handshakes.
        let padding_length = rng.gen_range(100..=300);
        encoded_auth_msg.resize(encoded_auth_msg.len() + padding_length, 0);

        // Precompute the size of the message. This is needed for computing the MAC.
        let ecies_overhead = SIGNATURE_SIZE + IV_SIZE + MAC_FOOTER_SIZE;
        let auth_size: u16 = (encoded_auth_msg.len() + ecies_overhead)
            .try_into()
            .unwrap();
        let auth_size_bytes = auth_size.to_be_bytes();

        // Generate a keypair just for this message.
        let message_secret_key = SecretKey::random(&mut rng);

        // Derive a shared secret for this message.
        let message_secret = ecdh_xchng(&message_secret_key, remote_static_pubkey);

        // Derive the AES and MAC keys from the message secret.
        let mut secret_keys = [0; 32];
        kdf(&message_secret, &mut secret_keys);
        let aes_key = &secret_keys[..16];
        let mac_key = sha256(&secret_keys[16..]);

        // Use the AES secret to encrypt the auth message.
        let iv = H128::random_using(&mut rng);
        let mut aes_cipher = Aes128Ctr64BE::new_from_slices(aes_key, &iv.0).unwrap();
        aes_cipher
            .try_apply_keystream(&mut encoded_auth_msg)
            .unwrap();
        let encrypted_auth_msg = encoded_auth_msg;

        // Use the MAC secret to compute the MAC.
        let r_public_key = message_secret_key.public_key().to_encoded_point(false);
        let mac_footer = sha256_hmac(&mac_key, &[&iv.0, &encrypted_auth_msg], &auth_size_bytes);

        // Save the Auth message for the egress-mac initialization
        let auth_message = [
            &auth_size_bytes,
            r_public_key.as_bytes(),
            &iv.0,
            &encrypted_auth_msg,
            &mac_footer,
        ]
        .concat();
        self.auth_message = Some(auth_message);

        // Write everything into the buffer.
        buf.put_slice(&auth_size_bytes);
        buf.put_slice(r_public_key.as_bytes());
        buf.put_slice(&iv.0);
        buf.put_slice(&encrypted_auth_msg);
        buf.put_slice(&mac_footer);
    }

    fn sign_shared_secret(&self, shared_secret: H256) -> Signature {
        let signature_prehash = shared_secret ^ self.nonce;
        let (signature, rid) = SigningKey::from(&self.ephemeral_key)
            .sign_prehash_recoverable(&signature_prehash.0)
            .unwrap();
        let mut signature_bytes = [0; 65];
        signature_bytes[..64].copy_from_slice(signature.to_bytes().as_slice());
        signature_bytes[64] = rid.to_byte();
        signature_bytes.into()
    }

    /// Decodes an Ack message, completing a handshake.
    /// Consumes `self` and returns an [`RLPxConnectionPending`]
    pub fn decode_ack_message(
        self,
        static_key: &SecretKey,
        msg: &[u8],
        auth_data: [u8; 2],
    ) -> RLPxConnectionPending {
        // TODO: return errors instead of panicking
        let sent_auth = self.auth_message.is_some();
        assert!(sent_auth, "received Ack without having sent Auth");
        assert!(msg.len() > 65 + 16 + 32, "message is too short");

        // Split the message into its components. General layout is:
        // public-key (65) || iv (16) || ciphertext || mac (32)
        let (pk, rest) = msg.split_at(65);
        let (iv, rest) = rest.split_at(16);
        let (c, d) = rest.split_at(rest.len() - 32);

        // Derive the message shared secret.
        let shared_secret = ecdh_xchng(static_key, &PublicKey::from_sec1_bytes(pk).unwrap());

        // Derive the AES and MAC keys from the message shared secret.
        let mut buf = [0; 32];
        kdf(&shared_secret, &mut buf);
        let aes_key = &buf[..16];
        let mac_key = sha256(&buf[16..]);

        // Verify the MAC.
        let expected_d = sha256_hmac(&mac_key, &[iv, c], &auth_data);
        assert_eq!(d, expected_d);

        // Decrypt the message with the AES key.
        let mut stream_cipher = Aes128Ctr64BE::new_from_slices(aes_key, iv).unwrap();
        let decoded_payload = {
            let mut decoded = c.to_vec();
            stream_cipher.try_apply_keystream(&mut decoded).unwrap();
            decoded
        };

        // RLP-decode the message.
        let (ack, _padding) = AckMessage::decode_unfinished(&decoded_payload).unwrap();
        let remote_nonce = ack.nonce;

        let (aes_key, mac_key) = self.derive_secrets(&ack);

        // Initiator
        // ingress-mac = keccak256.init((mac-secret ^ initiator-nonce) || ack)
        let ingress_mac = Keccak256::new()
            .chain_update(mac_key ^ self.nonce)
            .chain_update(auth_data)
            .chain_update(msg);

        // TODO: validate this
        assert_eq!(
            ingress_mac.clone().finalize(),
            Keccak256::new()
                .chain_update(mac_key ^ self.nonce)
                .chain_update([&auth_data, msg].concat())
                .finalize()
        );
        // egress-mac = keccak256.init((mac-secret ^ recipient-nonce) || auth)
        let egress_mac = Keccak256::new()
            .chain_update(mac_key ^ ack.nonce)
            .chain_update(self.auth_message.as_ref().unwrap());

        let ingress_aes = <Aes256Ctr64BE as KeyIvInit>::new(&aes_key.0.into(), &[0; 16].into());
        let egress_aes = ingress_aes.clone();

        let state = RLPxState {
            aes_key,
            mac_key,
            ingress_mac,
            egress_mac,
            ingress_aes,
            egress_aes,
        };

        RLPxConnectionPending { state }
    }

    fn derive_secrets(&self, ack: &AckMessage) -> (H256, H256) {
        // TODO: don't panic
        let ephemeral_key_secret =
            ecdh_xchng(&self.ephemeral_key, &ack.get_ephemeral_pubkey().unwrap());

        println!(
            "local_ephemeral_public_key: {:x}",
            pubkey2id(&self.ephemeral_key.public_key())
        );
        // ad57a6adc787e0db24d734e4fc0f9de9add333e17dc09677719ff8ae86b5741725a7f8467a56b4a190808c42f83a833e0543a842a2b6684df847609b009e97e9
        // ba202bf05245e569da208c195957cd0354e33ac429dcd1f596b59ce84977a40e 16718c27ef912689639dc28bc77dcf978803e0017093dc67c79852cdcb79352a
        println!("ephemeral_key_secret: {:x}", H256(ephemeral_key_secret));

        // keccak256(nonce || initiator-nonce)
        let nonce = ack.nonce.0;
        let initiator_nonce = self.nonce.0;
        let hashed_nonces = Keccak256::digest([nonce, initiator_nonce].concat()).into();
        // shared-secret = keccak256(ephemeral-key || keccak256(nonce || initiator-nonce))
        let shared_secret =
            Keccak256::digest([ephemeral_key_secret, hashed_nonces].concat()).into();

        // aes-secret = keccak256(ephemeral-key || shared-secret)
        let aes_key = Keccak256::digest([ephemeral_key_secret, shared_secret].concat()).into();
        // mac-secret = keccak256(ephemeral-key || aes-secret)
        let mac_key = Keccak256::digest([ephemeral_key_secret, aes_key].concat());

        (H256(aes_key), H256(mac_key.into()))
    }

    pub fn decode_auth_message_and_encode_ack() {
        todo!()
    }
}

#[derive(Debug)]
pub(crate) struct AuthMessage {
    /// The signature of the message.
    /// The signed data is `static-shared-secret ^ initiator-nonce`.
    pub signature: Signature,
    /// The node ID of the initiator.
    pub node_id: H512,
    /// The nonce generated by the initiator.
    pub nonce: H256,
    /// The version of RLPx used by the sender.
    /// The current version is 5.
    pub version: u8,
}

impl AuthMessage {
    pub fn new(signature: Signature, node_id: H512, nonce: H256) -> Self {
        Self {
            signature,
            node_id,
            nonce,
            version: 5,
        }
    }
}

impl RLPEncode for AuthMessage {
    fn encode(&self, buf: &mut dyn bytes::BufMut) {
        Encoder::new(buf)
            .encode_field(&self.signature)
            .encode_field(&self.node_id)
            .encode_field(&self.nonce)
            .encode_field(&self.version)
            .finish()
    }
}

impl RLPDecode for AuthMessage {
    // NOTE: discards any extra data in the list after the known fields.
    fn decode_unfinished(rlp: &[u8]) -> Result<(Self, &[u8]), RLPDecodeError> {
        let decoder = Decoder::new(rlp).unwrap();
        let (signature, decoder) = decoder.decode_field("signature").unwrap();
        let (node_id, decoder) = decoder.decode_field("node_id").unwrap();
        let (nonce, decoder) = decoder.decode_field("nonce").unwrap();
        let (version, decoder) = decoder.decode_field("version").unwrap();

        let rest = decoder.finish_unchecked();
        let this = Self {
            signature,
            node_id,
            nonce,
            version,
        };
        Ok((this, rest))
    }
}

#[derive(Debug, Clone)]
pub(crate) struct AckMessage {
    /// The recipient's ephemeral public key.
    pub ephemeral_pubkey: H512,
    /// The nonce generated by the recipient.
    pub nonce: H256,
    /// The version of RLPx used by the recipient.
    /// The current version is 5.
    pub version: u8,
}

impl AckMessage {
    pub fn get_ephemeral_pubkey(&self) -> Option<PublicKey> {
        id2pubkey(self.ephemeral_pubkey)
    }
}

impl RLPEncode for AckMessage {
    fn encode(&self, buf: &mut dyn bytes::BufMut) {
        Encoder::new(buf)
            .encode_field(&self.ephemeral_pubkey)
            .encode_field(&self.nonce)
            .encode_field(&self.version)
            .finish()
    }
}

impl RLPDecode for AckMessage {
    // NOTE: discards any extra data in the list after the known fields.
    fn decode_unfinished(rlp: &[u8]) -> Result<(Self, &[u8]), RLPDecodeError> {
        let decoder = Decoder::new(rlp).unwrap();
        let (ephemeral_pubkey, decoder) = decoder.decode_field("ephemeral_pubkey").unwrap();
        let (nonce, decoder) = decoder.decode_field("nonce").unwrap();
        let (version, decoder) = decoder.decode_field("version").unwrap();

        let rest = decoder.finish_unchecked();
        let this = Self {
            ephemeral_pubkey,
            nonce,
            version,
        };
        Ok((this, rest))
    }
}

#[cfg(test)]
mod tests {
    use crate::rlpx::handshake::RLPxLocalClient;
    use hex_literal::hex;
    use k256::SecretKey;

    #[test]
    fn test_ack_decoding() {
        // This is the Ack₂ message from EIP-8.
        let mut msg = hex!("01ea0451958701280a56482929d3b0757da8f7fbe5286784beead59d95089c217c9b917788989470b0e330cc6e4fb383c0340ed85fab836ec9fb8a49672712aeabbdfd1e837c1ff4cace34311cd7f4de05d59279e3524ab26ef753a0095637ac88f2b499b9914b5f64e143eae548a1066e14cd2f4bd7f814c4652f11b254f8a2d0191e2f5546fae6055694aed14d906df79ad3b407d94692694e259191cde171ad542fc588fa2b7333313d82a9f887332f1dfc36cea03f831cb9a23fea05b33deb999e85489e645f6aab1872475d488d7bd6c7c120caf28dbfc5d6833888155ed69d34dbdc39c1f299be1057810f34fbe754d021bfca14dc989753d61c413d261934e1a9c67ee060a25eefb54e81a4d14baff922180c395d3f998d70f46f6b58306f969627ae364497e73fc27f6d17ae45a413d322cb8814276be6ddd13b885b201b943213656cde498fa0e9ddc8e0b8f8a53824fbd82254f3e2c17e8eaea009c38b4aa0a3f306e8797db43c25d68e86f262e564086f59a2fc60511c42abfb3057c247a8a8fe4fb3ccbadde17514b7ac8000cdb6a912778426260c47f38919a91f25f4b5ffb455d6aaaf150f7e5529c100ce62d6d92826a71778d809bdf60232ae21ce8a437eca8223f45ac37f6487452ce626f549b3b5fdee26afd2072e4bc75833c2464c805246155289f4");

        let static_key = hex!("49a7b37aa6f6645917e7b807e9d1c00d4fa71f18343b0d4122a4d2df64dd6fee");
        let nonce = hex!("7e968bba13b6c50e2c4cd7f241cc0d64d1ac25c7f5952df231ac6a2bda8ee5d6");
        let ephemeral_key =
            hex!("869d6ecf5211f1cc60418a13b9d870b22959d0c16f02bec714c960dd2298a32d");

        let mut client =
            RLPxLocalClient::new(nonce.into(), SecretKey::from_slice(&ephemeral_key).unwrap());

        assert_eq!(&client.ephemeral_key.to_bytes()[..], &ephemeral_key[..]);
        assert_eq!(client.nonce.0, nonce);

        let auth_data = msg[..2].try_into().unwrap();

        client.auth_message = Some(vec![]);

        let conn = client.decode_ack_message(
            &SecretKey::from_slice(&static_key).unwrap(),
            &msg[2..],
            auth_data,
        );

        let state = conn.state;

        let expected_aes_secret =
            hex!("80e8632c05fed6fc2a13b0f8d31a3cf645366239170ea067065aba8e28bac487");
        let expected_mac_secret =
            hex!("2ea74ec5dae199227dff1af715362700e989d889d7a493cb0639691efb8e5f98");

        assert_eq!(state.aes_key.0, expected_aes_secret);
        assert_eq!(state.mac_key.0, expected_mac_secret);
    }
}
