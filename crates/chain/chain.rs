pub mod constants;
pub mod error;
use constants::{GAS_PER_BLOB, MAX_BLOB_GAS_PER_BLOCK, MAX_BLOB_NUMBER_PER_BLOCK};
use error::{ChainError, InvalidBlockError};
use ethereum_rust_core::types::{
    validate_block_header, validate_cancun_header_fields, validate_no_cancun_header_fields, Block,
    BlockHeader, EIP4844Transaction, Transaction,
};
use ethereum_rust_core::H256;

use ethereum_rust_evm::{
    apply_state_transitions, evm_state, execute_block, spec_id, EvmState, SpecId,
};
use ethereum_rust_storage::error::StoreError;
use ethereum_rust_storage::Store;

/// Adds a new block as head of the chain.
/// Performs pre and post execution validation, and updates the database.
pub fn add_block(block: &Block, storage: Store) -> Result<(), ChainError> {
    //
    let latest_block_number = storage
        .get_latest_block_number()
        .map_err(|e| ChainError::StoreError(e))?;
    if latest_block_number.is_some_and(|number| block.header.number > number.saturating_add(1)) {
        return Err(ChainError::NonCanonicalBlock);
    }
    // Validate if it can be the new head and find the parent
    let parent_header = find_parent_header(block, &storage)?;

    let mut state = evm_state(storage.clone());

    // Validate the block pre-execution
    validate_block(block, &parent_header, &state)?;

    execute_block(block, &mut state).map_err(|e| ChainError::EvmError(e))?;

    apply_state_transitions(&mut state).map_err(|e| ChainError::StoreError(e))?;

    // Check state root matches the one in block header after execution
    validate_state_root(&block.header, storage.clone())?;

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
pub fn validate_state_root(block_header: &BlockHeader, storage: Store) -> Result<(), ChainError> {
    // Compare state root
    if storage.world_state_root() == block_header.state_root {
        Ok(())
    } else {
        return Err(ChainError::InvalidBlock(
            InvalidBlockError::StateRootMismatch,
        ));
    }
}

pub fn latest_valid_hash(storage: &Store) -> Result<H256, ChainError> {
    if let Some(latest_block_number) = storage
        .get_latest_block_number()
        .map_err(|e| ChainError::StoreError(e))?
    {
        if let Some(latest_valid_header) = storage
            .get_block_header(latest_block_number)
            .map_err(|e| ChainError::StoreError(e))?
        {
            let latest_valid_hash = latest_valid_header.compute_block_hash();
            return Ok(latest_valid_hash);
        }
    }
    Err(ChainError::StoreError(StoreError::Custom(
        "Could not find latest valid hash".to_string(),
    )))
}

/// Validates if the provided block could be the new head of the chain, and returns the
/// parent_header in that case
fn find_parent_header(block: &Block, storage: &Store) -> Result<BlockHeader, ChainError> {
    let parent_hash = block.header.parent_hash;
    let parent_number = storage
        .get_block_number(parent_hash)
        .map_err(|e| ChainError::StoreError(e))?;

    if let Some(parent_number) = parent_number {
        let parent_header = storage
            .get_block_header(parent_number)
            .map_err(|e| ChainError::StoreError(e))?;

        if let Some(parent_header) = parent_header {
            Ok(parent_header)
        } else {
            Err(ChainError::ParentNotFound)
        }
    } else {
        Err(ChainError::ParentNotFound)
    }
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
