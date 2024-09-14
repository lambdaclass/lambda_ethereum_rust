use crate::{
    eth::block::block_number,
    types::{block_identifier::BlockIdentifier, receipt::RpcLog},
    RpcErr, RpcHandler,
};
use ethereum_rust_core::{
    types::{BlockNumber, Index},
    H160, H256, U256,
};
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
        let Ok(Some(from)) = self.fromBlock.resolve_block_number(&storage) else {
            return Err(RpcErr::BadParams);
        };
        let Ok(Some(to)) = self.toBlock.resolve_block_number(&storage) else {
            return Err(RpcErr::BadParams);
        };
        let receipts = storage.get_receipts_in_range(from, to)?;
        // Process transaction and block data for eth_getLogs response
        use std::collections::HashMap;

        // Process transaction and block data for eth_getLogs response
        let tx_and_block_data: Vec<(H256, Index, BlockNumber)> = receipts
            .iter()
            .filter_map(|receipt| {
                let tx_hash = receipt.tx_hash;
                storage
                    .get_transaction_location(tx_hash)
                    .ok()
                    .flatten()
                    .map(|(block_num, tx_indx)| (tx_hash, tx_indx, block_num))
            })
            .collect();

        // Retrieve block hashes for each transaction
        let block_hashes: Vec<H256> = tx_and_block_data
            .iter()
            .filter_map(|(_, _, block_num)| {
                storage
                    .get_block_header(*block_num)
                    .ok()
                    .flatten()
                    .map(|header| header.compute_block_hash())
            })
            .collect();

        // Group logs by block number
        let mut logs_by_block: HashMap<BlockNumber, Vec<RpcLog>> = HashMap::new();

        for (r_indx, receipt) in receipts.iter().enumerate() {
            let tx_data = &tx_and_block_data[r_indx];
            let block_hash = block_hashes[r_indx];

            for (_, log) in receipt.logs.iter().enumerate() {
                let rpc_log = RpcLog {
                    log: log.clone().into(),
                    // Ignore this value for now, we'll change it below...
                    log_index: 0,
                    transaction_hash: tx_data.0,
                    transaction_index: tx_data.1,
                    block_number: tx_data.2,
                    block_hash,
                    removed: false,
                };
                logs_by_block
                    .entry(tx_data.2)
                    .or_insert_with(Vec::new)
                    .push(rpc_log);
            }
        }

        let mut logs: Vec<RpcLog> = Vec::new();
        for (_, block_logs) in logs_by_block.iter_mut() {
            for (index, log) in block_logs.iter_mut().enumerate() {
                // Set properly the index
                log.log_index = index as u64;
            }
            logs.extend(block_logs.drain(..));
        }

        // Sort logs by block number, then by log index
        logs.sort_by(|a, b| {
            a.block_number
                .cmp(&b.block_number)
                .then(a.log_index.cmp(&b.log_index))
        });

        // Serialize the logs to JSON, returning an error if serialization fails
        serde_json::to_value(logs).map_err(|_| RpcErr::Internal)
    }
}
