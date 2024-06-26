use std::collections::HashMap;

use bytes::Bytes;
use ethereum_types::{H256, U256};
use tiny_keccak::{Hasher, Sha3};

use crate::rlp::encode::RLPEncode;

use super::GenesisAccount;

#[allow(unused)]
#[derive(Debug, PartialEq)]
pub struct Account {
    pub info: AccountInfo,
    pub code: Bytes,
    pub storage: HashMap<H256, H256>,
}

#[derive(Debug, PartialEq)]
pub struct AccountInfo {
    pub code_hash: H256,
    pub balance: U256,
    pub nonce: u64,
}

impl From<GenesisAccount> for Account {
    fn from(genesis: GenesisAccount) -> Self {
        Self {
            info: AccountInfo {
                code_hash: code_hash(&genesis.code),
                balance: genesis.balance,
                nonce: genesis.nonce,
            },
            code: genesis.code,
            storage: genesis.storage,
        }
    }
}

fn code_hash(code: &Bytes) -> H256 {
    let mut sha3 = Sha3::v256();
    let mut output = [0; 32];
    sha3.update(code.as_ref());
    sha3.finalize(&mut output);
    H256(output)
}

impl RLPEncode for AccountInfo {
    fn encode(&self, buf: &mut dyn bytes::BufMut) {
        self.code_hash.encode(buf);
        self.balance.encode(buf);
        self.nonce.encode(buf);
    }
}
