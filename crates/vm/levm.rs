use crate::EvmError;
use bytes::Bytes;
use ethereum_rust_core::{
    types::{Block, BlockHeader, Receipt, Transaction},
    Address,
};
use ethereum_rust_levm::{
    report::TransactionReport,
    vm::{Db, VM},
};

pub struct EvmState(Db);

impl EvmState {
    /// Get a reference to inner `Store` database
    pub fn database(&self) -> &Db {
        &self.0
    }
}

/// Executes all transactions in a block and returns their receipts.
pub fn execute_block(block: &Block, state: &mut EvmState) -> Result<Vec<Receipt>, EvmError> {
    let mut receipts = Vec::new();
    let mut cumulative_gas_used = 0;
    for tx in block.body.transactions.iter() {
        let report = execute_tx(tx, &block.header, state)?;
        cumulative_gas_used += report.gas_used;
        let receipt = Receipt::new(
            tx.tx_type(),
            report.is_success(),
            cumulative_gas_used,
            report.logs,
        );
        receipts.push(receipt);
    }
    Ok(receipts)
}

// Executes a single tx, doesn't perform state transitions
pub fn execute_tx(
    tx: &Transaction,
    block_header: &BlockHeader,
    state: &mut EvmState,
) -> Result<TransactionReport, EvmError> {
    // TODO: check all the parameters
    let mut vm = VM::new(
        Address::random(),
        tx.sender(),
        tx.value(),
        Bytes::new(),
        block_header.gas_limit,
        tx.gas_limit().into(),
        block_header.number.into(),
        block_header.coinbase,
        block_header.timestamp.into(),
        Some(block_header.prev_randao),
        tx.chain_id().unwrap().into(),
        block_header.base_fee_per_gas.unwrap().into(),
        tx.gas_price().into(),
        state.database().clone(), // shouldn't clone here
    );
    vm.transact()
        .map_err(|_| EvmError::Transaction("Levm error".to_string()))
}
