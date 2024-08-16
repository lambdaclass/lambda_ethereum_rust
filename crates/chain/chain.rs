pub mod constants;
use constants::{GAS_PER_BLOB, MAX_BLOB_GAS_PER_BLOCK, MAX_BLOB_NUMBER_PER_BLOCK};
use ethereum_rust_core::types::{
    validate_block_header, validate_cancun_header_fields, validate_no_cancun_header_fields, Block,
    BlockHeader, EIP4844Transaction, Transaction,
};

use ethereum_rust_evm::{
    apply_state_transitions, evm_state, execute_block, spec_id, EvmError, EvmState, SpecId,
};
use ethereum_rust_storage::error::StoreError;
use ethereum_rust_storage::Store;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ChainError {
    #[error("Invalid Block: {0}")]
    InvalidBlock(InvalidBlockError),
    #[error("Parent block not found (block with block_number-1)")]
    ParentNotFound,
    //TODO: If a block with block_number greater than latest plus one is received
    //maybe we are missing data and should wait for syncing
    #[error("Block number is greater than the latest plus one")]
    GreaterThanLatestPlusOne,

    //TODO: If a block with less than latest plus one is received we should reorg
    #[error("Block number is less than the latest plus one")]
    LessThanLatestPlusOne,
    #[error("Previous block's hash does not match parent_hash in header")]
    ParentHashMismatch,
    #[error("DB error: {0}")]
    StoreError(StoreError),
    #[error("EVM error: {0}")]
    EvmError(EvmError),
}

#[derive(Debug, Error)]
pub enum InvalidBlockError {
    #[error("World State Root does not match the one in the header after executing")]
    StateRootMismatch,
    #[error("Invalid Header, validation failed pre-execution")]
    InvalidHeader,
    #[error("Exceeded MAX_BLOB_GAS_PER_BLOCK")]
    ExceededMaxBlobGasPerBlock,
    #[error("Exceeded MAX_BLOB_NUMBER_PER_BLOCK")]
    ExceededMaxBlobNumberPerBlock,
    #[error("blob gas used doesn't match value in header")]
    BlobGasUsedMismatch,
}

/// Adds a new block as head of the chain.
/// Performs pre and post execution validation, and updates the database.
pub fn add_block(block: &Block, storage: Store) -> Result<(), ChainError> {
    // Validate if it can be the new head and find the parent
    let parent_header = find_parent_header(block, &storage)?;

    let mut state = evm_state(storage.clone());

    // Validate the block pre-execution
    validate_block(block, &parent_header, &state)?;

    execute_block(block, &mut state).map_err(|e| ChainError::EvmError(e))?;

    apply_state_transitions(&mut state).map_err(|e| ChainError::StoreError(e))?;

    // Check state root matches the one in block header after execution
    validate_state(&block.header, storage.clone())?;

    store_block(storage.clone(), block.clone())?;

    Ok(())
}

/// Stores block and header in the database
pub fn store_block(storage: Store, block: Block) -> Result<(), ChainError> {
    storage
        .add_block(block.clone())
        .map_err(|e| ChainError::StoreError(e))?;
    storage
        .update_latest_block_number(block.header.number)
        .map_err(|e| ChainError::StoreError(e))?;
    Ok(())
}

/// Performs post-execution checks
pub fn validate_state(block_header: &BlockHeader, storage: Store) -> Result<(), ChainError> {
    // Compare state root
    if storage.world_state_root() == block_header.state_root {
        Ok(())
    } else {
        return Err(ChainError::InvalidBlock(
            InvalidBlockError::StateRootMismatch,
        ));
    }
}

/// Validates if the provided block could be the new head of the chain, and returns the
/// parent_header in that case.
fn find_parent_header(block: &Block, storage: &Store) -> Result<BlockHeader, ChainError> {
    let block_number = block.header.number;

    let last_block_number = storage
        .get_latest_block_number()
        .map_err(|e| ChainError::StoreError(e))?
        .unwrap();

    if block_number > last_block_number.saturating_add(1) {
        return Err(ChainError::GreaterThanLatestPlusOne);
    }
    if block_number < last_block_number.saturating_add(1) {
        return Err(ChainError::LessThanLatestPlusOne);
    }

    // Fetch the block header with previous number
    let parent_header_result = storage
        .get_block_header(block_number.saturating_sub(1))
        .map_err(|e| ChainError::StoreError(e))?;
    let parent_header = match parent_header_result {
        Some(parent_header) => {
            if parent_header.compute_block_hash() != block.header.parent_hash {
                return Err(ChainError::ParentHashMismatch);
            }
            parent_header
        }
        None => {
            return Err(ChainError::ParentNotFound);
        }
    };
    Ok(parent_header)
}

/// Performs pre-execution validation of the block's header values in reference to the parent_header
/// Verifies that blob gas fields in the header are correct in reference to the block's body.
/// If a block passes this check, execution will still fail with execute_block when a transaction runs out of gas
pub fn validate_block(
    block: &Block,
    parent_header: &BlockHeader,
    state: &EvmState,
) -> Result<(), ChainError> {
    let spec = spec_id(state.database(), block.header.timestamp).unwrap();

    // Verify initial header validity against parent
    let mut valid_header = validate_block_header(&block.header, parent_header);

    valid_header = match spec {
        SpecId::CANCUN => {
            valid_header && validate_cancun_header_fields(&block.header, parent_header)
        }
        _ => valid_header && validate_no_cancun_header_fields(&block.header),
    };
    if !valid_header {
        return Err(ChainError::InvalidBlock(InvalidBlockError::InvalidHeader));
    }

    if spec == SpecId::CANCUN {
        verify_blob_gas_usage(block)?
    }
    Ok(())
}

fn verify_blob_gas_usage(block: &Block) -> Result<(), ChainError> {
    let mut blob_gas_used = 0_u64;
    let mut blobs_in_block = 0_u64;
    for transaction in block.body.transactions.iter() {
        if let Transaction::EIP4844Transaction(tx) = transaction {
            blob_gas_used += get_total_blob_gas(tx);
            blobs_in_block += tx.blob_versioned_hashes.len() as u64;
        }
    }
    if blob_gas_used > MAX_BLOB_GAS_PER_BLOCK {
        return Err(ChainError::InvalidBlock(
            InvalidBlockError::ExceededMaxBlobGasPerBlock,
        ));
    }
    if blobs_in_block > MAX_BLOB_NUMBER_PER_BLOCK {
        return Err(ChainError::InvalidBlock(
            InvalidBlockError::ExceededMaxBlobNumberPerBlock,
        ));
    }
    if blob_gas_used != block.header.blob_gas_used.unwrap() {
        return Err(ChainError::InvalidBlock(
            InvalidBlockError::BlobGasUsedMismatch,
        ));
    }
    Ok(())
}

/// Calculates the blob gas required by a transaction
fn get_total_blob_gas(tx: &EIP4844Transaction) -> u64 {
    GAS_PER_BLOB * tx.blob_versioned_hashes.len() as u64
}

#[cfg(test)]
mod tests {}
