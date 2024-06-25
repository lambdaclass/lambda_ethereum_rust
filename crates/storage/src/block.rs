use libmdbx::orm::{Decodable, Encodable};

pub type BlockNumber = u64;

// TODO: replace with actual types
pub type Bloom = [u8; 256];
pub type B256 = [u8; 32];
pub type U256 = [u8; 32];
pub type Bytes = Vec<u8>;
pub type Address = [u8; 20];

/// Header part of a block on the chain.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockHeader {
    parent_hash: B256,
    ommers_hash: B256,
    coinbase: Address,
    state_root: B256,
    transactions_root: B256,
    receipt_root: B256,
    logs_bloom: Bloom,
    difficulty: U256,
    number: BlockNumber,
    gas_limit: u64,
    gas_used: u64,
    timestamp: u64,
    extra_data: Bytes,
    prev_randao: B256,
    nonce: u64,
    base_fee_per_gas: u64,
    withdrawals_root: B256,
    blob_gas_used: u64,
    excess_blob_gas: u64,
    parent_beacon_block_root: B256,
}

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
