use libmdbx::orm::{Decodable, Encodable};

pub struct BlockHeaderRLP(Vec<u8>);

impl Encodable for BlockHeaderRLP {
    type Encoded = Vec<u8>;

    fn encode(self) -> Self::Encoded {
        self.0
    }
}

impl Decodable for BlockHeaderRLP {
    fn decode(b: &[u8]) -> anyhow::Result<Self> {
        Ok(BlockHeaderRLP(b.to_vec()))
    }
}

pub struct BlockBodyRLP(Vec<u8>);

impl Encodable for BlockBodyRLP {
    type Encoded = Vec<u8>;

    fn encode(self) -> Self::Encoded {
        self.0
    }
}

impl Decodable for BlockBodyRLP {
    fn decode(b: &[u8]) -> anyhow::Result<Self> {
        Ok(BlockBodyRLP(b.to_vec()))
    }
}
