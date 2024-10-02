use std::path::PathBuf;

use ethereum_rust_levm_mlir::{
    context::{Context, Session},
    db::Db,
    env::Env,
    executor::{Executor, OptLevel},
    journal::Journal,
    program::Program,
    syscall::SyscallContext,
};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let path = args.get(1).expect("No path provided").as_str();
    let opt_level = match args.get(2).map(String::as_str) {
        None | Some("2") => OptLevel::Default,
        Some("0") => OptLevel::None,
        Some("1") => OptLevel::Less,
        Some("3") => OptLevel::Aggressive,
        _ => panic!("Invalid optimization level"),
    };
    let bytecode = std::fs::read(path).expect("Could not read file");
    let program = Program::from_bytecode(&bytecode);

    let session = Session {
        raw_mlir_path: Some(PathBuf::from("output")),
        ..Default::default()
    };

    let context = Context::new();
    let module = context
        .compile(&program, session)
        .expect("failed to compile program");

    let env = Env::default();
    let initial_gas = env.tx.gas_limit;
    let mut db = Db::default();
    let journal = Journal::new(&mut db);
    let mut context = SyscallContext::new(env, journal, Default::default(), initial_gas);
    let executor = Executor::new(&module, &context, opt_level);

    let result = executor.execute(&mut context, initial_gas);
    println!("Execution result: {result}");
}
