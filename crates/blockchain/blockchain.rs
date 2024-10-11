pub mod constants;
pub mod error;
pub mod mempool;
pub mod payload;
mod smoke_test;

use constants::{GAS_PER_BLOB, MAX_BLOB_GAS_PER_BLOCK, MAX_BLOB_NUMBER_PER_BLOCK};
use error::{ChainError, InvalidBlockError, InvalidForkChoice};
use ethereum_rust_core::types::{
    validate_block_header, validate_cancun_header_fields, validate_no_cancun_header_fields, Block,
    BlockHash, BlockHeader, BlockNumber, EIP4844Transaction, Receipt, Transaction,
};
use ethereum_rust_core::H256;

use ethereum_rust_storage::error::StoreError;
use ethereum_rust_storage::Store;
use ethereum_rust_vm::{
    evm_state, execute_block, get_state_transitions, spec_id, EvmState, SpecId,
};

//TODO: Implement a struct Chain or BlockChain to encapsulate
//functionality and canonical chain state and config

/// Adds a new block to the store. It may or may not be canonical, as long as its ancestry links
/// with the canonical chain and its parent's post-state is calculated. It doesn't modify the
/// canonical chain/head. Fork choice needs to be updated for that in a separate step.
///
/// Performs pre and post execution validation, and updates the database with the post state.
pub fn import_block(block: &Block, storage: &Store) -> Result<(), ChainError> {
    // TODO(#438): handle cases where blocks are missing between the canonical chain and the block.

    // Validate if it can be the new head and find the parent
    let parent_header = find_parent_header(&block.header, storage)?;
    let mut state = evm_state(storage.clone(), block.header.parent_hash);

    // Validate the block pre-execution
    validate_block(block, &parent_header, &state)?;

    let receipts = execute_block(block, &mut state)?;

    validate_gas_used(&receipts, &block.header)?;

    let account_updates = get_state_transitions(&mut state);

    // Apply the account updates over the last block's state and compute the new state root
    let new_state_root = state
        .database()
        .apply_account_updates(block.header.parent_hash, &account_updates)?
        .unwrap_or_default();

    // Check state root matches the one in block header after execution
    validate_state_root(&block.header, new_state_root)?;

    let block_hash = block.header.compute_block_hash();
    store_block(storage, block.clone())?;
    store_receipts(storage, receipts, block_hash)?;

    Ok(())
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
    match storage.get_block_header_by_hash(block_header.parent_hash)? {
        Some(parent_header) => Ok(parent_header),
        None => Err(ChainError::ParentNotFound),
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
    validate_block_header(&block.header, parent_header).map_err(InvalidBlockError::from)?;

    match spec {
        SpecId::CANCUN => validate_cancun_header_fields(&block.header, parent_header)
            .map_err(InvalidBlockError::from)?,
        _other_specs => {
            validate_no_cancun_header_fields(&block.header).map_err(InvalidBlockError::from)?
        }
    };

    if spec == SpecId::CANCUN {
        verify_blob_gas_usage(block)?
    }
    Ok(())
}

pub fn is_canonical(
    store: &Store,
    block_number: BlockNumber,
    block_hash: BlockHash,
) -> Result<bool, StoreError> {
    match store.get_canonical_block_hash(block_number)? {
        Some(hash) if hash == block_hash => Ok(true),
        _ => Ok(false),
    }
}

/// Applies new fork choice data to the current blockchain. It performs validity checks:
/// - The finalized, safe and head hashes must correspond to already saved blocks.
/// - The saved blocks should be in the correct order (finalized <= safe <= head).
/// - They must be connected.
///
/// After the validity checks, the canonical chain is updated so that all head's ancestors
/// and itself are made canonical.
///
/// If the fork choice state is applied correctly, the head block header is returned.
pub fn apply_fork_choice(
    store: &Store,
    head_hash: H256,
    safe_hash: H256,
    finalized_hash: H256,
) -> Result<BlockHeader, InvalidForkChoice> {
    if head_hash.is_zero() {
        return Err(InvalidForkChoice::InvalidHeadHash);
    }

    // We get the block bodies even if we only use headers them so we check that they are
    // stored too.
    let finalized_header_res = store.get_block_by_hash(finalized_hash)?;
    let safe_header_res = store.get_block_by_hash(safe_hash)?;
    let head_header_res = store.get_block_by_hash(head_hash)?;

    // Check that we already have all the needed blocks stored and that we have the ancestors
    // if we have the descendants, as we are working on the assumption that we only add block
    // if they are connected to the canonical chain.
    let (finalized, safe, head) = match (finalized_header_res, safe_header_res, head_header_res) {
        (None, Some(_), _) => return Err(InvalidForkChoice::ElementNotFound),
        (_, None, Some(_)) => return Err(InvalidForkChoice::ElementNotFound),
        (Some(f), Some(s), Some(h)) => (f.header, s.header, h.header),
        _ => return Err(InvalidForkChoice::Syncing),
    };

    // Check that we are not being pushed pre-merge
    total_difficulty_check(&head_hash, &head, store)?;

    // Check that the headers are in the correct order.
    if finalized.number > safe.number || safe.number > head.number {
        return Err(InvalidForkChoice::Unordered);
    }

    // If the head block is already in our canonical chain, the beacon client is
    // probably resyncing. Ignore the update.
    if is_canonical(store, head.number, head_hash)? {
        return Err(InvalidForkChoice::NewHeadAlreadyCanonical);
    }

    // Find out if blocks are correctly connected.
    let Some(new_canonical_blocks) = find_link_with_canonical_chain(store, &head)? else {
        return Err(InvalidForkChoice::Disconnected(
            error::ForkChoiceElement::Head,
            error::ForkChoiceElement::Safe,
        ));
    };

    let link_block_number = match new_canonical_blocks.last() {
        Some((number, _)) => *number,
        None => head.number,
    };

    // Check that finalized and safe blocks are either in the new canonical blocks, or already
    // but prior to the canonical link to the new head. This is a relatively quick way of making
    // sure that head, safe and finalized are connected.

    if !(is_canonical(store, finalized.number, finalized_hash)?
        && finalized.number <= link_block_number
        || new_canonical_blocks.contains(&(finalized.number, finalized_hash))
        || (finalized.number == head.number && finalized_hash == head_hash))
    {
        return Err(InvalidForkChoice::Disconnected(
            error::ForkChoiceElement::Head,
            error::ForkChoiceElement::Finalized,
        ));
    };

    if !((is_canonical(store, safe.number, safe_hash)? && safe.number <= link_block_number)
        || new_canonical_blocks.contains(&(safe.number, safe_hash))
        || (safe.number == head.number && safe_hash == head_hash))
    {
        return Err(InvalidForkChoice::Disconnected(
            error::ForkChoiceElement::Head,
            error::ForkChoiceElement::Safe,
        ));
    };

    // Finished all validations.
    for (number, hash) in new_canonical_blocks {
        store.set_canonical_block(number, hash)?;
    }

    // TODO(#791): should we panic here? We should never not have a latest block number.
    let Some(latest) = store.get_latest_block_number()? else {
        return Err(StoreError::Custom("Latest block number not found".to_string()).into());
    };

    for number in head.number..(latest + 1) {
        store.unset_canonical_block(number)?;
    }

    store.set_canonical_block(head.number, head_hash)?;
    store.update_finalized_block_number(finalized.number)?;
    store.update_safe_block_number(safe.number)?;
    Ok(head)
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

fn total_difficulty_check<'a>(
    head_block_hash: &'a H256,
    head_block: &'a BlockHeader,
    storage: &'a Store,
) -> Result<(), InvalidForkChoice> {
    // This check is performed only for genesis or for blocks with difficulty.
    if head_block.difficulty.is_zero() && head_block.number != 0 {
        return Ok(());
    }

    let total_difficulty = storage
        .get_block_total_difficulty(*head_block_hash)?
        .ok_or(StoreError::Custom(
            "Block difficulty not found for head block".to_string(),
        ))?;

    let terminal_total_difficulty = storage
        .get_chain_config()?
        .terminal_total_difficulty
        .ok_or(StoreError::Custom(
            "Terminal total difficulty not found in chain config".to_string(),
        ))?;

    // Check that the header is post-merge.
    if total_difficulty < terminal_total_difficulty.into() {
        return Err(InvalidForkChoice::PreMergeBlock);
    }

    if head_block.number == 0 {
        return Ok(());
    }

    // Non genesis checks

    let parent_total_difficulty = storage
        .get_block_total_difficulty(head_block.parent_hash)?
        .ok_or(StoreError::Custom(
            "Block difficulty not found for parent block".to_string(),
        ))?;

    // TODO(#790): is this check necessary and correctly implemented?
    if parent_total_difficulty >= terminal_total_difficulty.into() {
        Err((StoreError::Custom(
            "Parent block is already post terminal total difficulty".to_string(),
        ))
        .into())
    } else {
        Ok(())
    }
}

// Find branch of the blockchain connecting a block with the canonical chain. Returns the
// number-hash pairs representing all blocks in that brunch. If genesis is reached and the link
// hasn't been found, an error is returned.
//
// Return values:
// - Err(StoreError): a db-related error happened.
// - Ok(None): The block is not connected to the canonical chain.
// - Ok(Some([])): the block is already canonical.
// - Ok(Some(branch)): the "branch" is a sequence of blocks that connects the ancestor and the
//   descendant.
fn find_link_with_canonical_chain(
    store: &Store,
    block: &BlockHeader,
) -> Result<Option<Vec<(BlockNumber, BlockHash)>>, StoreError> {
    let mut block_number = block.number;
    let block_hash = block.compute_block_hash();
    let mut header = block.clone();
    let mut branch = Vec::new();

    if is_canonical(store, block_number, block_hash)? {
        return Ok(Some(branch));
    }

    let Some(genesis_number) = store.get_earliest_block_number()? else {
        return Err(StoreError::Custom(
            "Earliest block number not found. Node setup must have been faulty.".to_string(),
        ));
    };

    while block_number > genesis_number {
        block_number -= 1;
        let parent_hash = header.parent_hash;

        // Check that the parent exists.
        let parent_header = match store.get_block_header_by_hash(parent_hash) {
            Ok(Some(header)) => header,
            Ok(None) => return Ok(None),
            Err(error) => return Err(error),
        };

        if is_canonical(store, block_number, parent_hash)? {
            return Ok(Some(branch));
        } else {
            branch.push((block_number, parent_hash));
        }

        header = parent_header;
    }

    Ok(None)
}
#[cfg(test)]
mod tests {}
