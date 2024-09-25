use std::str::FromStr;

use ethereum_types::U256;
use levm::{operations::Operation, program::Program, vm::VM};

#[test]
fn add_op() {
    let mut vm = VM::default();

    let operations = vec![
        Operation::Push32(U256::one()),
        Operation::Push32(U256::zero()),
        Operation::Add,
        Operation::Stop,
    ];

    let program = Program::from_operations(operations);

    vm.execute(program);

    assert!(vm.stack.pop().unwrap() == U256::one());
}

#[test]
fn add_op_overflow() {
    let mut vm = VM::default();

    let operations = vec![
        Operation::Push32(U256::max_value()),
        Operation::Push32(U256::max_value()),
        Operation::Add,
        Operation::Stop,
    ];

    let program = Program::from_operations(operations);

    vm.execute(program);

    assert!(vm.stack.pop().unwrap() == (U256::max_value() - U256::one()));
}

#[test]
fn mul_op() {
    let mut vm = VM::default();

    let operations = vec![
        Operation::Push32(U256::from(10)),
        Operation::Push32(U256::from(10)),
        Operation::Mul,
        Operation::Stop,
    ];

    let program = Program::from_operations(operations);

    vm.execute(program);

    assert!(vm.stack.pop().unwrap() == U256::from(10 * 10));
}

#[test]
fn mul_op_overflow() {
    let mut vm = VM::default();

    let operations = vec![
        Operation::Push32(
            U256::from_str("0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFE")
                .unwrap(),
        ),
        Operation::Push32(
            U256::from_str("0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFD")
                .unwrap(),
        ),
        Operation::Mul,
        Operation::Stop,
    ];

    let program = Program::from_operations(operations);

    vm.execute(program);

    assert!(vm.stack.pop().unwrap() == U256::from(6));
}

#[test]
fn sub_op() {
    let mut vm = VM::default();

    let operations = vec![
        Operation::Push32(U256::from(20)),
        Operation::Push32(U256::from(30)),
        Operation::Sub,
        Operation::Stop,
    ];

    let program = Program::from_operations(operations);

    vm.execute(program);

    assert!(vm.stack.pop().unwrap() == U256::from(10));
}
// example from evm.codes -> https://www.evm.codes/playground?fork=cancun&unit=Wei&codeType=Mnemonic&code='~30z~20zSUBz'~PUSH32%20z%5Cn%01z~_
#[test]
fn sub_op_overflow() {
    let mut vm = VM::default();

    let operations = vec![
        Operation::Push32(U256::from(30)),
        Operation::Push32(U256::from(20)),
        Operation::Sub,
        Operation::Stop,
    ];

    let program = Program::from_operations(operations);

    vm.execute(program);

    assert!(
        vm.stack.pop().unwrap()
            == U256::from_str("0xfffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff6")
                .unwrap()
    );
}

#[test]
fn div_op() {
    let mut vm = VM::default();

    let operations = vec![
        Operation::Push32(U256::from(6)),
        Operation::Push32(U256::from(12)),
        Operation::Div,
        Operation::Stop,
    ];

    let program = Program::from_operations(operations);

    vm.execute(program);

    assert!(vm.stack.pop().unwrap() == U256::from(2));
}

#[test]
fn div_op_for_zero() {
    let mut vm = VM::default();

    let operations = vec![
        Operation::Push32(U256::zero()),
        Operation::Push32(U256::one()),
        Operation::Div,
        Operation::Stop,
    ];

    let program = Program::from_operations(operations);

    vm.execute(program);

    assert!(vm.stack.pop().unwrap() == U256::zero());
}

#[test]
fn sdiv_op() {
    let mut vm = VM::default();

    let operations = vec![
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

    let program = Program::from_operations(operations);

    vm.execute(program);

    assert!(vm.stack.pop().unwrap() == U256::from(2));
}

#[test]
fn sdiv_op_for_zero() {
    let mut vm = VM::default();

    let operations = vec![
        Operation::Push32(U256::zero()),
        Operation::Push32(U256::one()),
        Operation::Sdiv,
        Operation::Stop,
    ];

    let program = Program::from_operations(operations);

    vm.execute(program);

    assert!(vm.stack.pop().unwrap() == U256::zero());
}

#[test]
fn mod_op() {
    let mut vm = VM::default();

    let operations = vec![
        Operation::Push32(U256::from(4)),
        Operation::Push32(U256::from(10)),
        Operation::Mod,
        Operation::Stop,
    ];

    let program = Program::from_operations(operations);

    vm.execute(program);

    assert!(vm.stack.pop().unwrap() == U256::from(2));
}

#[test]
fn mod_op_for_zero() {
    let mut vm = VM::default();

    let operations = vec![
        Operation::Push32(U256::zero()),
        Operation::Push32(U256::one()),
        Operation::Mod,
        Operation::Stop,
    ];

    let program = Program::from_operations(operations);

    vm.execute(program);

    assert!(vm.stack.pop().unwrap() == U256::zero());
}

#[test]
fn smod_op() {
    let mut vm = VM::default();

    let operations = vec![
        Operation::Push32(U256::from(0x03)),
        Operation::Push32(U256::from(0x0a)),
        Operation::SMod,
        Operation::Stop,
    ];

    let program = Program::from_operations(operations);

    vm.execute(program);

    assert!(vm.stack.pop().unwrap() == U256::one());
}

#[test]
fn smod_op_for_zero() {
    let mut vm = VM::default();

    let operations = vec![
        Operation::Push32(U256::zero()),
        Operation::Push32(U256::one()),
        Operation::SMod,
        Operation::Stop,
    ];

    let program = Program::from_operations(operations);

    vm.execute(program);

    assert!(vm.stack.pop().unwrap() == U256::zero());
}

#[test]
fn addmod_op() {
    let mut vm = VM::default();

    let operations = vec![
        Operation::Push32(U256::from(8)),
        Operation::Push32(U256::from(0x0a)),
        Operation::Push32(U256::from(0x0a)),
        Operation::Addmod,
        Operation::Stop,
    ];

    let program = Program::from_operations(operations);

    vm.execute(program);

    assert!(vm.stack.pop().unwrap() == U256::from(4));
}

#[test]
fn addmod_op_for_zero() {
    let mut vm = VM::default();

    let operations = vec![
        Operation::Push32(U256::zero()),
        Operation::Push32(U256::from(4)),
        Operation::Push32(U256::from(6)),
        Operation::Addmod,
        Operation::Stop,
    ];

    let program = Program::from_operations(operations);

    vm.execute(program);

    assert!(vm.stack.pop().unwrap() == U256::zero());
}

#[test]
fn addmod_op_big_numbers() {
    let mut vm = VM::default();

    let divisor = U256::max_value() - U256::one();
    let addend = U256::max_value() - U256::one() * 2;
    let augend = U256::max_value() - U256::one() * 3;
    let expected_result = U256::max_value() - U256::one() * 4;

    let operations = vec![
        Operation::Push32(divisor),
        Operation::Push32(addend),
        Operation::Push32(augend),
        Operation::Addmod,
        Operation::Stop,
    ];

    let program = Program::from_operations(operations);

    vm.execute(program);

    assert!(vm.stack.pop().unwrap() == expected_result);
}

#[test]
fn mulmod_op() {
    let mut vm = VM::default();

    let operations = vec![
        Operation::Push32(U256::from(4)),
        Operation::Push32(U256::from(2)),
        Operation::Push32(U256::from(5)),
        Operation::Mulmod,
        Operation::Stop,
    ];

    let program = Program::from_operations(operations);

    vm.execute(program);

    assert!(vm.stack.pop().unwrap() == U256::from(2));
}

#[test]
fn mulmod_op_for_zero() {
    let mut vm = VM::default();

    let operations = vec![
        Operation::Push32(U256::zero()),
        Operation::Push32(U256::from(2)),
        Operation::Push32(U256::from(5)),
        Operation::Mulmod,
        Operation::Stop,
    ];

    let program = Program::from_operations(operations);

    vm.execute(program);

    assert!(vm.stack.pop().unwrap() == U256::zero());
}

#[test]
fn mulmod_op_big_numbers() {
    let mut vm = VM::default();

    let divisor = U256::max_value() - U256::one();
    let multiplicand = U256::max_value() - U256::one() * 2;
    let multiplier = U256::max_value() - U256::one() * 3;
    let expected_result = U256::from(2);

    let operations = vec![
        Operation::Push32(divisor),
        Operation::Push32(multiplicand),
        Operation::Push32(multiplier),
        Operation::Mulmod,
        Operation::Stop,
    ];

    let program = Program::from_operations(operations);

    vm.execute(program);

    assert!(vm.stack.pop().unwrap() == expected_result);
}

// example from evm codes -> https://www.evm.codes/playground?fork=cancun&unit=Wei&codeType=Mnemonic&code='zvEwzvDwz~~0000wMULMOD'~yyyyzPUSH32%200x~vFFFw%5Cnv~~y%01vwyz~_
#[test]
fn mulmod_op_big_numbers_result_bigger_than_one_byte() {
    let mut vm = VM::default();

    let divisor = U256::max_value() - U256::one();
    let multiplicand = U256::max_value() - U256::one() * 2;
    let multiplier =
        U256::from_str("0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF0000")
            .unwrap();
    let expected_result = U256::from(0xfffe);

    let operations = vec![
        Operation::Push32(divisor),
        Operation::Push32(multiplicand),
        Operation::Push32(multiplier),
        Operation::Mulmod,
        Operation::Stop,
    ];

    let program = Program::from_operations(operations);

    vm.execute(program);

    assert!(vm.stack.pop().unwrap() == expected_result);
}

#[test]
fn exp_op() {
    let mut vm = VM::default();

    let operations = vec![
        Operation::Push32(U256::from(5)),
        Operation::Push32(U256::from(2)),
        Operation::Exp,
        Operation::Stop,
    ];

    let program = Program::from_operations(operations);

    vm.execute(program);

    assert!(vm.stack.pop().unwrap() == U256::from(32));
}

#[test]
fn exp_op_overflow() {
    let mut vm = VM::default();

    let operations = vec![
        Operation::Push32(U256::from(257)),
        Operation::Push32(U256::from(2)),
        Operation::Exp,
        Operation::Stop,
    ];

    let program = Program::from_operations(operations);

    vm.execute(program);

    assert!(vm.stack.pop().unwrap() == U256::zero());
}

#[test]
fn signextend_op_negative() {
    let mut vm = VM::default();

    let operations = vec![
        Operation::Push32(U256::from(0xff)),
        Operation::Push32(U256::zero()),
        Operation::SignExtend,
        Operation::Stop,
    ];

    let program = Program::from_operations(operations);

    vm.execute(program);

    assert!(vm.stack.pop().unwrap() == U256::max_value());
}

#[test]
fn signextend_op_positive() {
    let mut vm = VM::default();

    let operations = vec![
        Operation::Push32(U256::from(0x7f)),
        Operation::Push32(U256::zero()),
        Operation::SignExtend,
        Operation::Stop,
    ];

    let program = Program::from_operations(operations);

    vm.execute(program);

    assert!(vm.stack.pop().unwrap() == U256::from(0x7f));
}

#[test]
fn mstore() {
    let mut vm = VM::default();

    vm.stack.push(U256::from(0x33333)); // value
    vm.stack.push(U256::from(0)); // offset

    let operations = vec![Operation::Mstore, Operation::Msize, Operation::Stop];

    let program = Program::from_operations(operations);

    vm.execute(program);

    let stored_value = vm.memory.load(0);

    assert_eq!(stored_value, U256::from(0x33333));

    let memory_size = vm.stack.pop().unwrap();
    assert_eq!(memory_size, U256::from(32));
}

#[test]
fn mstore8() {
    let mut vm = VM::default();

    vm.stack.push(U256::from(0xAB)); // value
    vm.stack.push(U256::from(0)); // offset

    let operations = vec![Operation::Mstore8, Operation::Stop];

    let program = Program::from_operations(operations);

    vm.execute(program);

    let stored_value = vm.memory.load(0);

    let mut value_bytes = [0u8; 32];
    stored_value.to_big_endian(&mut value_bytes);

    assert_eq!(value_bytes[0..1], [0xAB]);
}

#[test]
fn mcopy() {
    let mut vm = VM::default();

    vm.stack.push(U256::from(32)); // size
    vm.stack.push(U256::from(0)); // source offset
    vm.stack.push(U256::from(64)); // destination offset

    vm.stack.push(U256::from(0x33333)); // value
    vm.stack.push(U256::from(0)); // offset

    let operations = vec![
        Operation::Mstore,
        Operation::Mcopy,
        Operation::Msize,
        Operation::Stop,
    ];

    let program = Program::from_operations(operations);
    vm.execute(program);

    let copied_value = vm.memory.load(64);
    assert_eq!(copied_value, U256::from(0x33333));

    let memory_size = vm.stack.pop().unwrap();
    assert_eq!(memory_size, U256::from(96));
}

#[test]
fn mload() {
    let mut vm = VM::default();

    vm.stack.push(U256::from(0)); // offset to load

    vm.stack.push(U256::from(0x33333)); // value to store
    vm.stack.push(U256::from(0)); // offset to store

    let operations = vec![Operation::Mstore, Operation::Mload, Operation::Stop];

    let program = Program::from_operations(operations);

    vm.execute(program);

    let loaded_value = vm.stack.pop().unwrap();
    assert_eq!(loaded_value, U256::from(0x33333));
}

#[test]
fn msize() {
    let mut vm = VM::default();

    let operations = vec![Operation::Msize, Operation::Stop];

    let program = Program::from_operations(operations);

    vm.execute(program);

    let initial_size = vm.stack.pop().unwrap();
    assert_eq!(initial_size, U256::from(0));

    vm.pc = 0;

    vm.stack.push(U256::from(0x33333)); // value
    vm.stack.push(U256::from(0)); // offset

    let operations = vec![Operation::Mstore, Operation::Msize, Operation::Stop];

    let program = Program::from_operations(operations);

    vm.execute(program);

    let after_store_size = vm.stack.pop().unwrap();
    assert_eq!(after_store_size, U256::from(32));

    vm.pc = 0;

    vm.stack.push(U256::from(0x55555)); // value
    vm.stack.push(U256::from(64)); // offset

    let operations = vec![Operation::Mstore, Operation::Msize, Operation::Stop];

    let program = Program::from_operations(operations);

    vm.execute(program);

    let final_size = vm.stack.pop().unwrap();
    assert_eq!(final_size, U256::from(96));
}

#[test]
fn mstore_mload_offset_not_multiple_of_32() {
    let mut vm = VM::default();

    vm.stack.push(U256::from(10)); // offset

    vm.stack.push(U256::from(0xabcdef)); // value
    vm.stack.push(U256::from(10)); // offset

    let operations = vec![
        Operation::Mstore,
        Operation::Mload,
        Operation::Msize,
        Operation::Stop,
    ];

    let program = Program::from_operations(operations);

    vm.execute(program);

    let memory_size = vm.stack.pop().unwrap();
    let loaded_value = vm.stack.pop().unwrap();

    assert_eq!(loaded_value, U256::from(0xabcdef));
    assert_eq!(memory_size, U256::from(64));

    //check with big offset

    vm.pc = 0;

    vm.stack.push(U256::from(2000)); // offset

    vm.stack.push(U256::from(0x123456)); // value
    vm.stack.push(U256::from(2000)); // offset

    let operations = vec![
        Operation::Mstore,
        Operation::Mload,
        Operation::Msize,
        Operation::Stop,
    ];

    let program = Program::from_operations(operations);

    vm.execute(program);

    let memory_size = vm.stack.pop().unwrap();
    let loaded_value = vm.stack.pop().unwrap();

    assert_eq!(loaded_value, U256::from(0x123456));
    assert_eq!(memory_size, U256::from(2048));
}

#[test]
fn mload_uninitialized_memory() {
    let mut vm = VM::default();

    vm.stack.push(U256::from(50)); // offset

    let operations = vec![Operation::Mload, Operation::Msize, Operation::Stop];

    let program = Program::from_operations(operations);

    vm.execute(program);

    let memory_size = vm.stack.pop().unwrap();
    let loaded_value = vm.stack.pop().unwrap();

    assert_eq!(loaded_value, U256::zero());
    assert_eq!(memory_size, U256::from(96));
}

#[test]
fn pc_op() {
    let mut vm = VM::default();

    let operations = vec![Operation::PC, Operation::Stop];

    let program = Program::from_operations(operations);

    vm.execute(program);

    assert!(vm.stack.pop().unwrap() == U256::from(0));
}

#[test]
fn pc_op_with_push_offset() {
    let mut vm = VM::default();

    let operations = vec![
        Operation::Push32(U256::one()),
        Operation::PC,
        Operation::Stop,
    ];

    let program = Program::from_operations(operations);

    vm.execute(program);

    assert!(vm.stack.pop().unwrap() == U256::from(33));
}

#[test]
fn jump_op() {
    let mut vm = VM::default();

    let operations = vec![
        Operation::Push32(U256::from(35)),
        Operation::Jump,
        Operation::Stop, // should skip this one
        Operation::Jumpdest,
        Operation::Push32(U256::from(10)),
        Operation::Stop,
    ];

    let program = Program::from_operations(operations);

    vm.execute(program);

    assert!(vm.stack.pop().unwrap() == U256::from(10));
    assert_eq!(vm.pc(), 70);
}

#[test]
fn jump_not_jumpdest_position() {
    let mut vm = VM::default();

    let operations = vec![
        Operation::Push32(U256::from(36)),
        Operation::Jump,
        Operation::Stop, // should go here
        Operation::Push32(U256::from(10)),
        Operation::Stop,
    ];

    let program = Program::from_operations(operations);

    vm.execute(program);
    assert_eq!(vm.pc(), 35);
}

#[test]
fn jump_position_bigger_than_program_size() {
    let mut vm = VM::default();

    let operations = vec![
        Operation::Push32(U256::from(5000)),
        Operation::Jump,
        Operation::Stop, // should skip this one
        Operation::Push32(U256::from(10)),
        Operation::Stop,
    ];

    let program = Program::from_operations(operations);

    vm.execute(program);
    assert_eq!(vm.pc(), 35);
}

#[test]
fn jumpi_not_zero() {
    let mut vm = VM::default();

    let operations = vec![
        Operation::Push32(U256::one()),
        Operation::Push32(U256::from(67)),
        Operation::Jumpi,
        Operation::Stop, // should skip this one
        Operation::Jumpdest,
        Operation::Push32(U256::from(10)),
        Operation::Stop,
    ];

    let program = Program::from_operations(operations);

    vm.execute(program);

    assert!(vm.stack.pop().unwrap() == U256::from(10));
}

#[test]
fn jumpi_for_zero() {
    let mut vm = VM::default();

    let operations = vec![
        Operation::Push32(U256::from(100)),
        Operation::Push32(U256::zero()),
        Operation::Push32(U256::from(100)),
        Operation::Jumpi,
        Operation::Stop, // should skip this one
        Operation::Jumpdest,
        Operation::Push32(U256::from(10)),
        Operation::Stop,
    ];

    let program = Program::from_operations(operations);

    vm.execute(program);

    assert!(vm.stack.pop().unwrap() == U256::from(100));
}
