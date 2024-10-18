use ethereum_rust_levm_mlir::{
    db::{Bytecode, Db},
    primitives::{Address, Bytes},
    Environment, Evm,
};

const SNAILTRACER_BYTECODE: &[u8] = include_bytes!("../programs/snailtracer.bytecode");

#[test]
#[ignore]
// TODO: this test requires SSTORE, SLOAD, and CALLDATA related opcodes
fn snailtracer() {
    let address = Address::zero();
    let mut env = Environment::default();
    env.tx_calldata = Bytes::from(vec![48, 98, 123, 124]);
    env.tx_gas_limit = 999_999;
    let mut caller_address = vec![0x0; 20];
    caller_address[0] = 16;
    env.tx_caller = Address::from_slice(&caller_address);
    env.tx_to = Some(address);

    let db = Db::new().with_contract(address, Bytecode::from(SNAILTRACER_BYTECODE));

    let mut evm = Evm::new(env, db);

    let _ = evm.transact();
}
