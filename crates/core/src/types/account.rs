use std::collections::HashMap;

use bytes::Bytes;
use ethereum_types::{H256, U256};

use crate::rlp::{
    decode::RLPDecode,
    encode::RLPEncode,
    structs::{Decoder, Encoder},
};

use super::GenesisAccount;

#[allow(unused)]
#[derive(Clone, Debug, PartialEq)]
pub struct Account {
    pub info: AccountInfo,
    pub code: Bytes,
    pub storage: HashMap<H256, H256>,
}

#[derive(Clone, Debug, PartialEq)]
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

pub fn code_hash(code: &Bytes) -> H256 {
    keccak_hash::keccak(code.as_ref())
}

impl RLPEncode for AccountInfo {
    fn encode(&self, buf: &mut dyn bytes::BufMut) {
        Encoder::new(buf)
            .encode_field(&self.code_hash)
            .encode_field(&self.balance)
            .encode_field(&self.nonce)
            .finish();
    }
}

impl RLPDecode for AccountInfo {
    fn decode_unfinished(rlp: &[u8]) -> Result<(Self, &[u8]), crate::rlp::error::RLPDecodeError> {
        let decoder = Decoder::new(rlp)?;
        let (code_hash, decoder) = decoder.decode_field("code_hash")?;
        let (balance, decoder) = decoder.decode_field("balance")?;
        let (nonce, decoder) = decoder.decode_field("nonce")?;
        let remaining = decoder.finish()?;
        let account_info = Self {
            code_hash,
            balance,
            nonce,
        };
        Ok((account_info, remaining))
    }
}

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn test_code_hash() {
        let empty_code = Bytes::new();
        let hash = code_hash(&empty_code);
        assert_eq!(
            hash,
            H256::from_str("c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470")
                .unwrap()
        )
    }
}
