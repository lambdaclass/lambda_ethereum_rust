use crate::rlp::{encode::RLPEncode, structs::Encoder};
use crate::types::Bloom;
use bytes::Bytes;
use ethereum_types::{Address, H256};
pub type Index = u64;

/// Result of a transaction
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Receipt {
    succeeded: bool,
    cumulative_gas_used: u64,
    bloom: Bloom,
    logs: Vec<Log>,
}

impl Receipt {
    pub fn new(succeeded: bool, cumulative_gas_used: u64, bloom: Bloom, logs: Vec<Log>) -> Self {
        Self {
            succeeded,
            cumulative_gas_used,
            bloom,
            logs,
        }
    }
}

impl RLPEncode for Receipt {
    fn encode(&self, buf: &mut dyn bytes::BufMut) {
        Encoder::new(buf)
            .encode_field(&self.succeeded)
            .encode_field(&self.cumulative_gas_used)
            .encode_field(&self.bloom)
            .encode_field(&self.logs)
            .finish();
    }
}

/// Data record produced during the execution of a transaction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Log {
    address: Address,
    topics: Vec<H256>,
    data: Bytes,
}

impl Log {
    pub fn new(address: Address, topics: Vec<H256>, data: Bytes) -> Self {
        Self {
            address,
            topics,
            data,
        }
    }
}

impl RLPEncode for Log {
    fn encode(&self, buf: &mut dyn bytes::BufMut) {
        Encoder::new(buf)
            .encode_field(&self.address)
            .encode_field(&self.topics)
            .encode_field(&self.data)
            .finish();
    }
}
