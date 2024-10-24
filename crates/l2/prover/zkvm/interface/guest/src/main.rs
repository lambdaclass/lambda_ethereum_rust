use ethereum_rust_rlp::{decode::RLPDecode, error::RLPDecodeError};
use risc0_zkvm::guest::env;

use ethereum_rust_blockchain::{validate_block, validate_gas_used, validate_state_root};
use ethereum_rust_core::types::{Block, BlockHeader};
use ethereum_rust_vm::{execute_block, execution_db::ExecutionDB, get_state_transitions, EvmState};

fn main() {
    let (block, execution_db, parent_header) = read_inputs().expect("failed to read inputs");
    let mut state = EvmState::from_exec_db(execution_db.clone());

    // Validate the block pre-execution
    validate_block(&block, &parent_header, &state).expect("invalid block");

    let receipts = execute_block(&block, &mut state).unwrap();

    validate_gas_used(&receipts, &block.header).expect("invalid gas used");

    let account_updates = get_state_transitions(&mut state);

    // Apply the account updates over the last block's state and compute the new state root
    let new_state_root = execution_db
        .apply_account_updates(block.header.parent_hash, &account_updates)
        .expect("failed to apply account updates")
        .unwrap_or_default();

    // Check state root matches the one in block header after execution
    // validate_state_root(&block.header, new_state_root).expect("invalid state root");
}

fn read_inputs() -> Result<(Block, ExecutionDB, BlockHeader), RLPDecodeError> {
    let head_block_bytes = env::read::<Vec<u8>>();
    let execution_db = env::read::<ExecutionDB>();
    let parent_header_bytes = env::read::<Vec<u8>>();

    let block = Block::decode(&head_block_bytes)?;
    let parent_header = BlockHeader::decode(&parent_header_bytes)?;

    // make inputs public
    env::commit(&block);
    env::commit(&execution_db);
    env::commit(&parent_header);

    Ok((block, execution_db, parent_header))
}
