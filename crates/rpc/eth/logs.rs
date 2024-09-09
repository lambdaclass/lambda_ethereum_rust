use crate::{block::BlockIdentifier, RpcErr, RpcHandler};
use ethereum_rust_core::{H160, U256};
use ethereum_rust_storage::Store;
use serde_json::{from_value, Value};

pub struct LogsRequest {
    /// The oldest block from which to start
    /// retrieving logs.
    /// Will default to `latest` if not provided.
    pub from: BlockIdentifier,
    /// Up to which block to stop retrieving logs.
    /// Will default to `latest` if not provided.
    pub to: BlockIdentifier,
    /// The addresses from where the logs origin from.
    pub address: Option<Vec<H160>>,
    /// Which topics to filter.
    pub topics: Option<Vec<U256>>,
}
impl RpcHandler for LogsRequest {
    fn parse(params: &Option<Vec<Value>>) -> Option<LogsRequest> {
        match params.as_deref() {
            Some([from, to, address, topics]) => Some(LogsRequest {
                from: from_value(from.clone()).unwrap_or(BlockIdentifier::latest()),
                to: from_value(to.clone()).unwrap_or(BlockIdentifier::latest()),
                address: from_value(address.clone()).ok(),
                topics: from_value(topics.clone()).ok(),
            }),
            _ => None,
        }
    }
    fn handle(&self, storage: Store) -> Result<Value, RpcErr> {
        todo!()
    }
}
