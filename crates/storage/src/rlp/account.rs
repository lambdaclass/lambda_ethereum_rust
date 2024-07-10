use ethereum_rust_core::{types::AccountInfo, Address};
use libmdbx::orm::{Decodable, Encodable};

// TODO: only Address and AccountInfo are wrapped by Rlp
//       should do the same for all structs here and in block and receipt
use crate::rlp::Rlp;

pub type AddressRLP = Rlp<Address>;

pub type AccountInfoRLP = Rlp<AccountInfo>;

pub struct AccountStorageKeyRLP(Vec<u8>);

pub struct AccountStorageValueRLP(Vec<u8>);

pub struct AccountCodeHashRLP(Vec<u8>);

pub struct AccountCodeRLP(Vec<u8>);

impl Encodable for AccountStorageKeyRLP {
    type Encoded = Vec<u8>;

    fn encode(self) -> Self::Encoded {
        self.0
    }
}

impl Decodable for AccountStorageKeyRLP {
    fn decode(b: &[u8]) -> anyhow::Result<Self> {
        Ok(AccountStorageKeyRLP(b.to_vec()))
    }
}

impl Encodable for AccountStorageValueRLP {
    type Encoded = Vec<u8>;

    fn encode(self) -> Self::Encoded {
        self.0
    }
}

impl Decodable for AccountStorageValueRLP {
    fn decode(b: &[u8]) -> anyhow::Result<Self> {
        Ok(AccountStorageValueRLP(b.to_vec()))
    }
}

impl Encodable for AccountCodeHashRLP {
    type Encoded = Vec<u8>;

    fn encode(self) -> Self::Encoded {
        self.0
    }
}

impl Decodable for AccountCodeHashRLP {
    fn decode(b: &[u8]) -> anyhow::Result<Self> {
        Ok(AccountCodeHashRLP(b.to_vec()))
    }
}

impl Encodable for AccountCodeRLP {
    type Encoded = Vec<u8>;

    fn encode(self) -> Self::Encoded {
        self.0
    }
}

impl Decodable for AccountCodeRLP {
    fn decode(b: &[u8]) -> anyhow::Result<Self> {
        Ok(AccountCodeRLP(b.to_vec()))
    }
}
