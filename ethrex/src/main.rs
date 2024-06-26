use core::types::Genesis;
use std::io::BufReader;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

mod cli;

#[tokio::main]
async fn main() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::DEBUG)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let matches = cli::cli().get_matches();

    let http_addr = matches
        .get_one::<String>("http.addr")
        .expect("http.addr is required");
    let http_port = matches
        .get_one::<String>("http.port")
        .expect("http.port is required");
    let authrpc_addr = matches
        .get_one::<String>("authrpc.addr")
        .expect("authrpc.addr is required");
    let authrpc_port = matches
        .get_one::<String>("authrpc.port")
        .expect("authrpc.port is required");
    let genesis_file_path = matches
        .get_one::<String>("network")
        .expect("network is required");

    let _genesis = read_genesis_file(genesis_file_path);

    rpc::start_api(http_addr, http_port, authrpc_addr, authrpc_port).await;
}

fn read_genesis_file(genesis_file_path: &str) -> Genesis {
    let genesis_file = std::fs::File::open(genesis_file_path).expect("Failed to open genesis file");
    let genesis_reader = BufReader::new(genesis_file);
    serde_json::from_reader(genesis_reader).expect("Failed to read genesis file")
}
