use ethereum_rust_core::{
    serde_utils,
    types::{BlockHash, BlockNumber, Transaction},
    Address, H256,
};
use serde::Serialize;

#[allow(unused)]
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcTransaction {
    #[serde(flatten)]
    tx: Transaction,
    #[serde(with = "serde_utils::u64::hex_str")]
    block_number: BlockNumber,
    block_hash: BlockHash,
    from: Address,
    hash: H256,
    #[serde(with = "serde_utils::u64::hex_str")]
    transaction_index: u64,
}

impl RpcTransaction {
    pub fn build(
        tx: Transaction,
        block_number: BlockNumber,
        block_hash: BlockHash,
        transaction_index: usize,
    ) -> Self {
        let from = tx.sender();
        let hash = tx.compute_hash();
        let transaction_index = transaction_index as u64;
        RpcTransaction {
            tx,
            block_number,
            block_hash,
            from,
            hash,
            transaction_index,
        }
    }
}
