use crate::EvmError;
use ethereum_rust_core::{
    types::{Block, BlockHeader, Receipt, Transaction, TxKind},
    Address, U256,
};
use ethereum_rust_levm::{
    report::TransactionReport,
    vm::{Db, StorageSlot, VM},
};
use ethereum_rust_storage::Store;

pub trait Db {
    fn read_account_storage(&self, address: &Address, key: &U256) -> Option<StorageSlot>;

    fn write_account_storage(&mut self, address: &Address, key: U256, slot: StorageSlot);

    fn get_account_bytecode(&self, address: &Address) -> Bytes;

    fn balance(&mut self, address: &Address) -> U256;

    fn add_account(&mut self, address: Address, account: Account);

    fn increment_account_nonce(&mut self, address: &Address);

    /// Returns the account associated with the given address.
    /// If the account does not exist in the Db, it creates a new one with the given address.
    fn get_account(&mut self, address: &Address) -> Result<&Account, VMError>;
}

impl Db for Store {
    fn read_account_storage(&self, _address: &Address, _key: &U256) -> Option<StorageSlot> {
        // self.get_storage_at(block_number, address, key);
        // let _storage_slot = StorageSlot { original_value: todo!(), current_value: todo!(), is_cold: todo!() };
        todo!()
    }

    fn write_account_storage(&mut self, _address: &Address, _key: U256, _slot: StorageSlot) {
        todo!()
    }

    fn get_account_bytecode(&self, _address: &Address) -> bytes::Bytes {
        // self.get_code_by_account_address(block_number, address).unwrap().unwrap()
        todo!()
    }

    fn balance(&mut self, _address: &Address) -> U256 {
        // self.get_account_info(block_number, address).unwrap().unwrap().balance
        todo!()
    }

    fn add_account(&mut self, _address: Address, _account: ethereum_rust_levm::vm::Account) {
        todo!()
    }

    fn increment_account_nonce(&mut self, _address: &Address) {
        // self.apply_account_updates(block_hash, account_updates)
    }

    fn get_account(&mut self, _address: &Address) -> Result<&ethereum_rust_levm::vm::Account, ethereum_rust_levm::errors::VMError> {
        todo!()
    }
}

/// Executes all transactions in a block and returns their receipts.
pub fn execute_block(block: &Block, state: impl Db) -> Result<Vec<Receipt>, EvmError> {
    let mut receipts = Vec::new();
    let mut cumulative_gas_used = 0;
    for tx in block.body.transactions.iter() {
        let report = execute_tx(tx, &block.header, &state)?;
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
    state: &impl Db,
) -> Result<TransactionReport, EvmError> {
    let to = match tx.to() {
        TxKind::Call(address) => address,
        TxKind::Create => Address::zero(),
    };

    let mut vm = VM::new(
        to,
        tx.sender(),
        tx.value(),
        tx.data().clone(),
        block_header.gas_limit,
        tx.gas_limit().into(),
        block_header.number.into(),
        block_header.coinbase,
        block_header.timestamp.into(),
        Some(block_header.prev_randao),
        tx.chain_id().unwrap().into(),
        block_header.base_fee_per_gas.unwrap_or_default().into(), // TODO: check this
        tx.gas_price().into(),
        state,  // TODO: change this
        block_header.blob_gas_used.map(U256::from),
        block_header.excess_blob_gas.map(U256::from),
    );
    vm.transact()
        .map_err(|_| EvmError::Transaction("Levm error".to_string()))
}
