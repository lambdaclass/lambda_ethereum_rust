use std::io::Read;

use bytes::Bytes;
use ethereum_rust_levm_mlir::{db::Db, env::TransactTo, Env, Evm};
use ethereum_types::Address;

fn read_compiled_file(file_path: &str) -> Result<Bytes, std::io::Error> {
    let mut file = std::fs::File::open(file_path)?;
    let mut buffer = String::new();
    file.read_to_string(&mut buffer)?;
    Ok(Bytes::from(hex::decode(buffer).unwrap()))
}

fn default_evm_with_bytecode(bytecode: Bytes, address: Address) -> Evm<Db> {
    let mut env = Env::default();
    env.tx.gas_limit = 999_999;
    env.tx.transact_to = TransactTo::Call(address);
    let db = Db::new().with_contract(address, bytecode);
    Evm::new(env, db)
}

#[test]
fn factorial_contract() {
    let address = Address::from_low_u64_be(40);
    let bytes = read_compiled_file("./programs/Factorial.bin").unwrap();
    let mut evm = default_evm_with_bytecode(bytes, address);
    let result = evm.transact().unwrap();
    assert!(result.result.is_success());
    let state = result.state.get(&address).unwrap();
    assert_eq!(
        state
            .storage
            .get(&ethereum_types::U256::zero())
            .unwrap()
            .present_value,
        ethereum_types::U256::from(3628800) // 10!
    )
}

#[test]
fn fibonacci_contract() {
    let address = Address::from_low_u64_be(40);
    let bytes = read_compiled_file("./programs/Fibonacci.bin").unwrap();
    let mut evm = default_evm_with_bytecode(bytes, address);
    let result = evm.transact().unwrap();
    assert!(result.result.is_success());
    let state = result.state.get(&address).unwrap();
    assert_eq!(
        state
            .storage
            .get(&ethereum_types::U256::zero())
            .unwrap()
            .present_value,
        ethereum_types::U256::from(55) // fibonacci(10)
    )
}

#[test]
fn recursive_fibonacci_contract() {
    let address = Address::from_low_u64_be(40);
    let bytes = read_compiled_file("./programs/RecursiveFibonacci.bin").unwrap();
    let mut evm = default_evm_with_bytecode(bytes, address);
    let result = evm.transact().unwrap();
    assert!(result.result.is_success());
    let state = result.state.get(&address).unwrap();
    assert_eq!(
        state
            .storage
            .get(&ethereum_types::U256::zero())
            .unwrap()
            .present_value,
        ethereum_types::U256::from(55) // fibonacci(10)
    )
}
