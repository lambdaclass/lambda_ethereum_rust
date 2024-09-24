use std::str::FromStr;

use bytes::Bytes;
use ethereum_types::U256;
use levm::{operations::Operation, vm::VM};

#[test]
fn add_op() {
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
}

#[test]
fn mul_op() {
    let mut vm = VM::default();

    let operations = [
        Operation::Push32(U256::from(10)),
        Operation::Push32(U256::from(10)),
        Operation::Mul,
        Operation::Stop,
    ];

    let bytecode = operations
        .iter()
        .flat_map(Operation::to_bytecode)
        .collect::<Bytes>();

    vm.execute(bytecode);

    assert!(vm.stack.pop().unwrap() == U256::from(10 * 10));
}

#[test]
fn sub_op() {
    let mut vm = VM::default();

    let operations = [
        Operation::Push32(U256::from(20)),
        Operation::Push32(U256::from(30)),
        Operation::Sub,
        Operation::Stop,
    ];

    let bytecode = operations
        .iter()
        .flat_map(Operation::to_bytecode)
        .collect::<Bytes>();

    vm.execute(bytecode);

    assert!(vm.stack.pop().unwrap() == U256::from(10));
}

#[test]
fn div_op() {
    let mut vm = VM::default();

    let operations = [
        Operation::Push32(U256::from(6)),
        Operation::Push32(U256::from(12)),
        Operation::Div,
        Operation::Stop,
    ];

    let bytecode = operations
        .iter()
        .flat_map(Operation::to_bytecode)
        .collect::<Bytes>();

    vm.execute(bytecode);

    assert!(vm.stack.pop().unwrap() == U256::from(2));
}

#[test]
fn div_op_for_zero() {
    let mut vm = VM::default();

    let operations = [
        Operation::Push32(U256::zero()),
        Operation::Push32(U256::one()),
        Operation::Div,
        Operation::Stop,
    ];

    let bytecode = operations
        .iter()
        .flat_map(Operation::to_bytecode)
        .collect::<Bytes>();

    vm.execute(bytecode);

    assert!(vm.stack.pop().unwrap() == U256::zero());
}

#[test]
fn sdiv_op() {
    let mut vm = VM::default();

    let operations = [
        Operation::Push32(
            U256::from_str("0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF")
                .unwrap(),
        ),
        Operation::Push32(
            U256::from_str("0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFE")
                .unwrap(),
        ),
        Operation::Sdiv,
        Operation::Stop,
    ];

    let bytecode = operations
        .iter()
        .flat_map(Operation::to_bytecode)
        .collect::<Bytes>();

    vm.execute(bytecode);

    assert!(vm.stack.pop().unwrap() == U256::from(2));
}

#[test]
fn sdiv_op_for_zero() {
    let mut vm = VM::default();

    let operations = [
        Operation::Push32(U256::zero()),
        Operation::Push32(U256::one()),
        Operation::Sdiv,
        Operation::Stop,
    ];

    let bytecode = operations
        .iter()
        .flat_map(Operation::to_bytecode)
        .collect::<Bytes>();

    vm.execute(bytecode);

    assert!(vm.stack.pop().unwrap() == U256::zero());
}

#[test]
fn mod_op() {
    let mut vm = VM::default();

    let operations = [
        Operation::Push32(U256::from(4)),
        Operation::Push32(U256::from(10)),
        Operation::Mod,
        Operation::Stop,
    ];

    let bytecode = operations
        .iter()
        .flat_map(Operation::to_bytecode)
        .collect::<Bytes>();

    vm.execute(bytecode);

    assert!(vm.stack.pop().unwrap() == U256::from(2));
}

#[test]
fn mod_op_for_zero() {
    let mut vm = VM::default();

    let operations = [
        Operation::Push32(U256::zero()),
        Operation::Push32(U256::one()),
        Operation::Mod,
        Operation::Stop,
    ];

    let bytecode = operations
        .iter()
        .flat_map(Operation::to_bytecode)
        .collect::<Bytes>();

    vm.execute(bytecode);

    assert!(vm.stack.pop().unwrap() == U256::zero());
}

#[test]
fn smod_op() {
    let mut vm = VM::default();

    let operations = [
        Operation::Push32(U256::from(0x03)),
        Operation::Push32(U256::from(0x0a)),
        Operation::SMod,
        Operation::Stop,
    ];

    let bytecode = operations
        .iter()
        .flat_map(Operation::to_bytecode)
        .collect::<Bytes>();

    vm.execute(bytecode);

    assert!(vm.stack.pop().unwrap() == U256::one());
}

#[test]
fn smod_op_for_zero() {
    let mut vm = VM::default();

    let operations = [
        Operation::Push32(U256::zero()),
        Operation::Push32(U256::one()),
        Operation::SMod,
        Operation::Stop,
    ];

    let bytecode = operations
        .iter()
        .flat_map(Operation::to_bytecode)
        .collect::<Bytes>();

    vm.execute(bytecode);

    assert!(vm.stack.pop().unwrap() == U256::zero());
}

#[test]
fn addmod_op() {
    let mut vm = VM::default();

    let operations = [
        Operation::Push32(U256::from(8)),
        Operation::Push32(U256::from(0x0a)),
        Operation::Push32(U256::from(0x0a)),
        Operation::Addmod,
        Operation::Stop,
    ];

    let bytecode = operations
        .iter()
        .flat_map(Operation::to_bytecode)
        .collect::<Bytes>();

    vm.execute(bytecode);

    assert!(vm.stack.pop().unwrap() == U256::from(4));
}

#[test]
fn addmod_op_for_zero() {
    let mut vm = VM::default();

    let operations = [
        Operation::Push32(U256::zero()),
        Operation::Push32(U256::from(4)),
        Operation::Push32(U256::from(6)),
        Operation::Addmod,
        Operation::Stop,
    ];

    let bytecode = operations
        .iter()
        .flat_map(Operation::to_bytecode)
        .collect::<Bytes>();

    vm.execute(bytecode);

    assert!(vm.stack.pop().unwrap() == U256::zero());
}

#[test]
fn mulmod_op() {
    let mut vm = VM::default();

    let operations = [
        Operation::Push32(U256::from(4)),
        Operation::Push32(U256::from(2)),
        Operation::Push32(U256::from(5)),
        Operation::Mulmod,
        Operation::Stop,
    ];

    let bytecode = operations
        .iter()
        .flat_map(Operation::to_bytecode)
        .collect::<Bytes>();

    vm.execute(bytecode);

    assert!(vm.stack.pop().unwrap() == U256::from(2));
}

#[test]
fn mulmod_op_for_zero() {
    let mut vm = VM::default();

    let operations = [
        Operation::Push32(U256::zero()),
        Operation::Push32(U256::from(2)),
        Operation::Push32(U256::from(5)),
        Operation::Mulmod,
        Operation::Stop,
    ];

    let bytecode = operations
        .iter()
        .flat_map(Operation::to_bytecode)
        .collect::<Bytes>();

    vm.execute(bytecode);

    assert!(vm.stack.pop().unwrap() == U256::zero());
}

#[test]
fn exp_op() {
    let mut vm = VM::default();

    let operations = [
        Operation::Push32(U256::from(5)),
        Operation::Push32(U256::from(2)),
        Operation::Exp,
        Operation::Stop,
    ];

    let bytecode = operations
        .iter()
        .flat_map(Operation::to_bytecode)
        .collect::<Bytes>();

    vm.execute(bytecode);

    assert!(vm.stack.pop().unwrap() == U256::from(32));
}

#[test]
fn signextend_op_negative() {
    let mut vm = VM::default();

    let operations = [
        Operation::Push32(U256::from(0xff)),
        Operation::Push32(U256::zero()),
        Operation::SignExtend,
        Operation::Stop,
    ];

    let bytecode = operations
        .iter()
        .flat_map(Operation::to_bytecode)
        .collect::<Bytes>();

    vm.execute(bytecode);

    assert!(vm.stack.pop().unwrap() == U256::max_value());
}

#[test]
fn signextend_op_positive() {
    let mut vm = VM::default();

    let operations = [
        Operation::Push32(U256::from(0x7f)),
        Operation::Push32(U256::zero()),
        Operation::SignExtend,
        Operation::Stop,
    ];

    let bytecode = operations
        .iter()
        .flat_map(Operation::to_bytecode)
        .collect::<Bytes>();

    vm.execute(bytecode);

    assert!(vm.stack.pop().unwrap() == U256::from(0x7f));
}
