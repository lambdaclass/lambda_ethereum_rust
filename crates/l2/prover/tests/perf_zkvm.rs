use serde::Deserialize;
use std::path::PathBuf;
use tracing::info;

use ethereum_rust_blockchain::add_block;
use ethereum_rust_core::types::{Block, Receipt};
use ethereum_rust_l2::proposer::prover_server::ProverInputData;
use ethereum_rust_prover_lib::prover::Prover;
use ethereum_rust_storage::{EngineType, Store};
use ethereum_rust_vm::execution_db::ExecutionDB;

// The order of variables in this structure should match the order in which they were
// committed in the zkVM, with each variable represented by a field.
#[derive(Debug, Deserialize)]
struct ProverOutputData {
    /// It is rlp encoded, it has to be decoded.
    /// Block::decode(&prover_output_data.block).unwrap());
    _block: Vec<u8>,
    _execution_db: ExecutionDB,
    block_receipts: Vec<Receipt>,
}

#[tokio::test]
async fn test_performance_zkvm() {
    tracing_subscriber::fmt::init();

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
    info!("Number of blocks to insert: {}", blocks.len());

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

    let receipt = prover.prove().unwrap();

    let duration = start.elapsed();
    info!("[SECONDS] Proving Took: {:?}", duration);
    info!("[MINUTES] Proving Took: {}[m]", duration.as_secs() / 60);

    prover.verify(&receipt).unwrap();

    let output: ProverOutputData = receipt.journal.decode().unwrap();

    let execution_cumulative_gas_used = output.block_receipts.last().unwrap().cumulative_gas_used;
    info!("Cumulative Gas Used {execution_cumulative_gas_used}");

    let gas_per_second = execution_cumulative_gas_used as f64 / duration.as_secs_f64();

    info!("Gas per Second: {}", gas_per_second);
}
