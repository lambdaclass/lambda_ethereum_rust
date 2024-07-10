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
#[allow(unused)]
pub(crate) struct ConnSecrets {
    pub remote_nonce: H256,
    pub remote_ephemeral_key: PublicKey,
    pub ephemeral_shared_secret: H256,
    pub aes_key: H256,
    pub mac_key: H256,
}

impl RLPxConnection {
    pub fn new(nonce: H256, ephemeral_key: SecretKey) -> Self {
        Self {
            nonce,
            ephemeral_key,
            secrets: None,
        }
    }

    pub fn random() -> Self {
        let mut rng = thread_rng();
        Self::new(H256::random_using(&mut rng), SecretKey::random(&mut rng))
    }

    #[allow(unused)]
    pub fn is_connected(&self) -> bool {
        self.secrets.is_some()
    }

    pub fn encode_auth_message(
        &self,
        static_key: &SecretKey,
        remote_static_pubkey: &PublicKey,
        buf: &mut dyn BufMut,
    ) {
        let mut rng = rand::thread_rng();
        let node_id = pubkey2id(&static_key.public_key());

        let message_secret_key = SecretKey::random(&mut rng);
        let message_secret = ecdh_xchng(&message_secret_key, remote_static_pubkey);

        let signature = self.sign_shared_secret(message_secret.into());

        let auth = AuthMessage::new(signature, node_id, self.nonce);
        let padding_length = rng.gen_range(100..=300);

        let mut encoded_auth_msg = vec![];
        auth.encode(&mut encoded_auth_msg);
        encoded_auth_msg.resize(encoded_auth_msg.len() + padding_length, 0);

        let ecies_data_size = 65 + 16 + 32;
        let auth_size: u16 = (encoded_auth_msg.len() + ecies_data_size)
            .try_into()
            .unwrap();
        let auth_size_bytes = auth_size.to_be_bytes();

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
        buf.put_slice(r_public_key.as_bytes());
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

    pub fn decode_ack_message(&mut self, static_key: &SecretKey, msg: &mut [u8]) -> AckMessage {
        // TODO: return errors instead of panicking
        assert!(msg.len() > 2 + 65 + 16 + 32, "message is invalid");
        let (ack_size_bytes, enc_auth_body_with_padding) = msg.split_at_mut(2);
        let ack_size = u16::from_be_bytes(ack_size_bytes.try_into().unwrap());

        // Discard any remaining bytes.
        let (enc_auth_body, _rest) = enc_auth_body_with_padding.split_at_mut(ack_size as _);

        let (pk, rest) = enc_auth_body.split_at_mut(65);
        let (iv, rest) = rest.split_at_mut(16);
        let (c, d) = rest.split_at_mut(rest.len() - 32);

        let shared_secret = ecdh_xchng(static_key, &PublicKey::from_sec1_bytes(pk).unwrap());

        let mut buf = [0; 32];
        kdf(&shared_secret, &mut buf);
        let aes_key = &buf[..16];
        let mac_key = sha256(&buf[16..]);

        let expected_d = sha256_hmac(&mac_key, &[iv, c], ack_size_bytes);

        assert_eq!(d, expected_d);

        let mut stream_cipher = Aes128Ctr64BE::new_from_slices(aes_key, iv).unwrap();

        stream_cipher.try_apply_keystream(c).unwrap();

        let (ack, _padding) = AckMessage::decode_unfinished(c).unwrap();
        // TODO: compute secrets from ack
        ack
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

#[derive(Debug)]
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
    #[allow(unused)]
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

fn sha256(data: &[u8]) -> [u8; 32] {
    use k256::sha2::Digest;
    k256::sha2::Sha256::digest(data).into()
}

fn sha256_hmac(key: &[u8], inputs: &[&[u8]], auth_data: &[u8]) -> [u8; 32] {
    use hmac::Mac;
    use k256::sha2::Sha256;

    let mut hasher = hmac::Hmac::<Sha256>::new_from_slice(key).unwrap();
    for input in inputs {
        hasher.update(input);
    }
    hasher.update(auth_data);
    hasher.finalize().into_bytes().into()
}

fn ecdh_xchng(secret_key: &SecretKey, public_key: &PublicKey) -> [u8; 32] {
    k256::ecdh::diffie_hellman(secret_key.to_nonzero_scalar(), public_key.as_affine())
        .raw_secret_bytes()[..32]
        .try_into()
        .unwrap()
}

fn kdf(secret: &[u8], output: &mut [u8]) {
    // We don't use the `other_info` field
    concat_kdf::derive_key_into::<k256::sha2::Sha256>(secret, &[], output).unwrap();
}

/// Computes recipient id from public key.
pub fn pubkey2id(pk: &PublicKey) -> H512 {
    let encoded = pk.to_encoded_point(false);
    let bytes = encoded.as_bytes();
    debug_assert_eq!(bytes[0], 4);
    H512::from_slice(&bytes[1..])
}

/// Computes public key from recipient id.
/// The node ID is the uncompressed public key of a node, with the first byte omitted (0x04).
pub fn id2pubkey(id: H512) -> Option<PublicKey> {
    let point = EncodedPoint::from_untagged_bytes(&id.0.into());
    PublicKey::from_encoded_point(&point).into_option()
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex_literal::hex;

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

    #[test]
    fn test_ack_decoding() {
        // This is the Ack₂ message from EIP-8.
        let mut msg = hex!("01ea0451958701280a56482929d3b0757da8f7fbe5286784beead59d95089c217c9b917788989470b0e330cc6e4fb383c0340ed85fab836ec9fb8a49672712aeabbdfd1e837c1ff4cace34311cd7f4de05d59279e3524ab26ef753a0095637ac88f2b499b9914b5f64e143eae548a1066e14cd2f4bd7f814c4652f11b254f8a2d0191e2f5546fae6055694aed14d906df79ad3b407d94692694e259191cde171ad542fc588fa2b7333313d82a9f887332f1dfc36cea03f831cb9a23fea05b33deb999e85489e645f6aab1872475d488d7bd6c7c120caf28dbfc5d6833888155ed69d34dbdc39c1f299be1057810f34fbe754d021bfca14dc989753d61c413d261934e1a9c67ee060a25eefb54e81a4d14baff922180c395d3f998d70f46f6b58306f969627ae364497e73fc27f6d17ae45a413d322cb8814276be6ddd13b885b201b943213656cde498fa0e9ddc8e0b8f8a53824fbd82254f3e2c17e8eaea009c38b4aa0a3f306e8797db43c25d68e86f262e564086f59a2fc60511c42abfb3057c247a8a8fe4fb3ccbadde17514b7ac8000cdb6a912778426260c47f38919a91f25f4b5ffb455d6aaaf150f7e5529c100ce62d6d92826a71778d809bdf60232ae21ce8a437eca8223f45ac37f6487452ce626f549b3b5fdee26afd2072e4bc75833c2464c805246155289f4");

        let static_key = hex!("49a7b37aa6f6645917e7b807e9d1c00d4fa71f18343b0d4122a4d2df64dd6fee");
        let nonce = hex!("7e968bba13b6c50e2c4cd7f241cc0d64d1ac25c7f5952df231ac6a2bda8ee5d6");
        let ephemeral_key =
            hex!("869d6ecf5211f1cc60418a13b9d870b22959d0c16f02bec714c960dd2298a32d");

        let mut conn =
            RLPxConnection::new(nonce.into(), SecretKey::from_slice(&ephemeral_key).unwrap());
        let ack = conn.decode_ack_message(&SecretKey::from_slice(&static_key).unwrap(), &mut msg);

        let expected_nonce =
            hex!("559aead08264d5795d3909718cdd05abd49572e84fe55590eef31a88a08fdffd");
        let expected_ephemeral_key = SecretKey::from_slice(&hex!(
            "e238eb8e04fee6511ab04c6dd3c89ce097b11f25d584863ac2b6d5b35b1847e4"
        ))
        .unwrap()
        .public_key()
        .to_encoded_point(false);

        assert_eq!(ack.nonce.0, expected_nonce);
        assert_eq!(
            &ack.ephemeral_pubkey.0,
            &expected_ephemeral_key.as_bytes()[1..]
        );
        // The test was for version 4.
        assert_eq!(ack.version, 4);
    }
}
