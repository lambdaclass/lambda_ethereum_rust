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
        let tx_and_block_data = {
            let mut data: Vec<(H256, Index, BlockNumber)> = vec![];
            for receipt in receipts.clone() {
                let tx_hash = receipt.tx_hash.clone();
                let (block_num, tx_indx) =
                    storage.get_transaction_location(tx_hash).unwrap().unwrap();
                data.push((tx_hash, tx_indx, block_num))
            }
            data
        };
        let block_hashes: Vec<H256> = {
            let mut hashes: Vec<H256> = vec![];
            for (_, _, block_num) in tx_and_block_data.clone().into_iter() {
                let block_hash = storage
                    .get_block_header(block_num)
                    .unwrap()
                    .unwrap()
                    .compute_block_hash();
                hashes.push(block_hash)
            }
            hashes
        };
        let mut logs: Vec<RpcLog> = vec![];
        for (r_indx, receipt) in receipts.iter().enumerate() {
            for (log_indx, log) in receipt.logs.iter().enumerate() {
                let log = RpcLog {
                    log: log.clone().into(),
                    log_index: log_indx as u64,
                    transaction_hash: tx_and_block_data[r_indx].0,
                    transaction_index: tx_and_block_data[r_indx].1,
                    block_number: tx_and_block_data[r_indx].2,
                    block_hash: block_hashes[r_indx],
                    removed: false,
                };
                logs.push(log)
            }
        }
        serde_json::to_value(logs).map_err(|_| RpcErr::Internal)
    }
}
