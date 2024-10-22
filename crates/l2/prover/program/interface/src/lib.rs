include!(concat!(env!("OUT_DIR"), "/methods.rs"));

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use ethereum_rust_storage::error::StoreError;
use revm::primitives::{
    db::DatabaseRef, result::EVMError as RevmError, AccountInfo as RevmAccountInfo,
    Address as RevmAddress, Bytecode as RevmBytecode, B256 as RevmB256, KECCAK_EMPTY,
    U256 as RevmU256,
};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ExecutionDBError {
    #[error("Account {0} not found")]
    AccountNotFound(RevmAddress),
    #[error("Code by hash {0} not found")]
    CodeNotFound(RevmB256),
    #[error("Storage value for address {0} and slot {1} not found")]
    StorageNotFound(RevmAddress, RevmU256),
    #[error("Hash of block with number {0} not found")]
    BlockHashNotFound(u64),
    #[error("{0}")]
    Custom(String),
}

#[derive(Debug, Error)]
pub enum EvmError {
    #[error("Invalid Transaction: {0}")]
    Transaction(String),
    #[error("Invalid Header: {0}")]
    Header(String),
    #[error("DB error: {0}")]
    DB(#[from] StoreError),
    #[error("Execution DB error: {0}")]
    ExecutionDB(#[from] ExecutionDBError),
    #[error("{0}")]
    Custom(String),
    #[error("{0}")]
    Precompile(String),
}

impl From<RevmError<ExecutionDBError>> for EvmError {
    fn from(value: RevmError<ExecutionDBError>) -> Self {
        match value {
            RevmError::Transaction(err) => EvmError::Transaction(err.to_string()),
            RevmError::Header(err) => EvmError::Header(err.to_string()),
            RevmError::Database(err) => EvmError::ExecutionDB(err),
            RevmError::Custom(err) => EvmError::Custom(err),
            RevmError::Precompile(err) => EvmError::Precompile(err),
        }
    }
}

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

impl Default for ExecutionDB {
    fn default() -> Self {
        let mut accounts = HashMap::new();
        let mut storage = HashMap::new();
        let mut code = HashMap::new();
        let mut block_hashes = HashMap::new();

        // Insert default accounts
        accounts.insert(RevmAddress::default(), RevmAccountInfo::default());

        // Insert default storage
        let mut default_storage = HashMap::new();
        default_storage.insert(RevmU256::from(0), RevmU256::from(0));
        storage.insert(RevmAddress::default(), default_storage);

        // Insert defualt code
        code.insert(KECCAK_EMPTY, RevmBytecode::default());

        // Insert a default block hash
        block_hashes.insert(0_u64, KECCAK_EMPTY);

        ExecutionDB {
            accounts,
            storage,
            code,
            block_hashes,
        }
    }
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
