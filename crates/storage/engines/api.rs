use std::fmt::Debug;

use bytes::Bytes;
use ethereum_types::{Address, H256, U256};

use ethereum_rust_core::types::{
    Account, AccountInfo, BlockBody, BlockHash, BlockHeader, BlockNumber, Index, Receipt,
    Transaction,
};

use crate::error::StoreError;

pub trait StoreEngine: Debug + Send {
    /// Add account info
    fn add_account_info(
        &mut self,
        address: Address,
        account_info: AccountInfo,
    ) -> Result<(), StoreError>;

    /// Obtain account info
    fn get_account_info(&self, address: Address) -> Result<Option<AccountInfo>, StoreError>;

    /// Remove account info
    fn remove_account_info(&mut self, address: Address) -> Result<(), StoreError>;

    /// Add block header
    fn add_block_header(
        &mut self,
        block_number: BlockNumber,
        block_header: BlockHeader,
    ) -> Result<(), StoreError>;

    /// Obtain block header
    fn get_block_header(
        &self,
        block_number: BlockNumber,
    ) -> Result<Option<BlockHeader>, StoreError>;

    /// Add block body
    fn add_block_body(
        &mut self,
        block_number: BlockNumber,
        block_body: BlockBody,
    ) -> Result<(), StoreError>;

    /// Obtain block body
    fn get_block_body(&self, block_number: BlockNumber) -> Result<Option<BlockBody>, StoreError>;

    /// Add block body
    fn add_block_number(
        &mut self,
        block_hash: BlockHash,
        block_number: BlockNumber,
    ) -> Result<(), StoreError>;

    /// Obtain block number
    fn get_block_number(&self, block_hash: BlockHash) -> Result<Option<BlockNumber>, StoreError>;

    /// Store transaction location (block number and index of the transaction within the block)
    fn add_transaction_location(
        &mut self,
        transaction_hash: H256,
        block_number: BlockNumber,
        index: Index,
    ) -> Result<(), StoreError>;

    /// Obtain transaction location (block number and index)
    fn get_transaction_location(
        &self,
        transaction_hash: H256,
    ) -> Result<Option<(BlockNumber, Index)>, StoreError>;

    /// Add receipt
    fn add_receipt(
        &mut self,
        block_number: BlockNumber,
        index: Index,
        receipt: Receipt,
    ) -> Result<(), StoreError>;

    /// Obtain receipt
    fn get_receipt(
        &self,
        block_number: BlockNumber,
        index: Index,
    ) -> Result<Option<Receipt>, StoreError>;

    /// Add account code
    fn add_account_code(&mut self, code_hash: H256, code: Bytes) -> Result<(), StoreError>;

    /// Obtain account code via code hash
    fn get_account_code(&self, code_hash: H256) -> Result<Option<Bytes>, StoreError>;

    /// Obtain account code via account address
    fn get_code_by_account_address(&self, address: Address) -> Result<Option<Bytes>, StoreError> {
        let code_hash = match self.get_account_info(address)? {
            Some(acc_info) => acc_info.code_hash,
            None => return Ok(None),
        };
        self.get_account_code(code_hash)
    }

    fn get_transaction_by_hash(
        &self,
        transaction_hash: H256,
    ) -> Result<Option<Transaction>, StoreError> {
        let (block_number, index) = match self.get_transaction_location(transaction_hash)? {
            Some(locations) => locations,
            None => return Ok(None),
        };
        let block_body = match self.get_block_body(block_number)? {
            Some(body) => body,
            None => return Ok(None),
        };
        Ok(index
            .try_into()
            .ok()
            .and_then(|index: usize| block_body.transactions.get(index).cloned()))
    }

    // Add storage value
    fn add_storage_at(
        &mut self,
        address: Address,
        storage_key: H256,
        storage_value: H256,
    ) -> Result<(), StoreError>;

    // Obtain storage value
    fn get_storage_at(
        &self,
        address: Address,
        storage_key: H256,
    ) -> Result<Option<H256>, StoreError>;

    // Add storage value
    fn remove_account_storage(&mut self, address: Address) -> Result<(), StoreError>;

    /// Stores account in db (including info, code & storage)
    fn add_account(&mut self, address: Address, account: Account) -> Result<(), StoreError> {
        self.add_account_info(address, account.info.clone())?;
        self.add_account_code(account.info.code_hash, account.code)?;
        for (storage_key, storage_value) in account.storage {
            self.add_storage_at(address, storage_key, storage_value)?;
        }
        Ok(())
    }

    /// Removes account info and storage
    fn remove_account(&mut self, address: Address) -> Result<(), StoreError> {
        self.remove_account_info(address)?;
        self.remove_account_storage(address)
    }

    /// Increments the balance of an account by a given ammount (if it exists)
    fn increment_balance(&mut self, address: Address, amount: U256) -> Result<(), StoreError> {
        if let Some(mut account_info) = self.get_account_info(address)? {
            account_info.balance = account_info.balance.saturating_add(amount);
            self.add_account_info(address, account_info)?;
        }
        Ok(())
    }

    /// Updates the value of the chain id
    fn update_chain_id(&mut self, chain_id: U256) -> Result<(), StoreError>;

    /// Obtain the current chain id
    fn get_chain_id(&self) -> Result<Option<U256>, StoreError>;

    /// Updates the value of the timestamp at which the cancun fork was activated
    fn update_cancun_time(&mut self, cancun_time: u64) -> Result<(), StoreError>;

    /// Obtain the timestamp at which the cancun fork was activated
    fn get_cancun_time(&self) -> Result<Option<u64>, StoreError>;
}
