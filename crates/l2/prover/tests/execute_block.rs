use ethereum_rust_l2::proposer::prover_server::ProverInputData;
use ethereum_rust_prover_lib::prover::Prover;

#[tokio::test]
async fn test_performance_execute_block() {
    let input = ProverInputData::default();

    let mut prover = Prover::new();
    prover.set_input(input);

    let start = std::time::Instant::now();

    let receipt = prover.prove();

    let duration = start.elapsed();
    println!("[SECONDS] Proving Took: {:?}", duration);
    println!("[MINUTES] Proving Took: {}[m]", duration.as_secs() / 60);

    prover.verify(&receipt.unwrap()).unwrap();
}
