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
    pub address: Option<AddressFilter>,
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
                let address_filter = param.get("address").and_then(|address| match address {
                    Value::String(_) | Value::Array(_) => {
                        Some(serde_json::from_value::<AddressFilter>(address.clone()).unwrap())
                    }
                    _ => None,
                });
                let topics = None;
                Ok(LogsRequest {
                    fromBlock,
                    address: address_filter,
                    topics,
                    toBlock,
                })
            }
            Some(params) => unreachable!("{params:?}"),
            _ => Err(RpcErr::BadParams),
        }
    }
    // TODO: This is longer than it has the right to be, we should refactor it.
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
        let receipts: BTreeMap<BlockNumber, Receipt> =
            storage.get_receipts_in_range(from, to).unwrap();

        // Create address filter set
        let address_filter: BTreeSet<_> = match &self.address {
            Some(AddressFilter::Single(address)) => std::iter::once(address).collect(),
            Some(AddressFilter::Many(addresses)) => addresses.iter().collect(),
            None => BTreeSet::new(),
        };

        // To build the RPC response, we'll need some information besides the log,
        // these are:
        // - Each tx hash + index inside the block
        // - Each Block Number (and eventually, hash)
        // We'll collect this information as we process the receipts

        // For the RPC response, each log has to know its index globally
        // We'll use a global counter for log indices
        let mut global_log_index: u64 = 0;
        let mut all_logs: Vec<RpcLog> = Vec::new();

        for (block_number, block_receipts) in receipts {
            // Fetch block hash once per block
            let block_hash = storage
                .get_block_header(block_number)
                .ok()
                .flatten()
                .map(|header| header.compute_block_hash())
                .unwrap_or_default();

            for receipt in block_receipts {
                // Fetch transaction location for each receipt
                let Ok(Some((_, tx_index))) = storage.get_transaction_location(receipt.tx_hash)
                else {
                    continue;
                };

                for log in &receipt.logs {
                    if address_filter.is_empty() || address_filter.contains(&log.address) {
                        all_logs.push(RpcLog {
                            log: log.clone().into(),
                            log_index: global_log_index,
                            transaction_hash: receipt.tx_hash,
                            transaction_index: tx_index,
                            block_number,
                            block_hash,
                            removed: false,
                        });
                        global_log_index += 1;
                    }
                }
            }
        }

        // Sort logs by block number and then by log index
        all_logs.sort_by(|a, b| {
            a.block_number
                .cmp(&b.block_number)
                .then(a.log_index.cmp(&b.log_index))
        });

        // Serialize the logs to JSON, returning an error if serialization fails
        serde_json::to_value(all_logs).map_err(|_| RpcErr::Internal)
    }
}
