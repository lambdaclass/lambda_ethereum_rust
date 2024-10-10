use ethereum_rust_core::types::{BlockHeader, Transaction};
use ethereum_types::U256;

use crate::{
    block::BlockEnv,
    transaction::{TransactTo, TxEnv},
};

#[derive(Debug, Clone, Default)]
pub struct Env {
    pub tx_env: TxEnv,
    pub block_env: BlockEnv,
}

impl Env {
    pub fn new(tx: &Transaction, header: &BlockHeader) -> Self {
        let effective_gas_price: U256 = todo!();
        let priority_fee: U256 = todo!();
        let transact_to: TransactTo = todo!();

        // TODO: blob stuff

        let tx_env = TxEnv {
            caller: tx.sender(),
            gas_limit: tx.gas_limit(),
            effective_gas_price,
            priority_fee,
            transact_to,
            value: tx.value(),
            data: tx.data().clone(),
            nonce: tx.nonce(),
            chain_id: tx.chain_id(),
            access_list: None,           // TODO
            blob_hashes: Vec::default(), // TODO
            max_fee_per_blob_gas: None,  // TODO
        };

        let block_env = BlockEnv::default();

        Self { tx_env, block_env }
    }

    pub fn validate(&self) {
        todo!()
    }
}
