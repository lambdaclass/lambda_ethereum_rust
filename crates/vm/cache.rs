use revm::{
    primitives::{
        AccountInfo as RevmAccountInfo, Address as RevmAddress, Bytecode as RevmBytecode,
        Bytes as RevmBytes, B256 as RevmB256, U256 as RevmU256,
    },
    DatabaseRef,
};
use serde::{Deserialize, Serialize};

use crate::errors::ExecutionDBError;

/// In-memory EVM database for cached execution data.
///
/// This is mainly used to store the relevant state data for executing a block and then feeding
/// the DB into a zkVM program to prove the execution.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ExecutionDB {}

impl DatabaseRef for ExecutionDB {
    /// The database error type.
    type Error = ExecutionDBError;

    /// Get basic account information.
    fn basic_ref(&self, address: RevmAddress) -> Result<Option<RevmAccountInfo>, Self::Error> {
        todo!()
    }

    /// Get account code by its hash.
    fn code_by_hash_ref(&self, code_hash: RevmB256) -> Result<RevmBytecode, Self::Error> {
        todo!()
    }

    /// Get storage value of address at index.
    fn storage_ref(&self, address: RevmAddress, index: RevmU256) -> Result<RevmU256, Self::Error> {
        todo!()
    }

    /// Get block hash by block number.
    fn block_hash_ref(&self, number: u64) -> Result<RevmB256, Self::Error> {
        todo!()
    }
}
