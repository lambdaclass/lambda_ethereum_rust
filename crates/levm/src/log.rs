use bytes::Bytes;
use ethereum_types::{Address, U256};

#[derive(Clone, Debug, Default, Eq, PartialEq, Hash)]
pub struct Log {
    pub address: Address,
    pub topics: Vec<U256>,
    pub data: Bytes,
}
