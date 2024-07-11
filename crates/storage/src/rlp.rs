use std::marker::PhantomData;

use ethereum_rust_core::{
    rlp::{decode::RLPDecode, encode::RLPEncode},
    types::{AccountInfo, BlockBody, BlockHeader, Receipt},
};
use ethereum_types::Address;
use libmdbx::orm::{Decodable, Encodable};

// Account types
pub type AddressRLP = Rlp<Address>;
pub type AccountInfoRLP = Rlp<AccountInfo>;
pub type AccountStorageKeyRLP = Rlp<Vec<u8>>;
pub type AccountStorageValueRLP = Rlp<Vec<u8>>;
pub type AccountCodeHashRLP = Rlp<Vec<u8>>;
pub type AccountCodeRLP = Rlp<Vec<u8>>;

// Block types
pub type BlockHeaderRLP = Rlp<BlockHeader>;
pub type BlockBodyRLP = Rlp<BlockBody>;

// Receipt types
pub type ReceiptRLP = Rlp<Receipt>;

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
