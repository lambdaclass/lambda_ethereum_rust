use clap::{Arg, ArgAction, Command};

pub fn cli() -> Command {
    Command::new("Ethrex")
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
            Arg::new("addr")
                .long("addr")
                .default_value("0.0.0.0")
                .value_name("ADDRESS")
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new("port")
                .long("port")
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
                .default_value("")
                .value_name("GENESIS_FILE_PATH")
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new("bootnodes")
                .long("bootnodes")
                .default_value("")
                .value_name("BOOTNODE_LIST")
                .action(ArgAction::Set),
        )
}
