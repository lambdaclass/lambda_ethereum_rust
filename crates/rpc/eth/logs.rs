use crate::{types::block_identifier::BlockIdentifier, RpcErr, RpcHandler};
use ethereum_rust_core::{H160, U256};
use ethereum_rust_storage::Store;
use serde::Deserialize;
use serde_json::{from_value, Value};

#[derive(Debug)]
pub struct LogsRequest {
    /// The oldest block from which to start
    /// retrieving logs.
    /// Will default to `latest` if not provided.
    pub fromBlock: BlockIdentifier,
    /// Up to which block to stop retrieving logs.
    /// Will default to `latest` if not provided.
    pub toBlock: BlockIdentifier,
    /// The addresses from where the logs origin from.
    pub address: Option<Vec<H160>>,
    /// Which topics to filter.
    pub topics: Option<Vec<U256>>,
}
impl RpcHandler for LogsRequest {
    fn parse(params: &Option<Vec<Value>>) -> Result<LogsRequest, RpcErr> {
        match params.as_deref() {
            Some([param]) => {
                let param = param.as_object().ok_or(RpcErr::BadParams)?;
                let fromBlock = {
                    if let Some(param) = param.get("fromBlock") {
                        BlockIdentifier::parse(param.clone(), 0)?
                    } else {
                        BlockIdentifier::latest()
                    }
                };
                let toBlock = {
                    if let Some(param) = param.get("toBlock") {
                        BlockIdentifier::parse(param.clone(), 1)?
                    } else {
                        BlockIdentifier::latest()
                    }
                };
                let address = None;
                let topics = None;
                Ok(LogsRequest {
                    fromBlock,
                    address,
                    topics,
                    toBlock,
                })
            }
            Some(params) => unreachable!("{params:?}"),
            _ => Err(RpcErr::BadParams),
        }
    }
    fn handle(&self, storage: Store) -> Result<Value, RpcErr> {
        let Ok(Some(from)) = dbg!(self.fromBlock.resolve_block_number(&storage)) else {
            return Err(RpcErr::BadParams);
        };
        let Ok(Some(to)) = self.toBlock.resolve_block_number(&storage) else {
            return Err(RpcErr::BadParams);
        };
        let logs = storage
            .get_logs_in_range(from, to)
            .map_err(|_| RpcErr::Internal)?;
        serde_json::to_value(logs).map_err(|_| RpcErr::Internal)
    }
}
