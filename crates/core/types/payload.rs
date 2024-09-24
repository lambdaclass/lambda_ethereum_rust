use ethereum_rust_rlp::encode::RLPEncode;
use ethereum_types::{Address, U256};
use keccak_hash::H256;
use sha3::{Digest, Keccak256};

use super::{BlockHash, Withdrawal};

pub struct BuildPayloadArgs {
    pub parent: BlockHash,
    pub timestamp: U256,
    pub fee_recipient: Address,
    pub random: H256,
    pub withdrawals: Vec<Withdrawal>,
    pub beacon_root: Option<H256>,
    pub version: u8,
}

impl BuildPayloadArgs {
    // Id computes an 8-byte identifier by hashing the components of the payload arguments.
    pub fn id(&self) -> u64 {
        let mut hasher = Keccak256::new();
        let mut timestamp = [0; 32];
        self.timestamp.to_big_endian(&mut timestamp);
        hasher.update(self.parent);
        hasher.update(timestamp);
        hasher.update(self.random);
        hasher.update(self.fee_recipient);
        hasher.update(self.withdrawals.encode_to_vec());
        if let Some(beacon_root) = self.beacon_root {
            hasher.update(beacon_root);
        }
        let res = &mut hasher.finalize()[..8];
        res[0] = self.version;
        u64::from_be_bytes(res.try_into().unwrap())
    }
}
