use ethereum_rust_rlp::decode::RLPDecode;
use risc0_zkvm::guest::env;

use ethereum_rust_core::types::Block;
use ethereum_rust_vm::{execute_block, execution_db::ExecutionDB, EvmState};

fn main() {
    // Read the input
    let head_block_bytes = env::read::<Vec<u8>>();
    // let parent_header_bytes = env::read::<Vec<u8>>();
    let execution_db = env::read::<ExecutionDB>();

    // SetUp data from inputs
    let block = Block::decode(&head_block_bytes).unwrap();

    // Make DataInputs public.
    env::commit(&block);
    //env::commit(&parent_header);
    env::commit(&execution_db);

    // Execute block
    let mut state = EvmState::from_exec_db(execution_db);
    let block_receipts = execute_block(&block, &mut state).unwrap();
    env::commit(&block_receipts);
}
