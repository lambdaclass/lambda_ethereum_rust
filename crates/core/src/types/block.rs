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
// TODO: rename it to `Body`
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockBody {
    transactions: Vec<Transaction>,
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
pub enum Transaction {
    LegacyTransaction(LegacyTransaction),
    EIP1559Transaction(EIP1559Transaction),
}

impl RLPEncode for Transaction {
    fn encode(&self, buf: &mut dyn bytes::BufMut) {
        match self {
            Transaction::LegacyTransaction(t) => t.encode(buf),
            Transaction::EIP1559Transaction(t) => t.encode(buf),
        };
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LegacyTransaction {
    nonce: U256,
    gas_price: u64,
    gas: u64,
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EIP1559Transaction {
    chain_id: u64,
    signer_nonce: U256,
    max_priority_fee_per_gas: u64,
    max_fee_per_gas: u64,
    gas_limit: u64,
    destination: Address,
    amount: u64,
    payload: Bytes,
    access_list: Vec<(Address, Vec<H256>)>,
    signature_y_parity: bool,
    signature_r: U256,
    signature_s: U256,
}

impl RLPEncode for EIP1559Transaction {
    fn encode(&self, buf: &mut dyn bytes::BufMut) {
        self.chain_id.encode(buf);
        self.signer_nonce.encode(buf);
        self.max_priority_fee_per_gas.encode(buf);
        self.max_fee_per_gas.encode(buf);
        self.gas_limit.encode(buf);
        self.destination.encode(buf);
        self.amount.encode(buf);
        self.payload.encode(buf);
        self.access_list.encode(buf);
        self.signature_y_parity.encode(buf);
        self.signature_r.encode(buf);
        self.signature_s.encode(buf);
    }
}
