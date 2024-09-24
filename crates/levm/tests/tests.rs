use bytes::Bytes;
use ethereum_types::U256;
use levm::{operations::Operation, vm::VM};

#[test]
fn test() {
    let mut vm = VM::default();

    let operations = [
        Operation::Push32(U256::one()),
        Operation::Push32(U256::zero()),
        Operation::Add,
        Operation::Stop,
    ];

    let bytecode = operations
        .iter()
        .flat_map(Operation::to_bytecode)
        .collect::<Bytes>();

    vm.execute(bytecode);

    assert!(vm.stack.pop().unwrap() == U256::one());
    assert!(vm.pc() == 68);

    println!("{vm:?}");
}
