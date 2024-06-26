use libmdbx::orm::{Decodable, Encodable};

pub struct AddressRLP(Vec<u8>);

pub struct AccountInfoRLP(Vec<u8>);

pub struct AccountStorageKeyRLP(Vec<u8>);

pub struct AccountStorageValueRLP(Vec<u8>);

impl Encodable for AddressRLP {
    type Encoded = Vec<u8>;

    fn encode(self) -> Self::Encoded {
        self.0
    }
}

impl Decodable for AddressRLP {
    fn decode(b: &[u8]) -> anyhow::Result<Self> {
        Ok(AddressRLP(b.to_vec()))
    }
}

impl Encodable for AccountInfoRLP {
    type Encoded = Vec<u8>;

    fn encode(self) -> Self::Encoded {
        self.0
    }
}

impl Decodable for AccountInfoRLP {
    fn decode(b: &[u8]) -> anyhow::Result<Self> {
        Ok(AccountInfoRLP(b.to_vec()))
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
