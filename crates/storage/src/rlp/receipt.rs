use libmdbx::orm::{Decodable, Encodable};

pub struct ReceiptRLP(Vec<u8>);

impl Encodable for ReceiptRLP {
    type Encoded = Vec<u8>;

    fn encode(self) -> Self::Encoded {
        self.0
    }
}

impl Decodable for ReceiptRLP {
    fn decode(b: &[u8]) -> anyhow::Result<Self> {
        Ok(ReceiptRLP(b.to_vec()))
    }
}
