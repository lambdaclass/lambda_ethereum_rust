use ethereum_rust_l2::utils::config::prover::ProverConfig;
use ethereum_rust_prover_lib::init_client;

use tracing::{self, Level};

#[tokio::main]
async fn main() {
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(Level::DEBUG)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
    let config = ProverConfig::from_env().unwrap();
    init_client(config).await;
}
