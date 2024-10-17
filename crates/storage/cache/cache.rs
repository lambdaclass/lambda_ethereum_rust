use error::MemoryDBError;
use revm::{
    primitives::{AccountInfo, Address, Bytecode, B256, U256},
    DatabaseRef,
};
use serde::{Deserialize, Serialize};

pub mod error;

/// In-memory EVM database for cached execution data.
///
/// This is mainly used for storing the relevant state data for executing a block and then feeding
/// the DB into a zkVM program to prove the execution.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct MemoryDB {}

impl DatabaseRef for MemoryDB {
    /// The database error type.
    type Error = MemoryDBError;

    /// Get basic account information.
    fn basic_ref(&self, address: Address) -> Result<Option<AccountInfo>, Self::Error> {
        todo!()
    }

    /// Get account code by its hash.
    fn code_by_hash_ref(&self, code_hash: B256) -> Result<Bytecode, Self::Error> {
        todo!()
    }

    /// Get storage value of address at index.
    fn storage_ref(&self, address: Address, index: U256) -> Result<U256, Self::Error> {
        todo!()
    }

    /// Get block hash by block number.
    fn block_hash_ref(&self, number: u64) -> Result<B256, Self::Error> {
        todo!()
    }
}
