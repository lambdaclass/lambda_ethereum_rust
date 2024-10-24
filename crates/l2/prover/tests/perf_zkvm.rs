use std::path::PathBuf;

use ethereum_rust_blockchain::add_block;
use ethereum_rust_core::types::Block;
use ethereum_rust_l2::proposer::prover_server::ProverInputData;
use ethereum_rust_prover_lib::prover::Prover;
use ethereum_rust_storage::{EngineType, Store};
use ethereum_rust_vm::execution_db::ExecutionDB;

#[tokio::test]
async fn test_performance_zkvm() {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // Go back 3 levels (Go to the root of the project)
    for _ in 0..3 {
        path.pop();
    }
    path.push("test_data");

    let genesis_file_path = path.join("genesis-execution-api.json");
    let chain_file_path = path.join("chain.rlp");

    let store = Store::new("memory", EngineType::InMemory).expect("Failed to create Store");

    let genesis = ethereum_rust_l2::utils::test_files_read::read_genesis_file(
        genesis_file_path.to_str().unwrap(),
    );
    store.add_initial_state(genesis.clone()).unwrap();

    let blocks = ethereum_rust_l2::utils::test_files_read::read_chain_file(
        chain_file_path.to_str().unwrap(),
    );
    println!("Number of blocks to insert: {}", blocks.len());

    let mut last_block = Block::default();
    for (i, block) in blocks.iter().enumerate() {
        add_block(block, &store).unwrap();
        if i == (blocks.len() - 1) {
            last_block = block.clone();
        }
    }

    let db = ExecutionDB::from_exec(&last_block, &store).unwrap();
    let input = ProverInputData {
        db,
        block: last_block,
    };

    let mut prover = Prover::new();
    prover.set_input(input);

    let start = std::time::Instant::now();

    let receipt = prover.prove();

    let duration = start.elapsed();
    println!("[SECONDS] Proving Took: {:?}", duration);
    println!("[MINUTES] Proving Took: {}[m]", duration.as_secs() / 60);

    prover.verify(&receipt.unwrap()).unwrap();
}
