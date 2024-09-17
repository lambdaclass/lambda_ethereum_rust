use crate::{
    eth::block::block_number,
    types::{block_identifier::BlockIdentifier, receipt::RpcLog},
    RpcErr, RpcHandler,
};
use ethereum_rust_core::{
    types::{BlockNumber, Index, Receipt},
    H160, H256, U256,
};
use ethereum_rust_storage::Store;
use serde::Deserialize;
use serde_json::{from_value, Value};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum AddressFilter {
    Single(H160),
    Many(Vec<H160>),
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum TopicFilter {
    Topic(U256),
    Topics(Vec<TopicFilter>),
}
// TODO: This struct should be using serde,
// but I couldn't get it to work, the culprit
// seems to be BlockIdentifier enum.
#[derive(Debug)]
#[allow(non_snake_case)]
pub struct LogsRequest {
    /// The oldest block from which to start
    /// retrieving logs.
    /// Will default to `latest` if not provided.
    pub fromBlock: BlockIdentifier,
    /// Up to which block to stop retrieving logs.
    /// Will default to `latest` if not provided.
    pub toBlock: BlockIdentifier,
    /// The addresses from where the logs origin from.
    pub address_filters: Option<AddressFilter>,
    /// Which topics to filter.
    pub topics: Option<Vec<TopicFilter>>,
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
                let address_filter = param.get("address").and_then(|address| match address {
                    Value::String(_) | Value::Array(_) => {
                        Some(serde_json::from_value::<AddressFilter>(address.clone()).unwrap())
                    }
                    _ => None,
                });
                let topics = param.get("topics").and_then(|topics| {
                    Some(
                        serde_json::from_value::<Option<Vec<TopicFilter>>>(topics.clone()).unwrap(),
                    )
                });
                Ok(LogsRequest {
                    fromBlock,
                    address_filters: address_filter,
                    topics: topics.unwrap(),
                    toBlock,
                })
            }
            _ => Err(RpcErr::BadParams),
        }
    }
    // TODO: This is longer than it has the right to be, maybe we should refactor it.
    // The main problem here is the layers of indirection needed
    // to fetch tx and block data for a log rpc response, some ideas here are:
    // - The ideal one is to have a key-value store BlockNumber -> Log, where the log also stores
    //   the block hash, transaction hash, transaction number and its own index.
    // - Another on is the receipt stores the block hash, transaction hash and block number,
    //   then we simply could retrieve each log from the receipt and add the info
    //   needed for the RPCLog struct.
    fn handle(&self, storage: Store) -> Result<Value, RpcErr> {
        let Ok(Some(from)) = self.fromBlock.resolve_block_number(&storage) else {
            return Err(RpcErr::BadParams);
        };
        let Ok(Some(to)) = self.toBlock.resolve_block_number(&storage) else {
            return Err(RpcErr::BadParams);
        };

        let address_filter: BTreeSet<_> = match &self.address_filters {
            Some(AddressFilter::Single(address)) => std::iter::once(address).collect(),
            Some(AddressFilter::Many(addresses)) => addresses.iter().collect(),
            None => BTreeSet::new(),
        };

        // let topic_filter: BTreeSet<_> = match &self.topics {
        //     Some(filters) => filters.iter().collect(),
        //     None => BTreeSet::new(),
        // };
        // let topic_filter
        let mut all_logs: Vec<RpcLog> = Vec::new();
        for block_num in from..=to {
            let block_body = storage.get_block_body(block_num)?.ok_or(RpcErr::Internal)?;
            let block_header = storage
                .get_block_header(block_num)?
                .ok_or(RpcErr::Internal)?;
            let block_hash = block_header.compute_block_hash();

            let mut block_log_index = 0_u64;

            for (tx_index, tx) in block_body.transactions.iter().enumerate() {
                let tx_hash = tx.compute_hash();
                let receipt = storage
                    .get_receipt(block_num, tx_index as u64)?
                    .ok_or(RpcErr::Internal)?;

                if receipt.succeeded {
                    for log in &receipt.logs {
                        if address_filter.is_empty() || address_filter.contains(&log.address) {
                            all_logs.push(RpcLog {
                                log: log.clone().into(),
                                log_index: block_log_index,
                                transaction_hash: tx_hash,
                                transaction_index: tx_index as u64,
                                block_number: block_num,
                                block_hash,
                                removed: false,
                            });
                        }
                        block_log_index += 1;
                    }
                }
            }
        }

        serde_json::to_value(all_logs).map_err(|_| RpcErr::Internal)
    }
}
