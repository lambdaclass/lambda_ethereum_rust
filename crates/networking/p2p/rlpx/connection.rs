use std::net::SocketAddr;

use crate::{
    rlpx::{handshake::encode_ack_message, message::Message, p2p, utils::id2pubkey},
    MAX_DISC_PACKET_SIZE,
};

use super::{
    frame,
    handshake::decode_auth_message,
    message as rlpx,
    utils::{ecdh_xchng, pubkey2id},
};
use aes::cipher::KeyIvInit;
use bytes::BufMut as _;
use ethereum_rust_core::{H256, H512};
use ethereum_rust_rlp::decode::RLPDecode;
use k256::{
    ecdsa::{RecoveryId, Signature, SigningKey, VerifyingKey},
    PublicKey, SecretKey,
};
use sha3::{Digest, Keccak256};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    net::{TcpSocket, TcpStream},
};
use tracing::info;
// pub const SUPPORTED_CAPABILITIES: [(&str, u8); 1] = [("p2p", 5)];
pub const SUPPORTED_CAPABILITIES: [(&str, u8); 2] = [("p2p", 5), ("eth", 68)];
// pub const SUPPORTED_CAPABILITIES: [(&str, u8); 3] = [("p2p", 5), ("eth", 68), ("snap", 1)];

pub(crate) type Aes256Ctr64BE = ctr::Ctr64BE<aes::Aes256>;

/// Fully working RLPx connection.
pub(crate) struct RLPxConnection<S> {
    signer: SigningKey,
    state: RLPxConnectionState,
    stream: S,
    established: bool,
    // ...capabilities information
}

impl<S: AsyncWrite + AsyncRead + std::marker::Unpin> RLPxConnection<S> {
    fn new(signer: SigningKey, stream: S, state: RLPxConnectionState) -> Self {
        Self {
            signer,
            state,
            stream,
            established: false,
        }
    }

    pub fn receiver(signer: SigningKey, stream: S) -> Self {
        let mut rng = rand::thread_rng();
        Self::new(
            signer,
            stream,
            RLPxConnectionState::Receiver(Receiver::new(
                H256::random_using(&mut rng),
                SecretKey::random(&mut rng),
            )),
        )
    }

    pub async fn initiator(signer: SigningKey, msg: &[u8], stream: S) -> Self {
        let mut rng = rand::thread_rng();
        let digest = Keccak256::digest(&msg[65..]);
        let signature = &Signature::from_bytes(msg[..64].into()).unwrap();
        let rid = RecoveryId::from_byte(msg[64]).unwrap();
        let peer_pk = VerifyingKey::recover_from_prehash(&digest, signature, rid).unwrap();
        let state = RLPxConnectionState::Initiator(Initiator::new(
            H256::random_using(&mut rng),
            SecretKey::random(&mut rng),
            pubkey2id(&peer_pk.into()),
        ));
        RLPxConnection::new(signer, stream, state)
    }

    pub async fn handshake(&mut self) {
        match &self.state {
            RLPxConnectionState::Receiver(_) => {
                self.receive_auth().await;
                self.send_ack().await;
            }
            RLPxConnectionState::Initiator(_) => {
                self.send_auth().await;
                self.receive_ack().await;
            }
            _ => panic!("Invalid state for handshake"),
        }
        info!("Completed handshake!");

        self.exchange_hello_messages().await;
        info!("Completed Hello roundtrip!");
    }

    pub async fn receive_auth(&mut self) {
        match &self.state {
            RLPxConnectionState::Receiver(receiver_state) => {
                let secret_key: SecretKey = self.signer.clone().into();
                let mut buf = vec![0; MAX_DISC_PACKET_SIZE];

                // Read the auth message's size
                self.stream.read_exact(&mut buf[..2]).await.unwrap();
                let auth_data = buf[..2].try_into().unwrap();
                let msg_size = u16::from_be_bytes(auth_data) as usize;

                // Read the rest of the auth message
                self.stream
                    .read_exact(&mut buf[2..msg_size + 2])
                    .await
                    .unwrap();

                let auth_bytes = &buf[..msg_size + 2];

                let msg = &buf[2..msg_size + 2];

                let (auth, remote_ephemeral_key) = decode_auth_message(&secret_key, msg, auth_data);

                self.state = RLPxConnectionState::HandshakeAuth(HandshakeAuth::receiver(
                    receiver_state,
                    auth.node_id,
                    auth_bytes.to_owned(),
                    auth.nonce,
                    remote_ephemeral_key,
                ))
            }
            // TODO proper error
            _ => panic!(),
        };
    }

    pub async fn send_ack(&mut self) {
        match &self.state {
            RLPxConnectionState::HandshakeAuth(handshake_auth) => {
                let secret_key: SecretKey = self.signer.clone().into();
                let peer_pk = id2pubkey(handshake_auth.remote_node_id).unwrap();

                let mut ack_message = vec![];
                let msg = encode_ack_message(
                    &secret_key,
                    &handshake_auth.local_ephemeral_key,
                    handshake_auth.local_nonce,
                    &peer_pk,
                    &handshake_auth.remote_ephemeral_key,
                    &mut ack_message,
                );

                ack_message.put_slice(&msg);
                self.stream.write_all(&ack_message).await.unwrap();
                info!("Sent ack message correctly!");

                let (aes_key, mac_key) = Self::derive_secrets(handshake_auth);

                self.state = RLPxConnectionState::PostHandshake(PostHandshake::receiver(
                    handshake_auth,
                    ack_message,
                    aes_key,
                    mac_key,
                ))
            }
            // TODO proper error
            _ => panic!(),
        };
    }

    pub async fn exchange_hello_messages(&mut self) {
        let hello_msg = Message::Hello(p2p::HelloMessage::new(
            SUPPORTED_CAPABILITIES
                .into_iter()
                .map(|(name, version)| (name.to_string(), version))
                .collect(),
            PublicKey::from(self.signer.verifying_key()),
        ));

        // Receive Hello message
        let msg = self.receive().await;

        info!("{msg:?}");

        self.send(hello_msg).await;

        // self.state = RLPxConnectionState::PostHandshake(PostHandshake::receiver(
        //     handshake_auth,
        //     ack_message,
        //     aes_key,
        //     mac_key,
        // ))
    }

    fn derive_secrets(auth_state: &HandshakeAuth) -> (H256, H256) {
        // TODO: don't panic
        let ephemeral_key_secret = ecdh_xchng(
            &auth_state.local_ephemeral_key,
            &auth_state.remote_ephemeral_key,
        );

        // Get proper receiver/initiator nonces
        let (receiver_nonce, initiator_nonce) = if auth_state.local_initiator {
            (auth_state.remote_nonce.0, auth_state.local_nonce.0)
        } else {
            (auth_state.local_nonce.0, auth_state.remote_nonce.0)
        };
        // keccak256(nonce || initiator-nonce)
        let hashed_nonces = Keccak256::digest([receiver_nonce, initiator_nonce].concat()).into();
        // shared-secret = keccak256(ephemeral-key || keccak256(nonce || initiator-nonce))
        let shared_secret =
            Keccak256::digest([ephemeral_key_secret, hashed_nonces].concat()).into();
        // aes-secret = keccak256(ephemeral-key || shared-secret)
        let aes_key = Keccak256::digest([ephemeral_key_secret, shared_secret].concat()).into();
        // mac-secret = keccak256(ephemeral-key || aes-secret)
        let mac_key = Keccak256::digest([ephemeral_key_secret, aes_key].concat());

        (H256(aes_key), H256(mac_key.into()))
    }

    pub async fn send(&mut self, message: rlpx::Message) {
        match &mut self.state {
            RLPxConnectionState::PostHandshake(post_handshake) => {
                let mut frame_buffer = vec![];
                message.encode(&mut frame_buffer);
                frame::write(frame_buffer, post_handshake, &mut self.stream).await;
            }
            // TODO proper error
            _ => panic!(),
        }
    }

    pub async fn receive(&mut self) -> rlpx::Message {
        match &mut self.state {
            RLPxConnectionState::PostHandshake(post_handshake) => {
                let frame_data = frame::read(post_handshake, &mut self.stream).await;
                let (msg_id, msg_data): (u8, _) =
                    RLPDecode::decode_unfinished(&frame_data).unwrap();
                if !self.established {
                    if msg_id == 0 {
                        let message = rlpx::Message::decode(msg_id, msg_data).unwrap();
                        assert!(
                            matches!(message, rlpx::Message::Hello(_)),
                            "Expected Hello message"
                        );
                        self.established = true;
                        // TODO, register shared capabilities
                        message
                    } else {
                        // if it is not a hello message panic
                        panic!("Expected Hello message")
                    }
                } else {
                    rlpx::Message::decode(msg_id, msg_data).unwrap()
                }
            }
            // TODO proper error
            _ => panic!(),
        }
    }
}

enum RLPxConnectionState {
    Initiator(Initiator),
    Receiver(Receiver),
    HandshakeAuth(HandshakeAuth),
    PostHandshake(PostHandshake),
    Live(),
}

struct Receiver {
    pub(crate) nonce: H256,
    pub(crate) ephemeral_key: SecretKey,
}

impl Receiver {
    pub fn new(nonce: H256, ephemeral_key: SecretKey) -> Self {
        Self {
            nonce,
            ephemeral_key,
        }
    }
}

struct Initiator {
    pub(crate) nonce: H256,
    pub(crate) ephemeral_key: SecretKey,
    pub(crate) remote_node_id: H512,
}

impl Initiator {
    pub fn new(nonce: H256, ephemeral_key: SecretKey, remote_node_id: H512) -> Self {
        Self {
            nonce,
            ephemeral_key,
            remote_node_id,
        }
    }
}

struct HandshakeAuth {
    pub(crate) local_initiator: bool,
    pub(crate) local_nonce: H256,
    pub(crate) local_ephemeral_key: SecretKey,
    pub(crate) remote_node_id: H512,
    pub(crate) remote_nonce: H256,
    pub(crate) remote_ephemeral_key: PublicKey,
    pub(crate) remote_init_message: Vec<u8>,
}

impl HandshakeAuth {
    pub fn receiver(
        previous_state: &Receiver,
        remote_node_id: H512,
        remote_init_message: Vec<u8>,
        remote_nonce: H256,
        remote_ephemeral_key: PublicKey,
    ) -> Self {
        Self {
            local_initiator: false,
            local_nonce: previous_state.nonce,
            local_ephemeral_key: previous_state.ephemeral_key.clone(),
            remote_node_id,
            remote_nonce,
            remote_ephemeral_key,
            remote_init_message,
        }
    }
}

pub struct PostHandshake {
    pub(crate) local_initiator: bool,

    pub(crate) local_nonce: H256,
    pub(crate) local_ephemeral_key: SecretKey,
    pub(crate) local_init_message: Vec<u8>,
    pub(crate) remote_node_id: H512,
    pub(crate) remote_nonce: H256,
    pub(crate) remote_ephemeral_key: PublicKey,
    pub(crate) remote_init_message: Vec<u8>,

    pub(crate) aes_key: H256,
    pub(crate) mac_key: H256,
    pub ingress_mac: Keccak256,
    pub egress_mac: Keccak256,
    pub ingress_aes: Aes256Ctr64BE,
    pub egress_aes: Aes256Ctr64BE,
}

impl PostHandshake {
    pub fn receiver(
        previous_state: &HandshakeAuth,
        init_message: Vec<u8>,
        aes_key: H256,
        mac_key: H256,
    ) -> Self {
        // egress-mac = keccak256.init((mac-secret ^ remote-nonce) || auth)
        let egress_mac = Keccak256::default()
            .chain_update(mac_key ^ previous_state.remote_nonce)
            .chain_update(&init_message);

        // ingress-mac = keccak256.init((mac-secret ^ initiator-nonce) || ack)
        let ingress_mac = Keccak256::default()
            .chain_update(mac_key ^ previous_state.local_nonce)
            .chain_update(&previous_state.remote_init_message);

        let ingress_aes = <Aes256Ctr64BE as KeyIvInit>::new(&aes_key.0.into(), &[0; 16].into());
        let egress_aes = ingress_aes.clone();
        Self {
            local_initiator: previous_state.local_initiator,
            local_nonce: previous_state.local_nonce,
            local_ephemeral_key: previous_state.local_ephemeral_key.clone(),
            local_init_message: init_message,
            remote_node_id: previous_state.remote_node_id,
            remote_nonce: previous_state.remote_nonce,
            remote_ephemeral_key: previous_state.remote_ephemeral_key,
            remote_init_message: previous_state.remote_init_message.clone(),
            aes_key,
            mac_key,
            ingress_mac,
            egress_mac,
            ingress_aes,
            egress_aes,
        }
    }
}

// TODO: this state will be replaced by RLPxConnectionState
//       Sould remove this: leaving by now for reference
/// The current state of an RLPx connection
#[derive(Clone)]
pub(crate) struct RLPxState {
    // TODO: maybe precompute some values that are used more than once
    #[allow(unused)]
    pub mac_key: H256,
    pub ingress_mac: Keccak256,
    pub egress_mac: Keccak256,
    pub ingress_aes: Aes256Ctr64BE,
    pub egress_aes: Aes256Ctr64BE,
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
            mac_key,
            ingress_mac,
            egress_mac,
            ingress_aes,
            egress_aes,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::rlpx::handshake::RLPxClient;
    use hex_literal::hex;
    use k256::SecretKey;

    #[test]
    fn test_ack_decoding() {
        // This is the Ack₂ message from EIP-8.
        let msg = hex!("01ea0451958701280a56482929d3b0757da8f7fbe5286784beead59d95089c217c9b917788989470b0e330cc6e4fb383c0340ed85fab836ec9fb8a49672712aeabbdfd1e837c1ff4cace34311cd7f4de05d59279e3524ab26ef753a0095637ac88f2b499b9914b5f64e143eae548a1066e14cd2f4bd7f814c4652f11b254f8a2d0191e2f5546fae6055694aed14d906df79ad3b407d94692694e259191cde171ad542fc588fa2b7333313d82a9f887332f1dfc36cea03f831cb9a23fea05b33deb999e85489e645f6aab1872475d488d7bd6c7c120caf28dbfc5d6833888155ed69d34dbdc39c1f299be1057810f34fbe754d021bfca14dc989753d61c413d261934e1a9c67ee060a25eefb54e81a4d14baff922180c395d3f998d70f46f6b58306f969627ae364497e73fc27f6d17ae45a413d322cb8814276be6ddd13b885b201b943213656cde498fa0e9ddc8e0b8f8a53824fbd82254f3e2c17e8eaea009c38b4aa0a3f306e8797db43c25d68e86f262e564086f59a2fc60511c42abfb3057c247a8a8fe4fb3ccbadde17514b7ac8000cdb6a912778426260c47f38919a91f25f4b5ffb455d6aaaf150f7e5529c100ce62d6d92826a71778d809bdf60232ae21ce8a437eca8223f45ac37f6487452ce626f549b3b5fdee26afd2072e4bc75833c2464c805246155289f4");

        let static_key = hex!("49a7b37aa6f6645917e7b807e9d1c00d4fa71f18343b0d4122a4d2df64dd6fee");
        let nonce = hex!("7e968bba13b6c50e2c4cd7f241cc0d64d1ac25c7f5952df231ac6a2bda8ee5d6");
        let ephemeral_key =
            hex!("869d6ecf5211f1cc60418a13b9d870b22959d0c16f02bec714c960dd2298a32d");

        let mut client = RLPxClient::new(
            true,
            nonce.into(),
            SecretKey::from_slice(&ephemeral_key).unwrap(),
        );

        assert_eq!(
            &client.local_ephemeral_key.to_bytes()[..],
            &ephemeral_key[..]
        );
        assert_eq!(client.local_nonce.0, nonce);

        let auth_data = msg[..2].try_into().unwrap();

        client.local_init_message = Some(vec![]);

        let state = client.decode_ack_message(
            &SecretKey::from_slice(&static_key).unwrap(),
            &msg[2..],
            auth_data,
        );

        let expected_mac_secret =
            hex!("2ea74ec5dae199227dff1af715362700e989d889d7a493cb0639691efb8e5f98");

        assert_eq!(state.mac_key.0, expected_mac_secret);
    }
}
