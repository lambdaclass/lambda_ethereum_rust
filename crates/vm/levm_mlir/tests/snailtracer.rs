use ethereum_rust_levm_mlir::{
    db::{Bytecode, Db},
    env::TransactTo,
    primitives::{Address, Bytes},
    Env, Evm,
};

const SNAILTRACER_BYTECODE: &[u8] = include_bytes!("../programs/snailtracer.bytecode");

#[test]
#[ignore]
// TODO: this test requires SSTORE, SLOAD, and CALLDATA related opcodes
fn snailtracer() {
    let address = Address::zero();
    let mut env = Env::default();
    env.tx.data = Bytes::from(vec![48, 98, 123, 124]);
    env.tx.gas_limit = 999_999;
    let mut caller_address = vec![0x0; 20];
    caller_address[0] = 16;
    env.tx.caller = Address::from_slice(&caller_address);
    env.tx.transact_to = TransactTo::Call(address);

    let db = Db::new().with_contract(address, Bytecode::from(SNAILTRACER_BYTECODE));

    let mut evm = Evm::new(env, db);

    let _ = evm.transact();
}
