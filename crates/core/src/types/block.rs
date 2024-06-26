use crate::{rlp::encode::RLPEncode, Address, H256, U256};
use bytes::Bytes;

pub type BlockNumber = u64;
pub type Bloom = [u8; 256];

/// Header part of a block on the chain.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockHeader {
    parent_hash: H256,
    ommers_hash: H256,
    coinbase: Address,
    state_root: H256,
    transactions_root: H256,
    receipt_root: H256,
    logs_bloom: Bloom,
    difficulty: U256,
    number: BlockNumber,
    gas_limit: u64,
    gas_used: u64,
    timestamp: u64,
    extra_data: Bytes,
    prev_randao: H256,
    nonce: u64,
    base_fee_per_gas: u64,
    withdrawals_root: H256,
    blob_gas_used: u64,
    excess_blob_gas: u64,
    parent_beacon_block_root: H256,
}

impl RLPEncode for BlockHeader {
    fn encode(&self, buf: &mut dyn bytes::BufMut) {
        self.parent_hash.encode(buf);
        self.ommers_hash.encode(buf);
        self.coinbase.encode(buf);
        self.state_root.encode(buf);
        self.transactions_root.encode(buf);
        self.receipt_root.encode(buf);
        self.logs_bloom.encode(buf);

        // TODO: move to rlp::encode
        let mut tmp_buf = vec![];
        self.difficulty.to_big_endian(&mut tmp_buf);
        tmp_buf.encode(buf);

        self.number.encode(buf);
        self.gas_limit.encode(buf);
        self.gas_used.encode(buf);
        self.timestamp.encode(buf);
        self.extra_data.encode(buf);
        self.prev_randao.encode(buf);
        self.nonce.encode(buf);
        self.base_fee_per_gas.encode(buf);
        self.withdrawals_root.encode(buf);
        self.blob_gas_used.encode(buf);
        self.excess_blob_gas.encode(buf);
        self.parent_beacon_block_root.encode(buf);
    }
}

// The body of a block on the chain
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockBody {
    transactions: Vec<LegacyTransaction>,
    ommers: Vec<BlockHeader>,
    withdrawals: Vec<Withdrawal>,
}

impl RLPEncode for BlockBody {
    fn encode(&self, buf: &mut dyn bytes::BufMut) {
        self.transactions.encode(buf);
        self.ommers.encode(buf);
        self.withdrawals.encode(buf);
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Withdrawal {
    index: u64,
    validator_index: u64,
    address: Address,
    amount: U256,
}

impl RLPEncode for Withdrawal {
    fn encode(&self, buf: &mut dyn bytes::BufMut) {
        self.index.encode(buf);
        self.validator_index.encode(buf);
        self.address.encode(buf);
        self.amount.encode(buf);
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LegacyTransaction {
    nonce: U256,
    gas_price: u32,
    gas: u32,
    to: Address,
    value: U256,
    data: Bytes,
    v: U256,
    r: U256,
    s: U256,
}

impl RLPEncode for LegacyTransaction {
    fn encode(&self, buf: &mut dyn bytes::BufMut) {
        self.nonce.encode(buf);
        self.gas_price.encode(buf);
        self.gas.encode(buf);
        self.to.encode(buf);
        self.value.encode(buf);
        self.data.encode(buf);
        self.v.encode(buf);
        self.r.encode(buf);
        self.s.encode(buf);
    }
}
