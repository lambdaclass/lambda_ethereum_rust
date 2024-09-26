pub mod constants;
pub mod error;
pub mod mempool;
pub mod payload;

use constants::{GAS_PER_BLOB, MAX_BLOB_GAS_PER_BLOCK, MAX_BLOB_NUMBER_PER_BLOCK};
use error::{ChainError, InvalidBlockError};
use ethereum_rust_core::types::{
    validate_block_header, validate_cancun_header_fields, validate_no_cancun_header_fields, Block,
    BlockHash, BlockHeader, EIP4844Transaction, Receipt, Transaction,
};
use ethereum_rust_core::H256;

use ethereum_rust_evm::{
    evm_state, execute_block, get_state_transitions, spec_id, EvmState, SpecId,
};
use ethereum_rust_storage::error::StoreError;
use ethereum_rust_storage::Store;

//TODO: Implement a struct Chain or BlockChain to encapsulate
//functionality and canonical chain state and config

/// Adds a new block to the store. It may or may not be canonical, as long as its parent is part
/// of the canonical chain. It doesn't modify the canonical chain/head.
///
/// Performs pre and post execution validation, and updates the database with the post state.
pub fn add_block(block: &Block, storage: &Store) -> Result<(), ChainError> {
    // TODO(#444): handle cases where the canonical chain is indirectly connected to the block.
    // TODO(#438): handle cases where blocks are missing between the canonical chain and the block.
    validate_parent_canonical(block, storage)?;

    // Validate if it can be the new head and find the parent
    let parent_header = find_parent_header(&block.header, storage)?;
    let mut state = evm_state(storage.clone(), parent_header.number);

    // Validate the block pre-execution
    validate_block(block, &parent_header, &state)?;

    let receipts = execute_block(block, &mut state)?;

    validate_gas_used(&receipts, &block.header)?;

    let account_updates = get_state_transitions(&mut state);

    // Apply the account updates over the last block's state and compute the new state root
    let new_state_root = state
        .database()
        .apply_account_updates(parent_header.number, &account_updates)?
        .unwrap_or_default();

    // Check state root matches the one in block header after execution
    validate_state_root(&block.header, new_state_root)?;

    let block_hash = block.header.compute_block_hash();
    store_block(storage, block.clone())?;
    store_receipts(storage, receipts, block_hash)?;

    Ok(())
}

/// Validates if the parent of the block is part of the canonical chain. Returns error if not.
pub fn validate_parent_canonical(block: &Block, storage: &Store) -> Result<(), ChainError> {
    match storage.get_canonical_block_hash(block.header.number.saturating_sub(1))? {
        Some(hash) if hash == block.header.parent_hash => Ok(()),
        _ => Err(ChainError::NonCanonicalParent),
    }
}

/// Stores block and header in the database
pub fn store_block(storage: &Store, block: Block) -> Result<(), ChainError> {
    storage.add_block(block)?;
    Ok(())
}

pub fn store_receipts(
    storage: &Store,
    receipts: Vec<Receipt>,
    block_hash: BlockHash,
) -> Result<(), ChainError> {
    for (index, receipt) in receipts.into_iter().enumerate() {
        storage.add_receipt(block_hash, index as u64, receipt)?;
    }
    Ok(())
}

/// Performs post-execution checks
pub fn validate_state_root(
    block_header: &BlockHeader,
    new_state_root: H256,
) -> Result<(), ChainError> {
    // Compare state root
    if new_state_root == block_header.state_root {
        Ok(())
    } else {
        Err(ChainError::InvalidBlock(
            InvalidBlockError::StateRootMismatch,
        ))
    }
}

pub fn latest_valid_hash(storage: &Store) -> Result<H256, ChainError> {
    if let Some(latest_block_number) = storage.get_latest_block_number()? {
        if let Some(latest_valid_header) = storage.get_block_header(latest_block_number)? {
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
pub fn find_parent_header(
    block_header: &BlockHeader,
    storage: &Store,
) -> Result<BlockHeader, ChainError> {
    let parent_hash = block_header.parent_hash;
    let parent_number = storage.get_block_number(parent_hash)?;

    if let Some(parent_number) = parent_number {
        let parent_header = storage.get_block_header(parent_number)?;

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

fn validate_gas_used(receipts: &[Receipt], block_header: &BlockHeader) -> Result<(), ChainError> {
    if let Some(last) = receipts.last() {
        if last.cumulative_gas_used != block_header.gas_used {
            return Err(ChainError::InvalidBlock(InvalidBlockError::GasUsedMismatch));
        }
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
    if block
        .header
        .blob_gas_used
        .is_some_and(|header_blob_gas_used| header_blob_gas_used != blob_gas_used)
    {
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
