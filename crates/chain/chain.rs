use ethereum_rust_core::types::{Block, BlockHeader};
use ethereum_rust_evm::{evm_state, execute_block, validate_block, EvmError};
use ethereum_rust_storage::error::StoreError;
use ethereum_rust_storage::Store;

pub enum ChainResult {
    InsertedBlock(String),
}
pub enum ChainError {
    RejectedBlock(String),
    StoreError(StoreError),
    EvmError(EvmError),
}

//TODO: Move validate_block and execute_block functions from evm crate into this crate
//      Those functions should also be refactored to return our own results and errors instead of
//      revm generic errors, empty results, or booleans.

//TODO: execute_block function should not have the responsability of updating the database.

//TODO: execute_block should return a result with some kind of execution receipts to validate
//      against the block header, for example we should be able to know how much gas was used
//      in the block execution to validate the gas_used field.

/// Adds a new block as head of the chain.
/// Performs pre and post execution validation, and updates the database.
pub fn add_block(block: &Block, storage: Store) -> Result<ChainResult, ChainError> {
    let parent_header = find_parent_header(block, &storage)?;
    let mut state = evm_state(storage);

    let valid_block = validate_block(block, &parent_header, &state);

    if !valid_block {
        return Err(ChainError::RejectedBlock(
            "Failed to validate block".to_string(),
        ));
    }

    execute_block(block, &mut state).map_err(|e| ChainError::EvmError(e))?;

    Ok(ChainResult::InsertedBlock("ok!".to_string()))
}

/// Validates if the provided block could be the new head of the chain, and returns the
/// parent_header in that case.
fn find_parent_header(block: &Block, storage: &Store) -> Result<BlockHeader, ChainError> {
    let block_number = block.header.number;
    let last_block_number = storage
        .get_latest_block_number()
        .map_err(|e| ChainError::StoreError(e))?
        .unwrap();
    if block_number != last_block_number.saturating_add(1) {
        return Err(ChainError::RejectedBlock(
            "Block is not the latest block".to_string(),
        ));
    }

    // Fetch the block header with previous number
    let parent_header_result = storage.get_block_header(block_number.saturating_sub(1));

    let parent_header = match parent_header_result {
        Ok(Some(parent_header)) => {
            if parent_header.compute_block_hash() != block.header.parent_hash {
                return Err(ChainError::RejectedBlock(
                    "Parent hash doesn't match block found in store".to_string(),
                ));
            }
            parent_header
        }
        _ => {
            return Err(ChainError::RejectedBlock(
                "Parent block not found in store (invalid block number)".to_string(),
            ));
        }
    };
    Ok(parent_header)
}

#[cfg(test)]
mod tests {}
