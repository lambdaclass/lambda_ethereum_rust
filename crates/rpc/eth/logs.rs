use crate::{
    types::{block_identifier::BlockIdentifier, receipt::RpcLog},
    RpcErr, RpcHandler,
};
use ethereum_rust_core::types::{AddressFilter, LogsFilter, TopicFilter};
use ethereum_rust_storage::Store;
use serde_json::Value;
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct LogsRequest {
    /// The oldest block from which to start
    /// retrieving logs.
    /// Will default to `latest` if not provided.
    pub from_block: BlockIdentifier,
    /// Up to which block to stop retrieving logs.
    /// Will default to `latest` if not provided.
    pub to_block: BlockIdentifier,
    /// The addresses from where the logs origin from.
    pub address_filters: Option<AddressFilter>,
    /// Which topics to filter.
    pub topics: Vec<TopicFilter>,
}
impl RpcHandler for LogsRequest {
    fn parse(params: &Option<Vec<Value>>) -> Result<LogsRequest, RpcErr> {
        match params.as_deref() {
            Some([param]) => {
                let param = param.as_object().ok_or(RpcErr::BadParams)?;
                let from_block = param
                    .get("fromBlock")
                    .ok_or_else(|| RpcErr::MissingParam("fromBlock".to_string()))
                    .and_then(|block_number| BlockIdentifier::parse(block_number.clone(), 0))?;
                let to_block = param
                    .get("toBlock")
                    .ok_or_else(|| RpcErr::MissingParam("toBlock".to_string()))
                    .and_then(|block_number| BlockIdentifier::parse(block_number.clone(), 0))?;
                let address_filters = param
                    .get("address")
                    .ok_or_else(|| RpcErr::MissingParam("address".to_string()))
                    .and_then(|address| {
                        match serde_json::from_value::<Option<AddressFilter>>(address.clone()) {
                            Ok(filters) => Ok(filters),
                            _ => Err(RpcErr::WrongParam("address".to_string())),
                        }
                    })?;
                let topics_filters = param
                    .get("topics")
                    .ok_or_else(|| RpcErr::MissingParam("topics".to_string()))
                    .and_then(|topics| {
                        match serde_json::from_value::<Option<Vec<TopicFilter>>>(topics.clone()) {
                            Ok(filters) => Ok(filters),
                            _ => Err(RpcErr::WrongParam("topics".to_string())),
                        }
                    })?;
                Ok(LogsRequest {
                    from_block,
                    to_block,
                    address_filters,
                    topics: topics_filters.unwrap_or_else(Vec::new),
                })
            }
            _ => Err(RpcErr::BadParams),
        }
    }
    // TODO: This is longer than it has the right to be, maybe we should refactor it.s
    // The main problem here is the layers of indirection needed
    // to fetch tx and block data for a log rpc response, some ideas here are:
    // - The ideal one is to have a key-value store BlockNumber -> Log, where the log also stores
    //   the block hash, transaction hash, transaction number and its own index.
    // - Another on is the receipt stores the block hash, transaction hash and block number,
    //   then we simply could retrieve each log from the receipt and add the info
    //   needed for the RPCLog struct.
    fn handle(&self, storage: Store) -> Result<Value, RpcErr> {
        let filter = self.request_to_filter(&storage)?;

        let address_filter: HashSet<_> = match &self.address_filters {
            Some(AddressFilter::Single(address)) => std::iter::once(address).collect(),
            Some(AddressFilter::Many(addresses)) => addresses.iter().collect(),
            None => HashSet::new(),
        };

        let mut logs: Vec<RpcLog> = Vec::new();
        // The idea here is to fetch every log and filter by address, if given.
        // For that, we'll need each block in range, and its transactions,
        // and for each transaction, we'll need its receipts, which
        // contain the actual logs we want.
        for block_num in filter.from_block..=filter.to_block {
            // Take the header of the block, we
            // will use it to access the transactions.
            let block_body = storage.get_block_body(block_num)?.ok_or(RpcErr::Internal)?;
            let block_header = storage
                .get_block_header(block_num)?
                .ok_or(RpcErr::Internal)?;
            let block_hash = block_header.compute_block_hash();

            let mut block_log_index = 0_u64;

            // Since transactions share indices with their receipts,
            // we'll use them to fetch their receipts, which have the actual logs.
            for (tx_index, tx) in block_body.transactions.iter().enumerate() {
                let tx_hash = tx.compute_hash();
                let receipt = storage
                    .get_receipt(block_num, tx_index as u64)?
                    .ok_or(RpcErr::Internal)?;

                if receipt.succeeded {
                    for log in &receipt.logs {
                        if address_filter.is_empty() || address_filter.contains(&log.address) {
                            // Some extra data is needed when
                            // forming the RPC response.
                            logs.push(RpcLog {
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
        // Now that we have the logs filtered by address,
        // we still need to filter by topics if it was a given parameter.

        let filtered_logs = if self.topics.is_empty() {
            logs
        } else {
            logs.into_iter()
                .filter(|rpc_log| {
                    if self.topics.len() > rpc_log.log.topics.len() {
                        return false;
                    }
                    for (i, topic_filter) in self.topics.iter().enumerate() {
                        match topic_filter {
                            TopicFilter::Topic(topic) => {
                                if rpc_log.log.topics[i] != *topic {
                                    return false;
                                }
                            }
                            TopicFilter::Topics(sub_topics) => {
                                if !sub_topics.is_empty()
                                    && !sub_topics
                                        .iter()
                                        .any(|topic| rpc_log.log.topics[i] == *topic)
                                {
                                    return false;
                                }
                            }
                        }
                    }
                    true
                })
                .collect::<Vec<RpcLog>>()
        };

        serde_json::to_value(filtered_logs).map_err(|_| RpcErr::Internal)
    }
}

impl LogsRequest {
    pub fn request_to_filter(&self, store: &Store) -> Result<LogsFilter, RpcErr> {
        let from_block = self
            .from_block
            .resolve_block_number(store)?
            .ok_or(RpcErr::WrongParam("fromBlock".to_string()))?;
        let to_block = self
            .to_block
            .resolve_block_number(store)?
            .ok_or(RpcErr::WrongParam("toBlock".to_string()))?;
        if from_block > to_block {
            return Err(RpcErr::BadParams);
        }
        Ok(LogsFilter {
            from_block,
            to_block,
            addresses: self
                .address_filters
                .clone()
                .map(|addr| match addr {
                    AddressFilter::Single(single_address) => vec![single_address],
                    AddressFilter::Many(addresses) => addresses,
                })
                .unwrap_or_default(),
            topics: self.topics.clone(),
        })
    }
}
