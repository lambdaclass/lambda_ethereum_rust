use std::collections::HashMap;

use ethereum_rust_core::{types::AccountInfo, Address, BigEndianHash, H256, U256};
use ethereum_rust_storage::AccountUpdate;
use revm::{
    db::AccountStatus as RevmAccounStatus,
    primitives::{
        AccountInfo as RevmAccountInfo, Address as RevmAddress, Bytecode as RevmBytecode,
        B256 as RevmB256, U256 as RevmU256,
    },
    DatabaseRef,
};
use serde::{Deserialize, Serialize};

use crate::errors::ExecutionDBError;

/// In-memory EVM database for caching execution data.
///
/// This is mainly used to store the relevant state data for executing a particular block and then
/// feeding the DB into a zkVM program to prove the execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionDB {
    /// indexed by account address
    accounts: HashMap<RevmAddress, RevmAccountInfo>,
    /// indexed by code hash
    code: HashMap<RevmB256, RevmBytecode>,
    /// indexed by account address and storage slot
    storage: HashMap<RevmAddress, HashMap<RevmU256, RevmU256>>,
    /// indexed by block number
    block_hashes: HashMap<u64, RevmB256>,
}

impl DatabaseRef for ExecutionDB {
    /// The database error type.
    type Error = ExecutionDBError;

    /// Get basic account information.
    fn basic_ref(&self, address: RevmAddress) -> Result<Option<RevmAccountInfo>, Self::Error> {
        Ok(self.accounts.get(&address).cloned())
    }

    /// Get account code by its hash.
    fn code_by_hash_ref(&self, code_hash: RevmB256) -> Result<RevmBytecode, Self::Error> {
        self.code
            .get(&code_hash)
            .cloned()
            .ok_or(ExecutionDBError::CodeNotFound(code_hash))
    }

    /// Get storage value of address at index.
    fn storage_ref(&self, address: RevmAddress, index: RevmU256) -> Result<RevmU256, Self::Error> {
        self.storage
            .get(&address)
            .ok_or(ExecutionDBError::AccountNotFound(address))?
            .get(&index)
            .cloned()
            .ok_or(ExecutionDBError::StorageNotFound(address, index))
    }

    /// Get block hash by block number.
    fn block_hash_ref(&self, number: u64) -> Result<RevmB256, Self::Error> {
        self.block_hashes
            .get(&number)
            .cloned()
            .ok_or(ExecutionDBError::BlockHashNotFound(number))
    }
}

impl ExecutionDB {
    /// From [crate::get_state_transitions].
    pub fn get_state_transitions(&self) -> Vec<AccountUpdate> {
        let mut db = revm::db::State::builder().with_database_ref(self).build();
        db.merge_transitions(revm::db::states::bundle_state::BundleRetention::PlainState);
        let bundle = db.take_bundle();
        // Update accounts
        let mut account_updates = Vec::new();
        for (address, account) in bundle.state() {
            if account.status.is_not_modified() {
                continue;
            }
            let address = Address::from_slice(address.0.as_slice());
            // Remove account from DB if destroyed (Process DestroyedChanged as changed account)
            if matches!(
                account.status,
                RevmAccounStatus::Destroyed | RevmAccounStatus::DestroyedAgain
            ) {
                account_updates.push(AccountUpdate::removed(address));
                continue;
            }

            // If account is empty, do not add to the database
            if account
                .account_info()
                .is_some_and(|acc_info| acc_info.is_empty())
            {
                continue;
            }

            // Apply account changes to DB
            let mut account_update = AccountUpdate::new(address);
            // If the account was changed then both original and current info will be present in the bundle account
            if account.is_info_changed() {
                // Update account info in DB
                if let Some(new_acc_info) = account.account_info() {
                    let code_hash = H256::from_slice(new_acc_info.code_hash.as_slice());
                    let account_info = AccountInfo {
                        code_hash,
                        balance: U256::from_little_endian(new_acc_info.balance.as_le_slice()),
                        nonce: new_acc_info.nonce,
                    };
                    account_update.info = Some(account_info);
                    if account.is_contract_changed() {
                        // Update code in db
                        if let Some(code) = new_acc_info.code {
                            account_update.code = Some(code.original_bytes().clone().0);
                        }
                    }
                }
            }
            // Update account storage in DB
            for (key, slot) in account.storage.iter() {
                if slot.is_changed() {
                    // TODO check if we need to remove the value from our db when value is zero
                    // if slot.present_value().is_zero() {
                    //     account_update.removed_keys.push(H256::from_uint(&U256::from_little_endian(key.as_le_slice())))
                    // }
                    account_update.added_storage.insert(
                        H256::from_uint(&U256::from_little_endian(key.as_le_slice())),
                        U256::from_little_endian(slot.present_value().as_le_slice()),
                    );
                }
            }
            account_updates.push(account_update)
        }
        account_updates
    }
}
