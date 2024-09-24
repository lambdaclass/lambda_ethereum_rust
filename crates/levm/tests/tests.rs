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

#[test]
fn lt_a_less_than_b() {
    let mut vm = VM::default();

    let operations = [
        Operation::Push32(U256::one()),  // b
        Operation::Push32(U256::zero()), // a
        Operation::Lt,
        Operation::Stop,
    ];

    let bytecode = operations
        .iter()
        .flat_map(Operation::to_bytecode)
        .collect::<Bytes>();

    vm.execute(bytecode);

    assert!(vm.stack.pop().unwrap() == U256::one());
    assert!(vm.pc() == 68);
}

#[test]
fn lt_a_equals_b() {
    let mut vm = VM::default();

    let operations = [
        Operation::Push32(U256::zero()), // b
        Operation::Push32(U256::zero()), // a
        Operation::Lt,
        Operation::Stop,
    ];

    let bytecode = operations
        .iter()
        .flat_map(Operation::to_bytecode)
        .collect::<Bytes>();

    vm.execute(bytecode);

    assert!(vm.stack.pop().unwrap() == U256::zero());
    assert!(vm.pc() == 68);
}

#[test]
fn lt_a_greater_than_b() {
    let mut vm = VM::default();

    let operations = [
        Operation::Push32(U256::zero()), // b
        Operation::Push32(U256::one()),  // a
        Operation::Lt,
        Operation::Stop,
    ];

    let bytecode = operations
        .iter()
        .flat_map(Operation::to_bytecode)
        .collect::<Bytes>();

    vm.execute(bytecode);

    assert!(vm.stack.pop().unwrap() == U256::zero());
    assert!(vm.pc() == 68);
}

#[test]
fn gt_a_greater_than_b() {
    let mut vm = VM::default();

    let operations = [
        Operation::Push32(U256::zero()), // b
        Operation::Push32(U256::one()),  // a
        Operation::Gt,
        Operation::Stop,
    ];

    let bytecode = operations
        .iter()
        .flat_map(Operation::to_bytecode)
        .collect::<Bytes>();

    vm.execute(bytecode);

    assert!(vm.stack.pop().unwrap() == U256::one());
    assert!(vm.pc() == 68);
}

#[test]
fn gt_a_equals_b() {
    let mut vm = VM::default();

    let operations = [
        Operation::Push32(U256::zero()), // b
        Operation::Push32(U256::zero()), // a
        Operation::Gt,
        Operation::Stop,
    ];

    let bytecode = operations
        .iter()
        .flat_map(Operation::to_bytecode)
        .collect::<Bytes>();

    vm.execute(bytecode);

    assert!(vm.stack.pop().unwrap() == U256::zero());
    assert!(vm.pc() == 68);
}

#[test]
fn gt_a_less_than_b() {
    let mut vm = VM::default();

    let operations = [
        Operation::Push32(U256::one()),  // b
        Operation::Push32(U256::zero()), // a
        Operation::Gt,
        Operation::Stop,
    ];

    let bytecode = operations
        .iter()
        .flat_map(Operation::to_bytecode)
        .collect::<Bytes>();

    vm.execute(bytecode);

    assert!(vm.stack.pop().unwrap() == U256::zero());
    assert!(vm.pc() == 68);
}

#[test]
fn slt_zero_a_less_than_positive_b() {
    let mut vm = VM::default();

    let operations = [
        Operation::Push32(U256::one()),  // b
        Operation::Push32(U256::zero()), // a
        Operation::Slt,
        Operation::Stop,
    ];

    let bytecode = operations
        .iter()
        .flat_map(Operation::to_bytecode)
        .collect::<Bytes>();

    vm.execute(bytecode);

    assert!(vm.stack.pop().unwrap() == U256::one());
    assert!(vm.pc() == 68);
}

#[test]
fn slt_negative_a_less_than_positive_b() {
    let mut vm = VM::default();

    let operations = [
        Operation::Push32(U256::one()),            // b
        Operation::Push32(U256::from([0xff; 32])), // a
        Operation::Slt,
        Operation::Stop,
    ];

    let bytecode = operations
        .iter()
        .flat_map(Operation::to_bytecode)
        .collect::<Bytes>();

    vm.execute(bytecode);

    assert!(vm.stack.pop().unwrap() == U256::one());
    assert!(vm.pc() == 68);
}

#[test]
fn slt_zero_a_greater_than_negative_b() {
    let mut vm = VM::default();

    let operations = [
        Operation::Push32(U256::from([0xff; 32])), // b
        Operation::Push32(U256::zero()),           // a
        Operation::Slt,
        Operation::Stop,
    ];

    let bytecode = operations
        .iter()
        .flat_map(Operation::to_bytecode)
        .collect::<Bytes>();

    vm.execute(bytecode);

    assert!(vm.stack.pop().unwrap() == U256::zero());
    assert!(vm.pc() == 68);
}

#[test]
fn slt_positive_a_greater_than_negative_b() {
    let mut vm = VM::default();

    let operations = [
        Operation::Push32(U256::from([0xff; 32])), // b
        Operation::Push32(U256::one()),            // a
        Operation::Slt,
        Operation::Stop,
    ];

    let bytecode = operations
        .iter()
        .flat_map(Operation::to_bytecode)
        .collect::<Bytes>();

    vm.execute(bytecode);

    assert!(vm.stack.pop().unwrap() == U256::zero());
    assert!(vm.pc() == 68);
}

#[test]
fn sgt_positive_a_greater_than_zero_b() {
    let mut vm = VM::default();

    let operations = [
        Operation::Push32(U256::zero()),  // b
        Operation::Push32(U256::one()), // a
        Operation::Sgt,
        Operation::Stop,
    ];

    let bytecode = operations
        .iter()
        .flat_map(Operation::to_bytecode)
        .collect::<Bytes>();

    vm.execute(bytecode);

    assert!(vm.stack.pop().unwrap() == U256::one());
    assert!(vm.pc() == 68);
}

#[test]
fn sgt_positive_a_greater_than_negative_b() {
    let mut vm = VM::default();

    let operations = [
        Operation::Push32(U256::from([0xff; 32])),  // b = -1
        Operation::Push32(U256::one()), // a
        Operation::Sgt,
        Operation::Stop,
    ];

    let bytecode = operations
        .iter()
        .flat_map(Operation::to_bytecode)
        .collect::<Bytes>();

    vm.execute(bytecode);

    assert!(vm.stack.pop().unwrap() == U256::one());
    assert!(vm.pc() == 68);
}

#[test]
fn sgt_negative_a_greater_than_negative_b() {
    let mut vm = VM::default();

    let operations = [
        Operation::Push32(U256::from([0xff; 32]).saturating_sub(U256::one())),  // b = -2
        Operation::Push32(U256::from([0xff; 32])), // a = -1
        Operation::Sgt,
        Operation::Stop,
    ];

    let bytecode = operations
        .iter()
        .flat_map(Operation::to_bytecode)
        .collect::<Bytes>();

    vm.execute(bytecode);

    assert!(vm.stack.pop().unwrap() == U256::one());
    assert!(vm.pc() == 68);
}

#[test]
fn sgt_negative_a_less_than_positive_b() {
    let mut vm = VM::default();

    let operations = [
        Operation::Push32(U256::one()), // b
        Operation::Push32(U256::from([0xff; 32])),  // a
        Operation::Sgt,
        Operation::Stop,
    ];

    let bytecode = operations
        .iter()
        .flat_map(Operation::to_bytecode)
        .collect::<Bytes>();

    vm.execute(bytecode);

    assert!(vm.stack.pop().unwrap() == U256::zero());
    assert!(vm.pc() == 68);
}
