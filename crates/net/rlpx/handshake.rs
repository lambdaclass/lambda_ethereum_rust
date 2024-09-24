use crate::rlpx::{
    connection::RLPxState,
    utils::{ecdh_xchng, id2pubkey, kdf, pubkey2id, sha256, sha256_hmac},
};

use aes::cipher::{KeyIvInit, StreamCipher};
use bytes::BufMut;
use ethereum_rust_core::{Signature, H128, H256, H512};
use ethereum_rust_rlp::{
    decode::RLPDecode,
    encode::RLPEncode,
    error::RLPDecodeError,
    structs::{Decoder, Encoder},
};
use k256::{ecdsa::SigningKey, elliptic_curve::sec1::ToEncodedPoint, PublicKey, SecretKey};
use rand::Rng;
use sha3::{Digest, Keccak256};

type Aes128Ctr64BE = ctr::Ctr64BE<aes::Aes128>;

/// RLPx local client for initiating or accepting connections.
/// Use [`RLPxLocalClient::encode_auth_message`] to initiate a connection,
/// or [`RLPxLocalClient::decode_auth_message_and_encode_ack`] to accept a connection.
#[derive(Debug)]
pub(crate) struct RLPxLocalClient {
    pub(crate) nonce: H256,
    pub(crate) ephemeral_key: SecretKey,
    pub(crate) auth_message: Option<Vec<u8>>,
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
    ) -> RLPxState {
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

        let (aes_key, mac_key) = self.derive_secrets(&ack);

        let ack_message = [&auth_data, msg].concat();

        RLPxState::new(
            aes_key,
            mac_key,
            self.nonce,
            self.auth_message.as_ref().unwrap(),
            ack.nonce,
            &ack_message,
        )
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
