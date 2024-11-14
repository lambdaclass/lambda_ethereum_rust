use risc0_zkvm::guest::env;

use ethereum_rust_blockchain::{validate_block, validate_gas_used};
use ethereum_rust_vm::{execute_block, get_state_transitions, EvmState};
use zkvm_interface::io::ProgramInput;

fn main() {
    let ProgramInput {
        block,
        parent_block_header,
        db,
    } = env::read();
    let mut state = EvmState::from(db.clone());

    // Validate the block pre-execution
    validate_block(&block, &parent_block_header, &state).expect("invalid block");

    // Validate the initial state
    let (state_trie, storage_tries) = db
        .build_tries()
        .expect("failed to build state and storage tries");

    let receipts = execute_block(&block, &mut state).expect("failed to execute block");

    validate_gas_used(&receipts, &block.header).expect("invalid gas used");

    // Output cumulative_gas_used to stdout
    env::commit(
        &receipts
            .last()
            .expect("no receipts found")
            .cumulative_gas_used,
    );

    let _account_updates = get_state_transitions(&mut state);

    // TODO: compute new state root from account updates and check it matches with the block's
    // header one.
}
