use std::str::FromStr;

use bytes::Bytes;
use ethereum_types::U256;
use levm::{opcodes::Opcode, operations::Operation, vm::VM};

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
fn add_op_overflow() {
    let mut vm = VM::default();

    let operations = [
        Operation::Push32(U256::max_value()),
        Operation::Push32(U256::max_value()),
        Operation::Add,
        Operation::Stop,
    ];

    let bytecode = operations
        .iter()
        .flat_map(Operation::to_bytecode)
        .collect::<Bytes>();

    vm.execute(bytecode);

    assert!(vm.stack.pop().unwrap() == (U256::max_value() - U256::one()));
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
fn mul_op_overflow() {
    let mut vm = VM::default();

    let operations = [
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

    let bytecode = operations
        .iter()
        .flat_map(Operation::to_bytecode)
        .collect::<Bytes>();

    vm.execute(bytecode);

    assert!(vm.stack.pop().unwrap() == U256::from(6));
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
// example from evm.codes -> https://www.evm.codes/playground?fork=cancun&unit=Wei&codeType=Mnemonic&code='~30z~20zSUBz'~PUSH32%20z%5Cn%01z~_
#[test]
fn sub_op_overflow() {
    let mut vm = VM::default();

    let operations = [
        Operation::Push32(U256::from(30)),
        Operation::Push32(U256::from(20)),
        Operation::Sub,
        Operation::Stop,
    ];

    let bytecode = operations
        .iter()
        .flat_map(Operation::to_bytecode)
        .collect::<Bytes>();

    vm.execute(bytecode);

    assert!(
        vm.stack.pop().unwrap()
            == U256::from_str("0xfffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff6")
                .unwrap()
    );
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
fn addmod_op_big_numbers() {
    let mut vm = VM::default();

    let divisor = U256::max_value() - U256::one();
    let addend = U256::max_value() - U256::one() * 2;
    let augend = U256::max_value() - U256::one() * 3;
    let expected_result = U256::max_value() - U256::one() * 4;

    let operations = [
        Operation::Push32(divisor),
        Operation::Push32(addend),
        Operation::Push32(augend),
        Operation::Addmod,
        Operation::Stop,
    ];

    let bytecode = operations
        .iter()
        .flat_map(Operation::to_bytecode)
        .collect::<Bytes>();

    vm.execute(bytecode);

    assert!(vm.stack.pop().unwrap() == expected_result);
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
fn mulmod_op_big_numbers() {
    let mut vm = VM::default();

    let divisor = U256::max_value() - U256::one();
    let multiplicand = U256::max_value() - U256::one() * 2;
    let multiplier = U256::max_value() - U256::one() * 3;
    let expected_result = U256::from(2);

    let operations = [
        Operation::Push32(divisor),
        Operation::Push32(multiplicand),
        Operation::Push32(multiplier),
        Operation::Mulmod,
        Operation::Stop,
    ];

    let bytecode = operations
        .iter()
        .flat_map(Operation::to_bytecode)
        .collect::<Bytes>();

    vm.execute(bytecode);

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

    let operations = [
        Operation::Push32(divisor),
        Operation::Push32(multiplicand),
        Operation::Push32(multiplier),
        Operation::Mulmod,
        Operation::Stop,
    ];

    let bytecode = operations
        .iter()
        .flat_map(Operation::to_bytecode)
        .collect::<Bytes>();

    vm.execute(bytecode);

    assert!(vm.stack.pop().unwrap() == expected_result);
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
fn exp_op_overflow() {
    let mut vm = VM::default();

    let operations = [
        Operation::Push32(U256::from(257)),
        Operation::Push32(U256::from(2)),
        Operation::Exp,
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

#[test]
fn lt_lho_less_than_rho() {
    let mut vm = VM::default();

    let operations = [
        Operation::Push32(U256::one()),  // rho
        Operation::Push32(U256::zero()), // lho
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
fn lt_lho_equals_rho() {
    let mut vm = VM::default();

    let operations = [
        Operation::Push32(U256::zero()), // rho
        Operation::Push32(U256::zero()), // lho
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
fn lt_lho_greater_than_rho() {
    let mut vm = VM::default();

    let operations = [
        Operation::Push32(U256::zero()), // rho
        Operation::Push32(U256::one()),  // lho
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
fn gt_lho_greater_than_rho() {
    let mut vm = VM::default();

    let operations = [
        Operation::Push32(U256::zero()), // rho
        Operation::Push32(U256::one()),  // lho
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
fn gt_lho_equals_rho() {
    let mut vm = VM::default();

    let operations = [
        Operation::Push32(U256::zero()), // rho
        Operation::Push32(U256::zero()), // lho
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
fn gt_lho_less_than_rho() {
    let mut vm = VM::default();

    let operations = [
        Operation::Push32(U256::one()),  // rho
        Operation::Push32(U256::zero()), // lho
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
fn slt_zero_lho_less_than_positive_rho() {
    let mut vm = VM::default();

    let operations = [
        Operation::Push32(U256::one()),  // rho
        Operation::Push32(U256::zero()), // lho
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
fn slt_long_lho_less_than_positive_rho() {
    let mut vm = VM::default();
    let lho = U256::from("0x0100000000000000000000000000000000000000000000000000000000000000");
    let operations = [
        Operation::Push32(U256::one()), // rho
        Operation::Push32(lho),         // lho
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
fn slt_negative_lho_less_than_positive_rho() {
    let mut vm = VM::default();

    let operations = [
        Operation::Push32(U256::one()),            // rho
        Operation::Push32(U256::from([0xff; 32])), // lho = -1
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
fn slt_negative_lho_less_than_negative_rho() {
    let mut vm = VM::default();

    let operations = [
        Operation::Push32(U256::from([0xff; 32])), // rho = -1
        Operation::Push32(U256::from([0xff; 32]).saturating_sub(U256::one())), // lho = -2
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
fn slt_zero_lho_greater_than_negative_rho() {
    let mut vm = VM::default();

    let operations = [
        Operation::Push32(U256::from([0xff; 32])), // rho = -1
        Operation::Push32(U256::zero()),           // lho
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
fn slt_positive_lho_greater_than_negative_rho() {
    let mut vm = VM::default();

    let operations = [
        Operation::Push32(U256::from([0xff; 32])), // rho = -1
        Operation::Push32(U256::one()),            // lho
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
fn sgt_positive_lho_greater_than_zero_rho() {
    let mut vm = VM::default();

    let operations = [
        Operation::Push32(U256::zero()), // rho
        Operation::Push32(U256::one()),  // lho
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
fn sgt_positive_lho_greater_than_negative_rho() {
    let mut vm = VM::default();

    let operations = [
        Operation::Push32(U256::from([0xff; 32])), // rho = -1
        Operation::Push32(U256::one()),            // lho
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
fn sgt_negative_lho_greater_than_negative_rho() {
    let mut vm = VM::default();

    let operations = [
        Operation::Push32(U256::from([0xff; 32]).saturating_sub(U256::one())), // rho = -2
        Operation::Push32(U256::from([0xff; 32])),                             // lho = -1
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
fn sgt_negative_lho_less_than_positive_rho() {
    let mut vm = VM::default();

    let operations = [
        Operation::Push32(U256::one()),            // rho
        Operation::Push32(U256::from([0xff; 32])), // lho = -1
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

#[test]
fn eq_lho_equals_rho() {
    let mut vm = VM::default();

    let operations = [
        Operation::Push32(U256::zero()), // rho
        Operation::Push32(U256::zero()), // lho
        Operation::Eq,
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
fn eq_lho_not_equals_rho() {
    let mut vm = VM::default();

    let operations = [
        Operation::Push32(U256::zero()), // rho
        Operation::Push32(U256::one()),  // lho
        Operation::Eq,
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
fn iszero_operand_is_zero() {
    let mut vm = VM::default();

    let operations = [
        Operation::Push32(U256::zero()),
        Operation::IsZero,
        Operation::Stop,
    ];

    let bytecode = operations
        .iter()
        .flat_map(Operation::to_bytecode)
        .collect::<Bytes>();

    vm.execute(bytecode);

    assert!(vm.stack.pop().unwrap() == U256::one());
    assert!(vm.pc() == 35);
}

#[test]
fn iszero_operand_is_not_zero() {
    let mut vm = VM::default();

    let operations = [
        Operation::Push32(U256::one()),
        Operation::IsZero,
        Operation::Stop,
    ];

    let bytecode = operations
        .iter()
        .flat_map(Operation::to_bytecode)
        .collect::<Bytes>();

    vm.execute(bytecode);

    assert!(vm.stack.pop().unwrap() == U256::zero());
    assert!(vm.pc() == 35);
}

#[test]
fn keccak256_zero_offset_size_four() {
    let mut vm = VM::default();

    let operations = [
        // Put the required value in memory
        Operation::Push32(U256::from(
            "0xFFFFFFFF00000000000000000000000000000000000000000000000000000000",
        )),
        Operation::Push0,
        Operation::Mstore,
        // Call the opcode
        Operation::Push((1, 4.into())), // size
        Operation::Push0,               // offset
        Operation::Keccak256,
        Operation::Stop,
    ];

    let bytecode = operations
        .iter()
        .flat_map(Operation::to_bytecode)
        .collect::<Bytes>();

    vm.execute(bytecode);

    assert!(
        vm.stack.pop().unwrap()
            == U256::from("0x29045a592007d0c246ef02c2223570da9522d0cf0f73282c79a1bc8f0bb2c238")
    );
    assert!(vm.pc() == 40);
}

#[test]
fn keccak256_zero_offset_size_bigger_than_actual_memory() {
    let mut vm = VM::default();

    let operations = [
        // Put the required value in memory
        Operation::Push32(U256::from(
            "0xFFFFFFFF00000000000000000000000000000000000000000000000000000000",
        )),
        Operation::Push0,
        Operation::Mstore,
        // Call the opcode
        Operation::Push((1, 33.into())), // size > memory.data.len() (32)
        Operation::Push0,                // offset
        Operation::Keccak256,
        Operation::Stop,
    ];

    let bytecode = operations
        .iter()
        .flat_map(Operation::to_bytecode)
        .collect::<Bytes>();

    vm.execute(bytecode);

    assert!(
        vm.stack.pop().unwrap()
            == U256::from("0xae75624a7d0413029c1e0facdd38cc8e177d9225892e2490a69c2f1f89512061")
    );
    assert!(vm.pc() == 40);
}

#[test]
fn keccak256_zero_offset_zero_size() {
    let mut vm = VM::default();

    let operations = [
        Operation::Push0, // size
        Operation::Push0, // offset
        Operation::Keccak256,
        Operation::Stop,
    ];

    let bytecode = operations
        .iter()
        .flat_map(Operation::to_bytecode)
        .collect::<Bytes>();

    vm.execute(bytecode);

    assert!(
        vm.stack.pop().unwrap()
            == U256::from("0xc5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470")
    );
    assert!(vm.pc() == 4);
}

#[test]
fn keccak256_offset_four_size_four() {
    let mut vm = VM::default();

    let operations = [
        // Put the required value in memory
        Operation::Push32(U256::from(
            "0xFFFFFFFF00000000000000000000000000000000000000000000000000000000",
        )),
        Operation::Push0,
        Operation::Mstore,
        // Call the opcode
        Operation::Push((1, 4.into())), // size
        Operation::Push((1, 4.into())), // offset
        Operation::Keccak256,
        Operation::Stop,
    ];

    let bytecode = operations
        .iter()
        .flat_map(Operation::to_bytecode)
        .collect::<Bytes>();

    vm.execute(bytecode);

    assert!(
        vm.stack.pop().unwrap()
            == U256::from("0xe8e77626586f73b955364c7b4bbf0bb7f7685ebd40e852b164633a4acbd3244c")
    );
    assert!(vm.pc() == 41);
}

#[test]
fn mstore() {
    let mut vm = VM::default();

    vm.stack.push(U256::from(0x33333)); // value
    vm.stack.push(U256::from(0)); // offset

    vm.execute(Bytes::from(vec![
        Opcode::MSTORE as u8,
        Opcode::MSIZE as u8,
        Opcode::STOP as u8,
    ]));

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

    vm.execute(Bytes::from(vec![Opcode::MSTORE8 as u8, Opcode::STOP as u8]));

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

    vm.execute(Bytes::from(vec![
        Opcode::MSTORE as u8,
        Opcode::MCOPY as u8,
        Opcode::MSIZE as u8,
        Opcode::STOP as u8,
    ]));

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

    vm.execute(Bytes::from(vec![
        Opcode::MSTORE as u8,
        Opcode::MLOAD as u8,
        Opcode::STOP as u8,
    ]));

    let loaded_value = vm.stack.pop().unwrap();
    assert_eq!(loaded_value, U256::from(0x33333));
}

#[test]
fn msize() {
    let mut vm = VM::default();

    vm.execute(Bytes::from(vec![Opcode::MSIZE as u8, Opcode::STOP as u8]));
    let initial_size = vm.stack.pop().unwrap();
    assert_eq!(initial_size, U256::from(0));

    vm.pc = 0;

    vm.stack.push(U256::from(0x33333)); // value
    vm.stack.push(U256::from(0)); // offset

    vm.execute(Bytes::from(vec![
        Opcode::MSTORE as u8,
        Opcode::MSIZE as u8,
        Opcode::STOP as u8,
    ]));

    let after_store_size = vm.stack.pop().unwrap();
    assert_eq!(after_store_size, U256::from(32));

    vm.pc = 0;

    vm.stack.push(U256::from(0x55555)); // value
    vm.stack.push(U256::from(64)); // offset

    vm.execute(Bytes::from(vec![
        Opcode::MSTORE as u8,
        Opcode::MSIZE as u8,
        Opcode::STOP as u8,
    ]));

    let final_size = vm.stack.pop().unwrap();
    assert_eq!(final_size, U256::from(96));
}

#[test]
fn mstore_mload_offset_not_multiple_of_32() {
    let mut vm = VM::default();

    vm.stack.push(U256::from(10)); // offset

    vm.stack.push(U256::from(0xabcdef)); // value
    vm.stack.push(U256::from(10)); // offset

    vm.execute(Bytes::from(vec![
        Opcode::MSTORE as u8,
        Opcode::MLOAD as u8,
        Opcode::MSIZE as u8,
        Opcode::STOP as u8,
    ]));

    let memory_size = vm.stack.pop().unwrap();
    let loaded_value = vm.stack.pop().unwrap();

    assert_eq!(loaded_value, U256::from(0xabcdef));
    assert_eq!(memory_size, U256::from(64));

    //check with big offset

    vm.pc = 0;

    vm.stack.push(U256::from(2000)); // offset

    vm.stack.push(U256::from(0x123456)); // value
    vm.stack.push(U256::from(2000)); // offset

    vm.execute(Bytes::from(vec![
        Opcode::MSTORE as u8,
        Opcode::MLOAD as u8,
        Opcode::MSIZE as u8,
        Opcode::STOP as u8,
    ]));

    let memory_size = vm.stack.pop().unwrap();
    let loaded_value = vm.stack.pop().unwrap();

    assert_eq!(loaded_value, U256::from(0x123456));
    assert_eq!(memory_size, U256::from(2048));
}

#[test]
fn mload_uninitialized_memory() {
    let mut vm = VM::default();

    vm.stack.push(U256::from(50)); // offset

    vm.execute(Bytes::from(vec![
        Opcode::MLOAD as u8,
        Opcode::MSIZE as u8,
        Opcode::STOP as u8,
    ]));

    let memory_size = vm.stack.pop().unwrap();
    let loaded_value = vm.stack.pop().unwrap();

    assert_eq!(loaded_value, U256::zero());
    assert_eq!(memory_size, U256::from(96));
}
