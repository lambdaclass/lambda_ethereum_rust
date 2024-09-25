use super::{frame, message as rlpx};
use aes::cipher::KeyIvInit;
use ethereum_rust_core::H256;
use ethereum_rust_rlp::decode::RLPDecode;
use sha3::{Digest, Keccak256};
use tokio::io::{AsyncRead, AsyncWrite};
// pub const SUPPORTED_CAPABILITIES: [(&str, u8); 1] = [("p2p", 5)];
pub const SUPPORTED_CAPABILITIES: [(&str, u8); 2] = [("p2p", 5), ("eth", 68)];
// pub const SUPPORTED_CAPABILITIES: [(&str, u8); 3] = [("p2p", 5), ("eth", 68), ("snap", 1)];

pub(crate) type Aes256Ctr64BE = ctr::Ctr64BE<aes::Aes256>;

/// Fully working RLPx connection.
pub(crate) struct RLPxConnection<S> {
    state: RLPxState,
    stream: S,
    established: bool,
    // ...capabilities information
}

impl<S: AsyncWrite + AsyncRead + std::marker::Unpin> RLPxConnection<S> {
    pub fn new(state: RLPxState, stream: S) -> Self {
        Self {
            state,
            stream,
            established: false,
        }
    }

    pub async fn send(&mut self, message: rlpx::Message) {
        let mut frame_buffer = vec![];
        message.encode(&mut frame_buffer);
        frame::write(frame_buffer, &mut self.state, &mut self.stream).await;
    }

    pub async fn receive(&mut self) -> rlpx::Message {
        let frame_data = frame::read(&mut self.state, &mut self.stream).await;
        let (msg_id, msg_data): (u8, _) = RLPDecode::decode_unfinished(&frame_data).unwrap();
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
}

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
    use crate::rlpx::handshake::RLPxLocalClient;
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

        let mut client =
            RLPxLocalClient::new(nonce.into(), SecretKey::from_slice(&ephemeral_key).unwrap());

        assert_eq!(&client.ephemeral_key.to_bytes()[..], &ephemeral_key[..]);
        assert_eq!(client.nonce.0, nonce);

        let auth_data = msg[..2].try_into().unwrap();

        client.auth_message = Some(vec![]);

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
