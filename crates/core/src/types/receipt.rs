use crate::rlp::encode::RLPEncode;
use bytes::Bytes;
use ethereum_types::{Address, H256};
pub type Bloom = [u8; 256];

/// Result of a transaction
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Receipt {
    succeeded: bool,
    cumulative_gas_used: u64,
    bloom: Bloom,
    logs: Vec<Log>,
}

impl RLPEncode for Receipt {
    fn encode(&self, buf: &mut dyn bytes::BufMut) {
        self.succeeded.encode(buf);
        self.cumulative_gas_used.encode(buf);
        self.bloom.encode(buf);
        self.logs.encode(buf);
    }
}

/// Data record produced during the execution of a transaction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Log {
    address: Address,
    topics: Vec<H256>,
    data: Bytes,
}

impl RLPEncode for Log {
    fn encode(&self, buf: &mut dyn bytes::BufMut) {
        self.address.encode(buf);
        self.topics.encode(buf);
        self.data.encode(buf);
    }
}
