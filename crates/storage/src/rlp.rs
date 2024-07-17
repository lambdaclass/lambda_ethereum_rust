use std::marker::PhantomData;

use bytes::Bytes;
use ethereum_rust_core::{
    rlp::{decode::RLPDecode, encode::RLPEncode},
    types::{AccountInfo, BlockBody, BlockHash, BlockHeader, Receipt},
    Address, H256,
};
use libmdbx::orm::{Decodable, Encodable};

// Account types
pub type AddressRLP = Rlp<Address>;
pub type AccountInfoRLP = Rlp<AccountInfo>;
pub type AccountCodeHashRLP = Rlp<H256>;
pub type AccountCodeRLP = Rlp<Bytes>;

// TODO: these structs were changed after a merge.
// See if we can reuse Rlp struct
pub struct AccountStorageKeyRLP(pub [u8; 32]);
pub struct AccountStorageValueRLP(pub [u8; 32]);

// Block types
pub type BlockHashRLP = Rlp<BlockHash>;
pub type BlockHeaderRLP = Rlp<BlockHeader>;
pub type BlockBodyRLP = Rlp<BlockBody>;

// Receipt types
pub type ReceiptRLP = Rlp<Receipt>;

#[derive(Clone)]
pub struct Rlp<T>(Vec<u8>, PhantomData<T>);

impl<T: RLPEncode> From<T> for Rlp<T> {
    fn from(value: T) -> Self {
        let mut buf = Vec::new();
        RLPEncode::encode(&value, &mut buf);
        Self(buf, Default::default())
    }
}

impl<T: RLPDecode> Rlp<T> {
    pub fn to(&self) -> T {
        T::decode(&self.0).unwrap()
    }
}

impl<T: Send + Sync> Decodable for Rlp<T> {
    fn decode(b: &[u8]) -> anyhow::Result<Self> {
        Ok(Rlp(b.to_vec(), Default::default()))
    }
}

impl<T: Send + Sync> Encodable for Rlp<T> {
    type Encoded = Vec<u8>;

    fn encode(self) -> Self::Encoded {
        self.0
    }
}

impl Encodable for AccountStorageKeyRLP {
    type Encoded = [u8; 32];

    fn encode(self) -> Self::Encoded {
        self.0
    }
}

impl Decodable for AccountStorageKeyRLP {
    fn decode(b: &[u8]) -> anyhow::Result<Self> {
        Ok(AccountStorageKeyRLP(b.try_into()?))
    }
}

impl Encodable for AccountStorageValueRLP {
    type Encoded = [u8; 32];

    fn encode(self) -> Self::Encoded {
        self.0
    }
}

impl Decodable for AccountStorageValueRLP {
    fn decode(b: &[u8]) -> anyhow::Result<Self> {
        Ok(AccountStorageValueRLP(b.try_into()?))
    }
}

impl From<H256> for AccountStorageKeyRLP {
    fn from(value: H256) -> Self {
        AccountStorageKeyRLP(value.0)
    }
}

impl From<H256> for AccountStorageValueRLP {
    fn from(value: H256) -> Self {
        AccountStorageValueRLP(value.0)
    }
}

impl From<AccountStorageValueRLP> for H256 {
    fn from(value: AccountStorageValueRLP) -> Self {
        H256(value.0)
    }
}
