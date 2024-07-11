use libmdbx::orm::{Decodable, Encodable};

pub struct AddressRLP(pub Vec<u8>);

pub struct AccountInfoRLP(pub Vec<u8>);

pub struct AccountStorageKeyRLP(pub [u8; 32]);

pub struct AccountStorageValueRLP(pub [u8; 32]);

pub struct AccountCodeHashRLP(Vec<u8>);

pub struct AccountCodeRLP(Vec<u8>);

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
