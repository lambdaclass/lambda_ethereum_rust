use crate::{db::StoreWrapper, EvmError};
use ethereum_rust_core::{
    types::{Account, AccountInfo, Block, BlockHeader, Receipt, Transaction, TxKind},
    Address, U256,
};
use ethereum_rust_levm::{
    report::TransactionReport,
    vm::{Account as LevmAccount, Db, StorageSlot, VM},
};
use ethereum_rust_storage::AccountUpdate;

impl Db for StoreWrapper {
    fn read_account_storage(&self, address: &Address, key: &U256) -> Option<StorageSlot> {
        let value = self.store
            .get_storage_at_hash(self.block_hash, address, key)
            .unwrap()
            .unwrap();
        let storage_slot = StorageSlot {
            original_value: value,
            current_value: value,
            is_cold: false,
        };
        Some(storage_slot)
    }

    fn write_account_storage(&mut self, _address: &Address, _key: U256, _slot: StorageSlot) {
        todo!()
    }

    fn get_account_bytecode(&self, address: &Address) -> bytes::Bytes {
        let block_number = self.store.get_block_number(self.block_hash);
        self.store.get_code_by_account_address(block_number, address).unwrap().unwrap()
    }

    fn balance(&mut self, address: &Address) -> U256 {
        let acc_info = self.store.get_account_info_by_hash(self.block_hash, address).unwrap().unwrap();
        acc_info.balance
    }

    fn add_account(&mut self, address: Address, account: LevmAccount) {
        // let new_acc = AccountUpdate {
        //     address,
        //     removed: false,
        //     info: Some(AccountInfo {
        //         code_hash: account.bytecode,
        //         balance: account.balance,
        //         nonce: account.nonce,
        //     }),
        //     code: Some(account.bytecode),
        //     added_storage: todo!(),
        // };
        // self.store.apply_account_updates(self.block_hash, account_updates)
        todo!()
    }

    fn increment_account_nonce(&mut self, _address: &Address) {
        // self.apply_account_updates(block_hash, account_updates)
        todo!()
    }

    fn get_account(
        &mut self,
        address: &Address,
    ) -> Result<&LevmAccount, ethereum_rust_levm::errors::VMError> {
        let block_number = self.store.get_block_number(self.block_hash);
        let acc_info = self.store.get_account_info_by_hash(self.block_hash, address).unwrap().unwrap();
        Ok(&LevmAccount {
            address,
            balance: acc_info.balance,
            bytecode: self.store.get_code_by_account_address(block_number, address),
            storage: StorageSlot {
                original_value: todo!(),
                current_value: todo!(),
                is_cold: todo!(),
            },
            nonce: acc_info.nonce,
        })
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
        state, // TODO: change this
        block_header.blob_gas_used.map(U256::from),
        block_header.excess_blob_gas.map(U256::from),
    );
    vm.transact()
        .map_err(|_| EvmError::Transaction("Levm error".to_string()))
}
