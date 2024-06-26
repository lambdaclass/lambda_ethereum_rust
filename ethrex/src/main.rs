use core::types::Genesis;
use net::types::BootNode;
use std::num::ParseIntError;
use std::{
    io::BufReader,
    net::{AddrParseError, SocketAddr},
};
use tokio::join;
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

    let tcp_addr = matches.get_one::<String>("addr").expect("addr is required");
    let tcp_port = matches.get_one::<String>("port").expect("port is required");
    let udp_addr = matches
        .get_one::<String>("discovery.addr")
        .expect("discovery.addr is required");
    let udp_port = matches
        .get_one::<String>("discovery.port")
        .expect("discovery.port is required");

    let genesis_file_path = matches
        .get_one::<String>("network")
        .expect("network is required");

    let bootnode_string = matches
        .get_one::<String>("bootnodes")
        .expect("bootnode is required");
    let bootnode = parse_bootnode(&bootnode_string);

    let http_socket_addr =
        parse_socket_addr(http_addr, http_port).expect("Failed to parse http address and port");
    let authrpc_socket_addr = parse_socket_addr(authrpc_addr, authrpc_port)
        .expect("Failed to parse authrpc address and port");

    let udp_socket_addr =
        parse_socket_addr(udp_addr, udp_port).expect("Failed to parse discovery address and port");
    let tcp_socket_addr =
        parse_socket_addr(tcp_addr, tcp_port).expect("Failed to parse addr and port");

    let _genesis = read_genesis_file(genesis_file_path);

    let rpc_api = rpc::start_api(http_socket_addr, authrpc_socket_addr);
    let networking = net::start_network(udp_socket_addr, tcp_socket_addr);

    join!(rpc_api, networking);
}

fn read_genesis_file(genesis_file_path: &str) -> Genesis {
    let genesis_file = std::fs::File::open(genesis_file_path).expect("Failed to open genesis file");
    let genesis_reader = BufReader::new(genesis_file);
    serde_json::from_reader(genesis_reader).expect("Failed to read genesis file")
}

fn parse_socket_addr(addr: &str, port: &str) -> Result<SocketAddr, AddrParseError> {
    format!("{addr}:{port}").parse()
}

fn parse_bootnode(input: &str) -> BootNode {
    // TODO: error handling
    let node_id = decode_hex(&input[8..136]).unwrap();
    let socket_address: SocketAddr = input[137..]
        .parse()
        .expect("Failed to parse bootnode address and port");
    BootNode {
        node_id,
        socket_address,
    }
}

pub fn decode_hex(s: &str) -> Result<Vec<u8>, ParseIntError> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
        .collect()
}
