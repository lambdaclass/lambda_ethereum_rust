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
use std::collections::BTreeMap;

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
        let receipts = storage.get_receipts_in_range(from, to)?;

        // To build the RPC response, we'll need some information besides the log,
        // these are:
        // - Each tx hash + index inside the block
        // - Each Block Number (and eventually, hash)
        // This is what each triple of this vec represents:
        // - First coordinate is hash of a tx.
        // - Second coordinate is the index inside the tx.
        // - Third coordinate is the block number on which this tx was executed.
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

        // Since we'll also need the block hashes to build the RPC response,
        // we'll obtain them this by fetching every block header and then computing its hash.
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

        // For the RPC response, each log has to know its index inside the receipt
        // here we'll use a BlockNumber -> Vec<RpcLog> map, backed by a BTreeMap,
        // since we can take advantage the natural ordering of the BTreeMaps keys to avoid
        // some more boilerplate. Plus, at this key size, can be more cache-friendly
        // than a HashMap.
        let mut logs_by_block: BTreeMap<BlockNumber, Vec<RpcLog>> = BTreeMap::new();

        for (r_indx, receipt) in receipts.iter().enumerate() {
            let (tx_hash, tx_index, block_number) = tx_and_block_data[r_indx];
            let block_hash = block_hashes[r_indx];

            let block_logs = logs_by_block.entry(block_number).or_insert_with(Vec::new);
            let start_index = block_logs.len();

            block_logs.extend(
                receipt
                    .logs
                    .iter()
                    .enumerate()
                    .map(|(log_index, log)| RpcLog {
                        log: log.clone().into(),
                        log_index: (start_index + log_index) as u64,
                        transaction_hash: tx_hash,
                        transaction_index: tx_index,
                        block_number,
                        block_hash,
                        removed: false,
                    }),
            );
        }

        // Flatten the result, since the BTreeMap is orderd by block number,
        // the RPC response will be ordered by block number, as expected
        let logs: Vec<RpcLog> = logs_by_block.into_values().flatten().collect();

        // Serialize the logs to JSON, returning an error if serialization fails
        serde_json::to_value(logs).map_err(|_| RpcErr::Internal)
    }
}
