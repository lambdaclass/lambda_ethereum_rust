use ethereum_rust_core::types::Transaction;
use risc0_zkvm::serde::from_slice;
use std::path::Path;
use tracing::info;

use ethereum_rust_blockchain::add_block;
use ethereum_rust_prover_lib::prover::Prover;
use ethereum_rust_storage::{EngineType, Store};
use ethereum_rust_vm::execution_db::ExecutionDB;
use zkvm_interface::io::ProgramInput;

#[ignore]
#[tokio::test]
async fn test_performance_zkvm() {
    tracing_subscriber::fmt::init();

    let mut path = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test_data"));

    // Another use is genesis-execution-api.json in conjunction with chain.rlp(20 blocks not too loaded).
    let genesis_file_path = path.join("genesis-l2.json");
    // l2-loadtest.rlp has blocks with many txs.
    let chain_file_path = path.join("l2-loadtest.rlp");

    let store = Store::new("memory", EngineType::InMemory).expect("Failed to create Store");

    let genesis = ethereum_rust_l2::utils::test_data_io::read_genesis_file(
        genesis_file_path.to_str().unwrap(),
    );
    store.add_initial_state(genesis.clone()).unwrap();

    let blocks =
        ethereum_rust_l2::utils::test_data_io::read_chain_file(chain_file_path.to_str().unwrap());
    info!("Number of blocks to insert: {}", blocks.len());

    for block in &blocks {
        add_block(block, &store).unwrap();
    }
    let block_to_prove = blocks.last().unwrap();

    let db = ExecutionDB::from_exec(block_to_prove, &store).unwrap();

    let parent_block_header = store
        .get_block_header_by_hash(block_to_prove.header.parent_hash)
        .unwrap()
        .unwrap();

    let input = ProgramInput {
        block: block_to_prove.clone(),
        parent_block_header,
        db,
    };

    let mut prover = Prover::new();

    let start = std::time::Instant::now();

    let receipt = prover.prove(input).unwrap();

    let duration = start.elapsed();
    info!(
        "Number of EIP1559 transactions in the proven block: {}",
        block_to_prove.body.transactions.len()
    );
    info!("[SECONDS] Proving Took: {:?}", duration);
    info!("[MINUTES] Proving Took: {}[m]", duration.as_secs() / 60);

    prover.verify(&receipt).unwrap();

    let _program_output = Prover::get_commitment(&receipt).unwrap();
    let cumulative_gas_used: u64 = from_slice(&prover.stdout).unwrap();

    info!("Cumulative Gas Used {cumulative_gas_used}");

    let gas_per_second = cumulative_gas_used as f64 / duration.as_secs_f64();

    info!("Gas per Second: {}", gas_per_second);
}

#[tokio::test]
async fn test_tx_serialization_serde_json() {
    let mut path = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test_data"));

    // l2-loadtest.rlp has blocks with many txs.
    let chain_file_path = path.join("l2-loadtest.rlp");

    let blocks =
        ethereum_rust_l2::utils::test_data_io::read_chain_file(chain_file_path.to_str().unwrap());

    for block in &blocks {
        let serialized_tx =
            serde_json::to_vec(&block.body.transactions).expect("failed to serialize");
        let deserialized_txs: Vec<Transaction> =
            serde_json::from_slice(&serialized_tx).expect("failed to deserialize");
    }
}

#[tokio::test]
async fn test_tx_serialization_bincode() {
    let mut path = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test_data"));

    // l2-loadtest.rlp has blocks with many txs.
    let chain_file_path = path.join("l2-loadtest.rlp");

    let blocks =
        ethereum_rust_l2::utils::test_data_io::read_chain_file(chain_file_path.to_str().unwrap());

    for block in &blocks {
        let serialized_txs =
            bincode::serialize(&block.body.transactions).expect("failed to serialize");
        let deserialized_txs: Vec<Transaction> =
            bincode::deserialize(&serialized_txs).expect("failed to deserialize");
    }
}
