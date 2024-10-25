use ethereum_rust_rlp::decode::RLPDecode;
use ethereum_rust_rlp::encode::RLPEncode;
use risc0_zkvm::guest::env;

use ethereum_rust_core::types::Block;
use ethereum_rust_vm::{execute_block, execution_db::ExecutionDB, EvmState};

fn main() {
    // Read inputs
    let head_block_bytes = env::read::<Vec<u8>>();
    let execution_db = env::read::<ExecutionDB>();

    let block = Block::decode(&head_block_bytes).unwrap();

    env::commit(&block.encode_to_vec());
    env::commit(&execution_db);

    // Execute block
    let mut state = EvmState::from_exec_db(execution_db);
    let block_receipts = execute_block(&block, &mut state).unwrap();
    env::commit(&block_receipts);
}
