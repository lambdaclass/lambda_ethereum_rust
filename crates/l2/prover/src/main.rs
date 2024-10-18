use ethereum_rust_prover_lib::init_client;

#[tokio::main]
async fn main() {
    init_client().await;
}
