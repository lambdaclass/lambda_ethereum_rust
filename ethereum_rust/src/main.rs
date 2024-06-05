use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;
fn main() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::DEBUG)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    info!("Starting Ethereum Rust application");

    rpc::start_api();
}
