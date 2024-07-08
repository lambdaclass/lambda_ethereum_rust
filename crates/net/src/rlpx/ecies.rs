use aes::cipher::{KeyIvInit, StreamCipher};
use bytes::BufMut;
use ethereum_rust_core::{
    rlp::{
        decode::RLPDecode,
        encode::RLPEncode,
        error::RLPDecodeError,
        structs::{Decoder, Encoder},
    },
    Signature, H128, H512, H520,
};
use k256::{
    ecdsa::SigningKey,
    elliptic_curve::sec1::{FromEncodedPoint, ToEncodedPoint},
    EncodedPoint, PublicKey, SecretKey,
};
use keccak_hash::H256;
use rand::{thread_rng, Rng};

type Aes128Ctr64BE = ctr::Ctr64BE<aes::Aes128>;

#[derive(Debug)]
pub(crate) struct RLPxConnection {
    pub nonce: H256,
    pub ephemeral_key: SecretKey,
    pub secrets: Option<ConnSecrets>,
}

#[derive(Debug)]
pub(crate) struct ConnSecrets {
    pub remote_nonce: H256,
    pub remote_ephemeral_key: PublicKey,
    pub ephemeral_shared_secret: H256,
    pub aes_key: H256,
    pub mac_key: H256,
}

impl RLPxConnection {
    pub fn random() -> Self {
        let mut rng = thread_rng();
        Self {
            nonce: H256::random_using(&mut rng),
            ephemeral_key: SecretKey::random(&mut rng),
            secrets: None,
        }
    }

    pub fn is_connected(&self) -> bool {
        self.secrets.is_some()
    }

    pub fn encode_auth_message(
        &self,
        static_key: &SecretKey,
        remote_static_pubkey: &PublicKey,
        buf: &mut dyn BufMut,
    ) {
        let shared_secret = ecdh_xchng(static_key, &remote_static_pubkey);
        let node_id = pubkey2id(&static_key.public_key());
        let signature = self.sign_shared_secret(shared_secret.into());

        let auth = AuthMessage::new(signature, node_id, self.nonce);
        let mut rng = rand::thread_rng();
        let padding_length = rng.gen_range(100..=300);

        let mut encoded_auth_msg = vec![];
        auth.encode(&mut encoded_auth_msg);
        encoded_auth_msg.resize(encoded_auth_msg.len() + padding_length, 0);

        let ecies_data_size = 65 + 16 + 32;
        let auth_size: u16 = (encoded_auth_msg.len() + ecies_data_size)
            .try_into()
            .unwrap();
        let auth_size_bytes = auth_size.to_be_bytes();

        let message_secret_key = SecretKey::random(&mut rng);
        let message_secret = ecdh_xchng(&message_secret_key, &remote_static_pubkey);

        let mut secret_keys = [0; 32];
        kdf(&message_secret, &mut secret_keys);
        let aes_key = &secret_keys[..16];
        let mac_key = sha256(&secret_keys[16..]);

        let iv = H128::random_using(&mut rng);
        let mut aes_cipher = Aes128Ctr64BE::new_from_slices(aes_key, &iv.0).unwrap();
        aes_cipher
            .try_apply_keystream(&mut encoded_auth_msg)
            .unwrap();
        let encrypted_auth_msg = encoded_auth_msg;

        let r_public_key = message_secret_key.public_key().to_encoded_point(false);
        let d = sha256_hmac(&mac_key, &[&iv.0, &encrypted_auth_msg], &auth_size_bytes);

        buf.put_slice(&auth_size_bytes);
        buf.put_slice(&r_public_key.as_bytes());
        buf.put_slice(&iv.0);
        buf.put_slice(&encrypted_auth_msg);
        buf.put_slice(&d);
    }

    fn sign_shared_secret(&self, shared_secret: H256) -> H520 {
        let signature_prehash = shared_secret ^ self.nonce;
        let (signature, rid) = SigningKey::from(&self.ephemeral_key)
            .sign_prehash_recoverable(&signature_prehash.0)
            .unwrap();
        let mut signature_bytes = [0; 65];
        signature_bytes[..64].copy_from_slice(signature.to_bytes().as_slice());
        signature_bytes[64] = rid.to_byte();
        H520(signature_bytes)
    }
}

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
        let (signature, decoder) = decoder.decode_field("sig").unwrap();
        let (node_id, decoder) = decoder.decode_field("initiator_pubkey").unwrap();
        let (nonce, decoder) = decoder.decode_field("initiator_nonce").unwrap();
        let (version, decoder) = decoder.decode_field("auth_vsn").unwrap();

        // Garbage used to pad the message. It is used to obfuscate the true message length.
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

fn sha256(data: &[u8]) -> [u8; 32] {
    use k256::sha2::Digest;
    k256::sha2::Sha256::digest(data).try_into().unwrap()
}

fn sha256_hmac(key: &[u8], inputs: &[&[u8]], auth_data: &[u8]) -> [u8; 32] {
    use hmac::Mac;
    use k256::sha2::Sha256;

    let mut hasher = hmac::Hmac::<Sha256>::new_from_slice(key).unwrap();
    for input in inputs {
        hasher.update(input);
    }
    hasher.update(auth_data);
    hasher.finalize().into_bytes().try_into().unwrap()
}

fn ecdh_xchng(secret_key: &SecretKey, public_key: &PublicKey) -> [u8; 32] {
    k256::ecdh::diffie_hellman(secret_key.to_nonzero_scalar(), public_key.as_affine())
        .raw_secret_bytes()[..32]
        .try_into()
        .unwrap()
}

fn kdf(secret: &[u8], output: &mut [u8]) {
    // We don't use the `other_info` field
    concat_kdf::derive_key_into::<k256::sha2::Sha256>(&secret, &[], output).unwrap();
}

/// Computes recipient id from public key.
pub fn pubkey2id(pk: &PublicKey) -> H512 {
    let encoded = pk.to_encoded_point(false);
    let bytes = encoded.as_bytes();
    debug_assert_eq!(bytes[0], 4);
    H512::from_slice(&bytes[1..])
}

/// Computes public key from recipient id.
pub fn id2pubkey(id: H512) -> Option<PublicKey> {
    let point = EncodedPoint::from_untagged_bytes(&id.0.into());
    PublicKey::from_encoded_point(&point).into_option()
}

#[cfg(test)]
mod tests {
    use aes::cipher::{KeyIvInit, StreamCipher};
    use ethereum_rust_core::{
        rlp::{decode::RLPDecode, encode::encode, structs::Decoder},
        H128, H520,
    };
    use hex_literal::hex;
    use k256::{
        ecdsa::{RecoveryId, Signature, SigningKey, VerifyingKey},
        elliptic_curve::sec1::ToEncodedPoint,
        PublicKey, SecretKey,
    };
    use keccak_hash::{keccak, H256};
    use rand::rngs::OsRng;

    use super::*;

    #[test]
    fn ecdh_xchng_smoke_test() {
        use rand::rngs::OsRng;

        let a_sk = SecretKey::random(&mut OsRng);
        let b_sk = SecretKey::random(&mut OsRng);

        let a_sk_b_pk = ecdh_xchng(&a_sk, &b_sk.public_key());
        let b_sk_a_pk = ecdh_xchng(&b_sk, &a_sk.public_key());

        // The shared secrets should be the same.
        // The operation done is:
        //   a_sk * b_pk = a * (b * G) = b * (a * G) = b_sk * a_pk
        assert_eq!(a_sk_b_pk, b_sk_a_pk);
    }

    #[test]
    fn id2pubkey_pubkey2id_smoke_test() {
        use rand::rngs::OsRng;

        let sk = SecretKey::random(&mut OsRng);
        let pk = sk.public_key();
        let id = pubkey2id(&pk);
        let _pk2 = id2pubkey(id).unwrap();
    }

    // TODO: clean this up
    #[test]
    fn test_rlpx_handshake() {
        // Test vector input (these would be random in practice)
        // Static Key A
        let static_key_a_bytes =
            hex!("49a7b37aa6f6645917e7b807e9d1c00d4fa71f18343b0d4122a4d2df64dd6fee");
        let a_static_key = SecretKey::from_slice(&static_key_a_bytes).unwrap();

        // Static Key B
        let static_key_b_bytes =
            hex!("b71c71a67e1177ad4e901695e1b4b9ee17ae16c6668d313eac2f96dbcda3f291");
        let b_static_key = SecretKey::from_slice(&static_key_b_bytes).unwrap();

        // Ephemeral Key A
        let ephemeral_key_a_bytes =
            hex!("869d6ecf5211f1cc60418a13b9d870b22959d0c16f02bec714c960dd2298a32d");

        // Ephemeral Key B
        let ephemeral_key_b_bytes =
            hex!("e238eb8e04fee6511ab04c6dd3c89ce097b11f25d584863ac2b6d5b35b1847e4");
        let b_ephemeral_key = SecretKey::from_slice(&ephemeral_key_b_bytes).unwrap();

        // Nonce A
        let nonce_a_bytes =
            hex!("7e968bba13b6c50e2c4cd7f241cc0d64d1ac25c7f5952df231ac6a2bda8ee5d6");

        // Nonce B
        let nonce_b_bytes =
            hex!("559aead08264d5795d3909718cdd05abd49572e84fe55590eef31a88a08fdffd");

        // The steps that follow were taken from the spec and the vbhattaccmu/rlpx-handshake repo

        // AUTH MESSAGE ENCODING
        // 1. Generate ephemeral key and nonce
        // Random data generated by initiator (here it's given as input).
        let initiator_nonce = H256(nonce_a_bytes);
        let a_ephemeral_key = SecretKey::from_slice(&ephemeral_key_a_bytes).unwrap();

        // 2. Compute shared secret for aes and mac keys
        let b_static_pubkey = b_static_key.public_key();
        let static_shared_secret = ecdh_xchng(&a_static_key, &b_static_pubkey);
        let a_node_id = pubkey2id(&a_static_key.public_key());

        let signature_prehash = H256(static_shared_secret) ^ initiator_nonce;
        let (signature, rid) = SigningKey::from(&a_ephemeral_key)
            .sign_prehash_recoverable(&signature_prehash.0)
            .unwrap();
        let mut recoverable_signature = [0; 65];
        recoverable_signature[..64].copy_from_slice(signature.to_bytes().as_slice());
        recoverable_signature[64] = rid.to_byte();

        let mut auth_msg =
            AuthMessage::new(H520(recoverable_signature), a_node_id, initiator_nonce);
        auth_msg.version = 4;
        let mut encoded_auth_msg = encode(auth_msg);

        // Pad with random amount of data. Value range is from the spec.
        // let mut rng = rand::thread_rng();
        // let padding_length = rng.gen_range(100..=300);

        // For this case we use a fixed padding length
        let padding_length = 435 - encoded_auth_msg.len();

        encoded_auth_msg.resize(encoded_auth_msg.len() + padding_length, 0);

        // The layout is:
        // | auth_size (2) | R (65) | IV (16) | C (variable) | D (32) |
        // See https://github.com/ethereum/devp2p/blob/master/rlpx.md#ecies-encryption for the meaning of each field
        let ecies_data_size = 65 + 16 + 32;
        let auth_size: u16 = (encoded_auth_msg.len() + ecies_data_size)
            .try_into()
            .unwrap();

        let message_secret_key = SecretKey::random(&mut OsRng);
        let message_secret = ecdh_xchng(&message_secret_key, &b_static_pubkey);

        let mut secret_keys = [0; 32];
        kdf(&message_secret, &mut secret_keys);
        let aes_key = &secret_keys[..16];
        let mac_key = sha256(&secret_keys[16..]);

        let iv = H128::random();
        let mut aes_cipher = Aes128Ctr64BE::new_from_slices(aes_key, &iv.0).unwrap();
        aes_cipher
            .try_apply_keystream(&mut encoded_auth_msg)
            .unwrap();
        let encrypted_auth_msg = encoded_auth_msg;

        let r_public_key = message_secret_key.public_key().to_encoded_point(false);
        let d = sha256_hmac(
            &mac_key,
            &[&iv.0, &encrypted_auth_msg],
            &auth_size.to_be_bytes(),
        );

        let mut encrypted_auth_message = vec![0; (auth_size as usize) + 2];
        encrypted_auth_message[..2].copy_from_slice(&auth_size.to_be_bytes());
        encrypted_auth_message[2..(2 + 65)].copy_from_slice(r_public_key.as_bytes());
        encrypted_auth_message[(2 + 65)..(2 + 65 + 16)].copy_from_slice(&iv.0);
        encrypted_auth_message[(2 + 65 + 16)..(2 + 65 + 16 + encrypted_auth_msg.len())]
            .copy_from_slice(&encrypted_auth_msg);
        encrypted_auth_message[(2 + 65 + 16 + encrypted_auth_msg.len())..].copy_from_slice(&d);
        // The message uses randomized data, so it won't be the same as the test vectors

        // This is Auth₁ (A -> B)
        let mut msg = hex!("01b304ab7578555167be8154d5cc456f567d5ba302662433674222360f08d5f1534499d3678b513b0fca474f3a514b18e75683032eb63fccb16c156dc6eb2c0b1593f0d84ac74f6e475f1b8d56116b849634a8c458705bf83a626ea0384d4d7341aae591fae42ce6bd5c850bfe0b999a694a49bbbaf3ef6cda61110601d3b4c02ab6c30437257a6e0117792631a4b47c1d52fc0f8f89caadeb7d02770bf999cc147d2df3b62e1ffb2c9d8c125a3984865356266bca11ce7d3a688663a51d82defaa8aad69da39ab6d5470e81ec5f2a7a47fb865ff7cca21516f9299a07b1bc63ba56c7a1a892112841ca44b6e0034dee70c9adabc15d76a54f443593fafdc3b27af8059703f88928e199cb122362a4b35f62386da7caad09c001edaeb5f8a06d2b26fb6cb93c52a9fca51853b68193916982358fe1e5369e249875bb8d0d0ec36f917bc5e1eafd5896d46bd61ff23f1a863a8a8dcd54c7b109b771c8e61ec9c8908c733c0263440e2aa067241aaa433f0bb053c7b31a838504b148f570c0ad62837129e547678c5190341e4f1693956c3bf7678318e2d5b5340c9e488eefea198576344afbdf66db5f51204a6961a63ce072c8926c");

        // AUTH MESSAGE DECODING
        // The layout is:
        // | auth_size (2) | R (65) | IV (16) | C (variable) | D (32) |
        // See https://github.com/ethereum/devp2p/blob/master/rlpx.md#ecies-encryption for the meaning of each field
        let (auth_size, enc_auth_body) = msg.split_at_mut(2);
        let auth_size = u16::from_be_bytes(auth_size.try_into().unwrap());
        assert_eq!(auth_size as usize, enc_auth_body.len());

        let (pk, rest) = enc_auth_body.split_at_mut(65);
        let (iv, rest) = rest.split_at_mut(16);
        let (c, d) = rest.split_at_mut(rest.len() - 32);

        // This pubkey is used just for this message
        let r_pubkey = PublicKey::from_sec1_bytes(pk).unwrap();
        let iv: [u8; 16] = iv.try_into().unwrap();
        let d: [u8; 32] = d.try_into().unwrap();

        // Derive the shared secret used just for this message
        let message_shared_secret = ecdh_xchng(&b_static_key, &r_pubkey);

        // kE || kM = KDF(S, 32)
        let mut buf = [0; 32];
        kdf(&message_shared_secret, &mut buf);

        let aes_key = &buf[..16];
        let mac_key = sha256(&buf[16..]);

        let result = sha256_hmac(&mac_key, &[&iv, c], &auth_size.to_be_bytes());

        assert_eq!(result.as_slice(), &d);

        let mut stream_cipher = Aes128Ctr64BE::new_from_slices(aes_key, &iv).unwrap();
        stream_cipher.try_apply_keystream(c).unwrap();
        let decoded_msg = c;
        let expected_msg_without_padding = hex!("f8a7b841299ca6acfd35e3d72d8ba3d1e2b60b5561d5af5218eb5bc182045769eb4226910a301acae3b369fffc4a4899d6b02531e89fd4fe36a2cf0d93607ba470b50f7800b840fda1cff674c90c9a197539fe3dfb53086ace64f83ed7c6eabec741f7f381cc803e52ab2cd55d5569bce4347107a310dfd5f88a010cd2ffd1005ca406f1842877a07e968bba13b6c50e2c4cd7f241cc0d64d1ac25c7f5952df231ac6a2bda8ee5d604");
        let decoded_msg_without_padding = &decoded_msg[..expected_msg_without_padding.len()];
        assert_eq!(decoded_msg_without_padding, expected_msg_without_padding);

        // Fields are from `auth-body` in the spec: https://github.com/ethereum/devp2p/blob/master/rlpx.md#initial-handshake
        // Some info is missing from the spec: https://github.com/ethereum/devp2p/issues/218

        // Garbage used to pad the message. It is used to obfuscate the true message length.
        let (auth, _padding) = AuthMessage::decode_unfinished(&decoded_msg).unwrap();
        let rlp_decoder = Decoder::new(decoded_msg).unwrap();
        let sig = auth.signature;
        let initiator_pubkey = auth.node_id;
        let initiator_nonce = auth.nonce;
        let auth_vsn = auth.version;

        assert_eq!(initiator_nonce.0, nonce_a_bytes);
        // We're now at version 5, though.
        assert_eq!(auth_vsn, 4);

        // Garbage used to pad the message. It is used to obfuscate the true message length.
        let _padding = rlp_decoder.finish_unchecked();

        let remote_public_key = id2pubkey(initiator_pubkey.try_into().unwrap()).unwrap();

        assert_eq!(
            remote_public_key.to_sec1_bytes(),
            a_static_key.public_key().to_sec1_bytes()
        );

        // TODO: create the ack message

        // Secret 1
        let static_shared_secret = ecdh_xchng(&b_static_key, &remote_public_key);

        let signature = Signature::from_bytes(sig[..64].into()).unwrap();
        let rid = RecoveryId::from_byte(sig[64]).unwrap();

        let prehash = H256(static_shared_secret) ^ initiator_nonce;
        let remote_ephemeral_public_key = PublicKey::from(
            VerifyingKey::recover_from_prehash(&prehash.0, &signature, rid).unwrap(),
        );

        assert_eq!(remote_ephemeral_public_key, a_ephemeral_key.public_key());

        // Secret 2
        let ephemeral_key = ecdh_xchng(&b_ephemeral_key, &remote_ephemeral_public_key);

        let shared_secret_suffix = keccak([nonce_b_bytes, initiator_nonce.0].concat());
        // Secret 3
        let shared_secret = keccak([ephemeral_key, shared_secret_suffix.0].concat());

        // Secret 4
        let aes_secret = keccak([ephemeral_key, shared_secret.0].concat());

        // Secret 5
        let mac_secret = keccak([ephemeral_key, aes_secret.0].concat());

        // VALIDATION
        let expected_aes_secret =
            hex!("80e8632c05fed6fc2a13b0f8d31a3cf645366239170ea067065aba8e28bac487");
        let expected_mac_secret =
            hex!("2ea74ec5dae199227dff1af715362700e989d889d7a493cb0639691efb8e5f98");

        assert_eq!(aes_secret.0, expected_aes_secret);
        assert_eq!(mac_secret.0, expected_mac_secret);
    }
}
