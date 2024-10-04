use clap::{Arg, ArgAction, Command};
use ethereum_rust_net::bootnode::BootNode;
use tracing::Level;

pub fn cli() -> Command {
    Command::new("ethereum_rust")
        .about("Ethereum Rust Execution client")
        .author("Lambdaclass")
        .arg(
            Arg::new("http.addr")
                .long("http.addr")
                .default_value("localhost")
                .value_name("ADDRESS")
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new("http.port")
                .long("http.port")
                .default_value("8545")
                .value_name("PORT")
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new("log-level")
                .long("log-level")
                .default_value(Level::INFO.as_str())
                .value_name("LOG_LEVEL")
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new("authrpc.addr")
                .long("authrpc.addr")
                .default_value("localhost")
                .value_name("ADDRESS")
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new("authrpc.port")
                .long("authrpc.port")
                .default_value("8551")
                .value_name("PORT")
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new("authrpc.jwtsecret")
                .long("authrpc.jwtsecret")
                .default_value("jwt.hex")
                .value_name("JWTSECRET_PATH")
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new("p2p.addr")
                .long("p2p.addr")
                .default_value("0.0.0.0")
                .value_name("ADDRESS")
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new("p2p.port")
                .long("p2p.port")
                .default_value("30303")
                .value_name("PORT")
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new("discovery.addr")
                .long("discovery.addr")
                .default_value("0.0.0.0")
                .value_name("ADDRESS")
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new("discovery.port")
                .long("discovery.port")
                .default_value("30303")
                .value_name("PORT")
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new("network")
                .long("network")
                .value_name("GENESIS_FILE_PATH")
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new("bootnodes")
                .long("bootnodes")
                .value_name("BOOTNODE_LIST")
                .value_parser(clap::value_parser!(BootNode))
                .value_delimiter(',')
                .num_args(1..)
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new("datadir")
                .long("datadir")
                .value_name("DATABASE_DIRECTORY")
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new("import")
                .long("import")
                .required(false)
                .value_name("CHAIN_RLP_PATH"),
        )
        .subcommand(
            Command::new("removedb").about("Remove the database").arg(
                Arg::new("datadir")
                    .long("datadir")
                    .value_name("DATABASE_DIRECTORY")
                    .action(ArgAction::Set),
            ),
        )
}
