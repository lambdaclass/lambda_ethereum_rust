//! Tests for simple EVM operations
//!
//! These don't receive any input, and the CODE* opcodes
//! may not work properly.
use ethereum_rust_levm_mlir::{
    constants::gas_cost::{self, log_dynamic_gas_cost},
    context::Context,
    db::Db,
    env::Env,
    executor::Executor,
    journal::Journal,
    primitives::Bytes,
    program::{Operation, Program},
    result::{ExecutionResult, HaltReason, Output, SuccessReason},
    syscall::SyscallContext,
};
use hex_literal::hex;
use num_bigint::{BigInt, BigUint};
use rstest::rstest;

fn run_program_get_result_with_gas(
    operations: Vec<Operation>,
    initial_gas: u64,
) -> ExecutionResult {
    // Insert a return operation at the end of the program to verify top of stack.
    let program = Program::from(operations);

    let context = Context::new();
    let module = context
        .compile(&program, Default::default())
        .expect("failed to compile program");

    let mut env = Env::default();
    env.tx.gas_limit = initial_gas;
    let mut db = Db::default();
    let journal = Journal::new(&mut db).with_prefetch(&env.tx.access_list);
    let mut context = SyscallContext::new(env, journal, Default::default(), initial_gas);
    let executor = Executor::new(&module, &context, Default::default());

    let _result = executor.execute(&mut context, initial_gas);

    context.get_result().unwrap().result
}

fn run_program_assert_result(operations: Vec<Operation>, expected_result: &[u8]) {
    let result = run_program_get_result_with_gas(operations, 1e7 as _);
    assert!(result.is_success());
    assert_eq!(result.output().unwrap_or(&Bytes::new()), expected_result);
}

fn run_program_assert_stack_top(operations: Vec<Operation>, expected_result: BigUint) {
    run_program_assert_stack_top_with_gas(operations, expected_result, 1e7 as _)
}

fn run_program_assert_stack_top_with_gas(
    mut operations: Vec<Operation>,
    expected_result: BigUint,
    initial_gas: u64,
) {
    // NOTE: modifying this will break codesize related tests
    operations.extend([
        Operation::Push0,
        Operation::Mstore,
        Operation::Push((1, 32_u8.into())),
        Operation::Push0,
        Operation::Return,
    ]);
    let mut result_bytes = [0_u8; 32];
    if expected_result != BigUint::ZERO {
        let bytes = expected_result.to_bytes_be();
        result_bytes[32 - bytes.len()..].copy_from_slice(&bytes);
    }
    let result = run_program_get_result_with_gas(operations, initial_gas);
    assert!(result.is_success());
    assert_eq!(result.output().unwrap().as_ref(), result_bytes);
}

fn run_program_assert_halt(program: Vec<Operation>) {
    let result = run_program_get_result_with_gas(program, 1e7 as _);
    assert!(result.is_halt());
}

fn run_program_assert_revert(program: Vec<Operation>, expected_result: &[u8]) {
    let result = run_program_get_result_with_gas(program, 1e7 as _);
    assert!(result.is_revert());
    assert_eq!(result.output().unwrap(), expected_result);
}

fn run_program_assert_gas_exact(program: Vec<Operation>, expected_gas: u64) {
    let result = run_program_get_result_with_gas(program.clone(), expected_gas);
    assert!(result.is_success());

    let result = run_program_get_result_with_gas(program, expected_gas - 1);
    assert!(result.is_halt());
}

pub fn biguint_256_from_bigint(value: BigInt) -> BigUint {
    if value >= BigInt::ZERO {
        value.magnitude().clone()
    } else {
        let bytes = value.to_signed_bytes_be();
        let mut buffer = vec![255_u8; 32];
        let finish = 32;
        let start = finish - bytes.len();
        buffer[start..finish].copy_from_slice(&bytes);
        BigUint::from_bytes_be(&buffer)
    }
}

#[test]
fn test_keccak256() {
    let program = vec![
        Operation::Push((1, BigUint::from(0x00_u8))),
        Operation::Push((1, BigUint::from(0x00_u8))),
        Operation::Keccak256,
    ];
    let expected = hex!("c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470");
    run_program_assert_stack_top(program, BigUint::from_bytes_be(&expected));
}

#[test]
fn test_keccak_with_mstore() {
    let program = vec![
        Operation::Push((
            32,
            BigUint::from_bytes_be(&hex!(
                "FFFFFFFF00000000000000000000000000000000000000000000000000000000"
            )),
        )),
        Operation::Push((1, BigUint::from(0_u8))),
        Operation::Mstore,
        Operation::Push((1, BigUint::from(4_u8))),
        Operation::Push((1, BigUint::from(0_u8))),
        Operation::Keccak256,
    ];
    let expected = hex!("29045a592007d0c246ef02c2223570da9522d0cf0f73282c79a1bc8f0bb2c238");
    run_program_assert_stack_top(program, BigUint::from_bytes_be(&expected));
}

#[test]
fn test_keccak256_with_size() {
    let program = vec![
        Operation::Push((1, BigUint::from(0x04_u8))),
        Operation::Push((1, BigUint::from(0x00_u8))),
        Operation::Keccak256,
    ];
    let expected = hex!("e8e77626586f73b955364c7b4bbf0bb7f7685ebd40e852b164633a4acbd3244c");
    run_program_assert_stack_top(program, BigUint::from_bytes_be(&expected));
}

#[test]
fn test_keccak_with_overflow() {
    let program = vec![Operation::Push((1_u8, BigUint::from(88_u8))); 1025];
    run_program_assert_halt(program);
}

#[test]
fn test_keccak_with_underflow() {
    let program = vec![Operation::Keccak256];
    run_program_assert_halt(program);
}

#[test]
fn test_keccak_gas_cost() {
    let offset = 0_u8;
    let size = 4_u8;
    let program = vec![
        Operation::Push((1, size.into())),
        Operation::Push((1, offset.into())),
        Operation::Keccak256,
    ];
    let dynamic_gas = gas_cost::memory_expansion_cost(0, 32) + 2 * gas_cost::memory_copy_cost(32);
    let static_gas = gas_cost::KECCAK256 + 2 * gas_cost::PUSHN;
    let gas_needed = static_gas + dynamic_gas;

    run_program_assert_gas_exact(program, gas_needed as _);
}

#[test]
fn test_calldatasize_with_gas() {
    let program = vec![Operation::CallDataSize];
    run_program_assert_stack_top(program, BigUint::ZERO);
}

#[test]
fn test_return_with_gas() {
    let program = vec![
        Operation::Push((1, 1_u8.into())),
        Operation::Push((1, 2_u8.into())),
        Operation::Return,
    ];
    let dynamic_gas = gas_cost::memory_expansion_cost(0, 32);
    let needed_gas = gas_cost::PUSHN * 2 + dynamic_gas;

    run_program_assert_gas_exact(program, needed_gas as _);
}

#[test]
fn test_revert_with_gas() {
    let program = vec![
        Operation::Push((1, 1_u8.into())),
        Operation::Push((1, 2_u8.into())),
        Operation::Revert,
    ];
    let dynamic_gas = gas_cost::memory_expansion_cost(0, 32);
    let needed_gas = gas_cost::PUSHN * 2 + dynamic_gas;

    // When gas is not enough, exits as halt instead of revert.
    let result = run_program_get_result_with_gas(program.clone(), (needed_gas - 1) as _);
    assert!(result.is_halt());

    run_program_assert_revert(program, &[0]);
}

#[test]
fn push_once() {
    let value = BigUint::from(5_u8);

    // For OPERATION::PUSH0
    let program = vec![Operation::Push0];
    run_program_assert_stack_top(program, BigUint::ZERO);

    // For OPERATION::PUSH1, ... , OPERATION::PUSH32
    for i in 0..32 {
        let shifted_value: BigUint = value.clone() << (i * 8);
        let program = vec![Operation::Push((i, shifted_value.clone()))];
        run_program_assert_stack_top(program, shifted_value.clone());
    }
}

#[test]
fn push_twice() {
    let the_answer = BigUint::from(42_u8);

    let program = vec![
        Operation::Push((1_u8, BigUint::from(1_u8))),
        Operation::Push((1_u8, the_answer.clone())),
    ];
    run_program_assert_stack_top(program, the_answer);
}

#[test]
fn push_fill_stack() {
    let stack_top = BigUint::from(88_u8);

    // Push 1024 times
    let program = vec![Operation::Push((1_u8, stack_top.clone())); 1024];
    run_program_assert_result(program, &[]);
}

#[test]
fn push_stack_overflow() {
    // Push 1025 times
    let program = vec![Operation::Push((1_u8, BigUint::from(88_u8))); 1025];
    run_program_assert_halt(program);
}

#[test]
fn push_reverts_without_gas() {
    let stack_top = 88_u8;
    let initial_gas = (gas_cost::PUSH0 + gas_cost::PUSHN) as _;

    let program = vec![
        Operation::Push0,
        Operation::Push((1_u8, BigUint::from(stack_top))),
    ];
    run_program_assert_gas_exact(program, initial_gas);
}

#[test]
fn dup1_once() {
    let program = vec![
        Operation::Push((1_u8, BigUint::from(10_u8))),
        Operation::Push((1_u8, BigUint::from(31_u8))),
        Operation::Dup(1),
        Operation::Pop,
    ];

    run_program_assert_stack_top(program, 31_u8.into());
}

#[test]
fn dup2_once() {
    let program = vec![
        Operation::Push((1_u8, BigUint::from(4_u8))),
        Operation::Push((1_u8, BigUint::from(5_u8))),
        Operation::Push((1_u8, BigUint::from(6_u8))),
        Operation::Dup(2),
    ];

    run_program_assert_stack_top(program, 5_u8.into());
}

#[rstest]
#[case(1)]
#[case(2)]
#[case(3)]
#[case(4)]
#[case(5)]
#[case(6)]
#[case(7)]
#[case(8)]
#[case(9)]
#[case(10)]
#[case(11)]
#[case(12)]
#[case(13)]
#[case(14)]
#[case(15)]
#[case(16)]
fn dup_nth(#[case] nth: u8) {
    let iter = (0..16u8)
        .rev()
        .map(|x| Operation::Push((1_u8, BigUint::from(x))));
    let mut program = Vec::from_iter(iter);

    program.push(Operation::Dup(nth));

    run_program_assert_stack_top(program, (nth - 1).into());
}

#[test]
fn dup_with_stack_underflow() {
    let program = vec![Operation::Dup(1)];

    run_program_assert_halt(program);
}

#[test]
fn dup_out_of_gas() {
    let a = BigUint::from(2_u8);
    let program = vec![Operation::Push((1_u8, a.clone())), Operation::Dup(1)];
    let gas_needed = gas_cost::PUSHN + gas_cost::DUPN;

    run_program_assert_gas_exact(program, gas_needed as _);
}

#[test]
fn push_push_shl() {
    let program = vec![
        Operation::Push((1_u8, BigUint::from(1_u8))),
        Operation::Push((1_u8, BigUint::from(4_u8))),
        Operation::Shl,
    ];

    run_program_assert_stack_top(program, 16_u8.into());
}

#[test]
fn shl_shift_grater_than_255() {
    let program = vec![
        Operation::Push((1_u8, BigUint::from(2_u8))),
        Operation::Push((1_u8, BigUint::from(256_u16))),
        Operation::Shl,
    ];

    run_program_assert_stack_top(program, 0_u8.into());
}

#[test]
fn shl_with_stack_underflow() {
    let program = vec![Operation::Shl];

    run_program_assert_halt(program);
}

#[test]
fn shl_out_of_gas() {
    let program = vec![
        Operation::Push((1_u8, BigUint::from(1_u8))),
        Operation::Push((1_u8, BigUint::from(4_u8))),
        Operation::Shl,
    ];
    let gas_needed = gas_cost::PUSHN * 2 + gas_cost::SHL;

    run_program_assert_gas_exact(program, gas_needed as _);
}

#[test]
fn swap_first() {
    let program = vec![
        Operation::Push((1_u8, BigUint::from(1_u8))),
        Operation::Push((1_u8, BigUint::from(2_u8))),
        Operation::Swap(1),
    ];

    run_program_assert_stack_top(program, 1_u8.into());
}

#[test]
fn swap_16_and_get_the_swapped_one() {
    let program = vec![
        Operation::Push((1_u8, BigUint::from(1_u8))),
        Operation::Push((1_u8, BigUint::from(2_u8))),
        Operation::Push((1_u8, BigUint::from(2_u8))),
        Operation::Push((1_u8, BigUint::from(2_u8))),
        Operation::Push((1_u8, BigUint::from(2_u8))),
        Operation::Push((1_u8, BigUint::from(2_u8))),
        Operation::Push((1_u8, BigUint::from(2_u8))),
        Operation::Push((1_u8, BigUint::from(2_u8))),
        Operation::Push((1_u8, BigUint::from(2_u8))),
        Operation::Push((1_u8, BigUint::from(2_u8))),
        Operation::Push((1_u8, BigUint::from(2_u8))),
        Operation::Push((1_u8, BigUint::from(2_u8))),
        Operation::Push((1_u8, BigUint::from(2_u8))),
        Operation::Push((1_u8, BigUint::from(2_u8))),
        Operation::Push((1_u8, BigUint::from(2_u8))),
        Operation::Push((1_u8, BigUint::from(2_u8))),
        Operation::Push((1_u8, BigUint::from(3_u8))),
        Operation::Swap(16),
        Operation::Pop,
        Operation::Pop,
        Operation::Pop,
        Operation::Pop,
        Operation::Pop,
        Operation::Pop,
        Operation::Pop,
        Operation::Pop,
        Operation::Pop,
        Operation::Pop,
        Operation::Pop,
        Operation::Pop,
        Operation::Pop,
        Operation::Pop,
        Operation::Pop,
        Operation::Pop,
    ];

    run_program_assert_stack_top(program, 3_u8.into());
}

#[test]
fn swap_stack_underflow() {
    let program = vec![
        Operation::Push((1_u8, BigUint::from(1_u8))),
        Operation::Push((1_u8, BigUint::from(2_u8))),
        Operation::Swap(2),
    ];

    run_program_assert_halt(program);
}

#[test]
fn swap_out_of_gas() {
    let (a, b) = (BigUint::from(1_u8), BigUint::from(2_u8));
    let program = vec![
        Operation::Push((1_u8, a.clone())),
        Operation::Push((1_u8, b.clone())),
        Operation::Swap(1),
    ];
    let gas_needed = gas_cost::PUSHN * 2 + gas_cost::SWAPN;

    run_program_assert_gas_exact(program, gas_needed as _);
}

#[test]
fn push_push_add() {
    let (a, b) = (BigUint::from(11_u8), BigUint::from(31_u8));

    let program = vec![
        Operation::Push((1_u8, a.clone())),
        Operation::Push((1_u8, b.clone())),
        Operation::Add,
    ];
    run_program_assert_stack_top(program, a + b);
}

#[test]
fn add_with_stack_underflow() {
    run_program_assert_halt(vec![Operation::Add]);
}

#[test]
fn push_push_sub() {
    let (a, b) = (BigUint::from(11_u8), BigUint::from(31_u8));

    let program = vec![
        Operation::Push((1_u8, a.clone())),
        Operation::Push((1_u8, b.clone())),
        Operation::Sub,
    ];
    run_program_assert_stack_top(program, 20_u8.into());
}

#[test]
fn substraction_wraps_the_result() {
    let (a, b) = (BigUint::from(0_u8), BigUint::from(10_u8));

    let program = vec![
        Operation::Push((1_u8, b.clone())),
        Operation::Push((1_u8, a.clone())),
        Operation::Sub,
    ];

    let result = BigInt::from(a) - BigInt::from(b);

    run_program_assert_stack_top(program, biguint_256_from_bigint(result));
}

#[test]
fn sub_add_wrapping() {
    let a = (BigUint::from(1_u8) << 256) - 1_u8;

    let program = vec![
        Operation::Push((32_u8, a)),
        Operation::Push((1_u8, BigUint::from(10_u8))),
        Operation::Add,
        Operation::Push((1_u8, BigUint::from(10_u8))),
        Operation::Sub,
    ];

    run_program_assert_stack_top(program, 1_u8.into());
}

#[test]
fn sub_out_of_gas() {
    let (a, b) = (BigUint::from(1_u8), BigUint::from(2_u8));
    let program = vec![
        Operation::Push((1_u8, a.clone())),
        Operation::Push((1_u8, b.clone())),
        Operation::Sub,
    ];
    let gas_needed = gas_cost::PUSHN * 2 + gas_cost::SUB;

    run_program_assert_gas_exact(program, gas_needed as _);
}

#[test]
fn div_without_remainder() {
    let (a, b) = (BigUint::from(20_u8), BigUint::from(5_u8));

    let expected_result = &a / &b;

    let program = vec![
        Operation::Push((1_u8, b)), // <No collapse>
        Operation::Push((1_u8, a)), // <No collapse>
        Operation::Div,
    ];

    run_program_assert_stack_top(program, expected_result);
}

#[test]
fn div_signed_division() {
    // a = [1, 0, 0, 0, .... , 0, 0, 0, 0] == 1 << 255
    let mut a = BigUint::from(0_u8);
    a.set_bit(255, true);
    // b = [0, 0, 1, 0, .... , 0, 0, 0, 0] == 1 << 253
    let mut b = BigUint::from(0_u8);
    b.set_bit(253, true);

    //r = a / b = [0, 0, 0, 0, ....., 0, 1, 0, 0] = 4 in decimal
    //If we take the lowest byte
    //r = [0, 0, 0, 0, 0, 1, 0, 0] = 4 in decimal
    let expected_result = &a / &b;

    let program = vec![
        Operation::Push((1_u8, b)), // <No collapse>
        Operation::Push((1_u8, a)), // <No collapse>
        Operation::Div,             // <No collapse>
    ];
    run_program_assert_stack_top(program, expected_result);
}

#[test]
fn div_with_remainder() {
    let (a, b) = (BigUint::from(21_u8), BigUint::from(5_u8));

    let expected_result = &a / &b;

    let program = vec![
        Operation::Push((1_u8, b)), // <No collapse>
        Operation::Push((1_u8, a)), // <No collapse>
        Operation::Div,
    ];
    run_program_assert_stack_top(program, expected_result);
}

#[test]
fn div_with_zero_denominator() {
    let (a, b) = (BigUint::from(5_u8), BigUint::from(0_u8));

    let expected_result: u8 = 0_u8;

    let program = vec![
        Operation::Push((1_u8, b)), // <No collapse>
        Operation::Push((1_u8, a)), // <No collapse>
        Operation::Div,
    ];
    run_program_assert_stack_top(program, expected_result.into());
}

#[test]
fn div_with_zero_numerator() {
    let (a, b) = (BigUint::from(0_u8), BigUint::from(10_u8));

    let expected_result = &a / &b;

    let program = vec![
        Operation::Push((1_u8, b)), // <No collapse>
        Operation::Push((1_u8, a)), // <No collapse>
        Operation::Div,
    ];
    run_program_assert_stack_top(program, expected_result);
}

#[test]
fn div_with_stack_underflow() {
    run_program_assert_halt(vec![Operation::Div]);
}

#[test]
fn div_gas_should_revert() {
    let (a, b) = (BigUint::from(21_u8), BigUint::from(5_u8));

    let program = vec![
        Operation::Push((1_u8, b)), // <No collapse>
        Operation::Push((1_u8, a)), // <No collapse>
        Operation::Div,
    ];

    let needed_gas = gas_cost::PUSHN * 2 + gas_cost::DIV;

    run_program_assert_gas_exact(program, needed_gas as _);
}

#[test]
fn sdiv_without_remainder() {
    let (a, b) = (BigUint::from(20_u8), BigUint::from(5_u8));

    let expected_result = &a / &b;

    let program = vec![
        Operation::Push((1_u8, b)), // <No collapse>
        Operation::Push((1_u8, a)), // <No collapse>
        Operation::Sdiv,
    ];

    run_program_assert_stack_top(program, expected_result);
}

#[test]
fn sdiv_signed_division_1() {
    let a = BigInt::from(-30_i8);
    let b = BigInt::from(3_i8);

    let expected_result = biguint_256_from_bigint(&a / &b);

    let a_biguint = biguint_256_from_bigint(a);
    let b_biguint = biguint_256_from_bigint(b);

    let program = vec![
        Operation::Push((1_u8, b_biguint)), // <No collapse>
        Operation::Push((1_u8, a_biguint)), // <No collapse>
        Operation::Sdiv,                    // <No collapse>
    ];
    run_program_assert_stack_top(program, expected_result);
}

#[test]
fn sdiv_signed_division_2() {
    let a = BigInt::from(-2_i8);
    let b = BigInt::from(-1_i8);

    let expected_result = biguint_256_from_bigint(&a / &b);

    let a_biguint = biguint_256_from_bigint(a);
    let b_biguint = biguint_256_from_bigint(b);

    let program = vec![
        Operation::Push((1_u8, b_biguint)), // <No collapse>
        Operation::Push((1_u8, a_biguint)), // <No collapse>
        Operation::Sdiv,                    // <No collapse>
    ];
    run_program_assert_stack_top(program, expected_result);
}

#[test]
fn sdiv_with_remainder() {
    let (a, b) = (BigUint::from(21_u8), BigUint::from(5_u8));

    let expected_result = &a / &b;

    let program = vec![
        Operation::Push((1_u8, b)), // <No collapse>
        Operation::Push((1_u8, a)), // <No collapse>
        Operation::Sdiv,
    ];
    run_program_assert_stack_top(program, expected_result);
}

#[test]
fn sdiv_with_zero_denominator() {
    let (a, b) = (BigUint::from(5_u8), BigUint::from(0_u8));

    let expected_result: u8 = 0_u8;

    let program = vec![
        Operation::Push((1_u8, b)), // <No collapse>
        Operation::Push((1_u8, a)), // <No collapse>
        Operation::Sdiv,
    ];
    run_program_assert_stack_top(program, expected_result.into());
}

#[test]
fn sdiv_with_zero_numerator() {
    let (a, b) = (BigUint::from(0_u8), BigUint::from(10_u8));

    let expected_result = &a / &b;

    let program = vec![
        Operation::Push((1_u8, b)), // <No collapse>
        Operation::Push((1_u8, a)), // <No collapse>
        Operation::Sdiv,
    ];
    run_program_assert_stack_top(program, expected_result);
}

#[test]
fn sdiv_gas_should_revert() {
    let (a, b) = (2_u8, 10_u8);

    let program = vec![
        Operation::Push((1_u8, BigUint::from(b))),
        Operation::Push((1_u8, BigUint::from(a))),
        Operation::Sdiv,
    ];
    let initial_gas = gas_cost::PUSHN * 2 + gas_cost::SDIV;
    run_program_assert_gas_exact(program, initial_gas as _);
}

#[test]
fn push_push_normal_mul() {
    let (a, b) = (BigUint::from(2_u8), BigUint::from(42_u8));

    let program = vec![
        Operation::Push((1_u8, a.clone())),
        Operation::Push((1_u8, b.clone())),
        Operation::Mul,
    ];
    run_program_assert_stack_top(program, a * b);
}

#[test]
fn mul_wraps_result() {
    let a = BigUint::from_bytes_be(&[0xFF; 32]);
    let program = vec![
        Operation::Push((32_u8, a.clone())),
        Operation::Push((1_u8, BigUint::from(2_u8))),
        Operation::Mul,
    ];
    let expected_result = (a * 2_u8).modpow(&1_u8.into(), &(BigUint::from(1_u8) << 256_u32));
    run_program_assert_stack_top(program, expected_result);
}

#[test]
fn mul_with_stack_underflow() {
    run_program_assert_halt(vec![Operation::Mul]);
}

#[test]
fn mul_gas_should_revert() {
    let (a, b) = (BigUint::from(1_u8), BigUint::from(2_u8));
    let program = vec![
        Operation::Push((1_u8, b)), // <No collapse>
        Operation::Push((1_u8, a)), // <No collapse>
        Operation::Mul,             // <No collapse>
    ];

    let needed_gas = gas_cost::PUSHN * 2 + gas_cost::MUL;
    run_program_assert_gas_exact(program, needed_gas as _);
}

#[test]
fn push_push_shr() {
    let program = vec![
        Operation::Push((1_u8, BigUint::from(32_u8))),
        Operation::Push((1_u8, BigUint::from(2_u8))),
        Operation::Shr,
    ];

    run_program_assert_stack_top(program, 8_u8.into());
}

#[test]
fn shift_bigger_than_256() {
    let program = vec![
        Operation::Push((1_u8, BigUint::from(255_u8))),
        Operation::Push((1_u8, BigUint::from(256_u16))),
        Operation::Shr,
    ];

    run_program_assert_stack_top(program, 0_u8.into());
}

#[test]
fn shr_with_stack_underflow() {
    run_program_assert_halt(vec![Operation::Shr]);
}

#[test]
fn push_push_xor() {
    let program = vec![
        Operation::Push((1_u8, BigUint::from(10_u8))),
        Operation::Push((1_u8, BigUint::from(5_u8))),
        Operation::Xor,
    ];

    run_program_assert_stack_top(program, 15_u8.into());
}

#[test]
fn xor_with_stack_underflow() {
    let program = vec![Operation::Xor];

    run_program_assert_halt(program);
}

#[test]
fn xor_out_of_gas() {
    let (a, b) = (1_u8, 2_u8);
    let program = vec![
        Operation::Push((1_u8, BigUint::from(a))),
        Operation::Push((1_u8, BigUint::from(b))),
        Operation::Xor,
    ];
    let initial_gas = gas_cost::PUSHN * 2 + gas_cost::XOR;
    run_program_assert_gas_exact(program, initial_gas as _);
}

#[test]
fn push_push_pop() {
    // Operation::Push two values to the stack and then pop once
    // The program result should be equal to the first
    // operation::pushed value
    let (a, b) = (BigUint::from(1_u8), BigUint::from(2_u8));

    let program = vec![
        Operation::Push((1_u8, a.clone())),
        Operation::Push((1_u8, b)),
        Operation::Pop,
    ];
    run_program_assert_stack_top(program, a);
}

#[test]
fn pop_with_stack_underflow() {
    // Pop with an empty stack
    let program = vec![Operation::Pop];
    run_program_assert_halt(program);
}

#[test]
fn push_push_sar() {
    let (value, shift) = (2_u8, 1_u8);
    let program = vec![
        Operation::Push((1_u8, BigUint::from(value))),
        Operation::Push((1_u8, BigUint::from(shift))),
        Operation::Sar,
    ];
    let expected_result = value >> shift;
    run_program_assert_stack_top(program, expected_result.into());
}

#[test]
fn sar_with_stack_underflow() {
    let program = vec![Operation::Sar];
    run_program_assert_halt(program);
}

#[test]
fn check_codesize() {
    let mut a: BigUint;
    let mut program = vec![Operation::Push0, Operation::Codesize];
    // This is the size of the push + mstore + return code added inside the function.
    let return_code_size = 6;
    let mut codesize = 2 + return_code_size;

    run_program_assert_stack_top(program, codesize.into());

    // iterate from 1 byte to 32 byte operation::push cases
    for i in 0..255 {
        a = BigUint::from(1_u8) << i;

        program = vec![Operation::Push((i / 8 + 1, a.clone())), Operation::Codesize];

        codesize = 1 + (i / 8 + 1) + 1 + return_code_size; // OPERATION::PUSHN + N + CODESIZE

        run_program_assert_stack_top(program, codesize.into());
    }
}

#[test]
fn push_push_byte() {
    let mut value: [u8; 32] = [0; 32];
    let desired_byte = 0xff;
    let offset: u8 = 16;
    value[offset as usize] = desired_byte;
    let value: BigUint = BigUint::from_bytes_be(&value);
    let program = vec![
        Operation::Push((32_u8, value)),
        Operation::Push((1_u8, BigUint::from(offset))),
        Operation::Byte,
    ];
    run_program_assert_stack_top(program, desired_byte.into());
}

#[test]
fn byte_with_stack_underflow() {
    let program = vec![Operation::Byte];
    run_program_assert_halt(program);
}

#[test]
fn sar_with_negative_value_preserves_sign() {
    // in this example the the value to be shifted is a 256 bit number
    // where the most significative bit is 1 cand the rest of the bits are 0.
    // i.e,  value = 1000..0000
    //
    // if we shift this value 255 positions to the right, given that
    // the sar operation preserves the sign, the result must be a number
    // in which every bit is 1
    // i.e, result = 1111..1111
    //
    // given that the program results is a u8, the result is then truncated
    // to the less 8 significative bits, i.e  result = 0b11111111.
    //
    // this same example can be visualized in the evm playground in the following link
    // https://www.evm.codes/playground?fork=cancun&unit=Wei&codeType=Mnemonic&code='%2F%2F%20Example%201z32%200x8yyyz8%20255wSAR'~0000000zwOPERATION::PUSHy~~~w%5Cn%01wyz~_

    let mut value: [u8; 32] = [0; 32];
    value[0] = 0b10000000;
    let value = BigUint::from_bytes_be(&value);

    let shift: u8 = 255;
    let program = vec![
        Operation::Push((32_u8, value)),
        Operation::Push((1_u8, BigUint::from(shift))),
        Operation::Sar,
    ];
    let expected_result = BigUint::from_bytes_be(&[255; 32]);
    run_program_assert_stack_top(program, expected_result);
}

#[test]
fn sar_with_positive_value_preserves_sign() {
    let mut value: [u8; 32] = [0xff; 32];
    value[0] = 0;
    let value = BigUint::from_bytes_be(&value);
    let shift: u8 = 30;
    let expected_result = &value >> shift;

    let program = vec![
        Operation::Push((32_u8, value.clone())),
        Operation::Push((1_u8, BigUint::from(shift))),
        Operation::Sar,
    ];
    run_program_assert_stack_top(program, expected_result);
}

#[test]
fn sar_with_shift_out_of_bounds() {
    // even if the shift is larger than 255 the SAR operation should
    // work the same.

    let value = BigUint::from_bytes_be(&[0xff; 32]);
    let shift: usize = 1024;
    let program = vec![
        Operation::Push((32_u8, value.clone())),
        Operation::Push((1_u8, BigUint::from(shift))),
        Operation::Sar,
    ];
    // in this case the expected result stays the same because of the sign extension
    run_program_assert_stack_top(program, value);
}

#[test]
fn byte_with_offset_out_of_bounds() {
    // must consider this case yet
    let value: [u8; 32] = [0xff; 32];
    let value: BigUint = BigUint::from_bytes_be(&value);
    let offset = BigUint::from(32_u8);
    let program = vec![
        Operation::Push((32_u8, value)),
        Operation::Push((1_u8, offset)),
        Operation::Byte,
    ];
    run_program_assert_stack_top(program, 0_u8.into());
}

#[test]
fn jumpdest() {
    let expected = 5_u8;
    let program = vec![
        Operation::Jumpdest { pc: 0 },
        Operation::Push((1_u8, BigUint::from(expected))),
        Operation::Jumpdest { pc: 34 },
    ];
    run_program_assert_stack_top(program, expected.into())
}

#[test]
fn jumpdest_gas_should_revert() {
    let program = vec![
        Operation::Push0,
        Operation::Jumpdest { pc: 0 },
        Operation::Jumpdest { pc: 1 },
        Operation::Jumpdest { pc: 2 },
    ];
    let needed_gas = gas_cost::PUSH0 + gas_cost::JUMPDEST * 3;
    run_program_assert_gas_exact(program, needed_gas as _);
}

#[test]
fn test_eq_true() {
    let a = BigInt::from(-3_i64);
    let b = BigInt::from(-3_i64);

    let program = vec![
        Operation::Push((32_u8, biguint_256_from_bigint(b.clone()))),
        Operation::Push((32_u8, biguint_256_from_bigint(a.clone()))),
        Operation::Eq,
    ];

    let expected_result = a == b;
    run_program_assert_stack_top(program, expected_result.into());
}

#[test]
fn test_eq_false() {
    let a = BigUint::from(2_u64 << 45);
    let b = BigUint::from(3_u64 << 45);

    let program = vec![
        Operation::Push((32_u8, b.clone())),
        Operation::Push((32_u8, a.clone())),
        Operation::Eq,
    ];

    let expected_result = a == b;
    run_program_assert_stack_top(program, expected_result.into());
}

#[test]
fn test_eq_with_stack_underflow() {
    run_program_assert_halt(vec![Operation::Eq]);
}

#[test]
fn test_or() {
    let a = BigUint::from(0b1010_u8);
    let b = BigUint::from(0b1110_u8);
    let expected = 0b1110_u8;
    let program = vec![
        Operation::Push((1_u8, a.clone())),
        Operation::Push((1_u8, b.clone())),
        Operation::Or,
    ];
    run_program_assert_stack_top(program, expected.into());
}

#[test]
fn test_or_with_stack_underflow() {
    let program = vec![Operation::Or];
    run_program_assert_halt(program);
}

#[test]
fn jumpi_with_true_condition() {
    // this test is equivalent to the following bytecode program
    //
    // [00] OPERATION::PUSH1 5
    // [02] OPERATION::PUSH1 1  // operation::push condition
    // [04] OPERATION::PUSH1 9  // operation::push pc
    // [06] JUMPI
    // [07] OPERATION::PUSH1 10
    // [09] JUMPDEST
    let (a, b) = (5_u8, 10_u8);
    let condition: BigUint = BigUint::from(1_u8);
    let pc: usize = 9;
    let program = vec![
        Operation::Push((1_u8, BigUint::from(a))),
        Operation::Push((1_u8, condition)),
        Operation::Push((1_u8, BigUint::from(pc as u8))),
        Operation::Jumpi,
        Operation::Push((1_u8, BigUint::from(b))), // this should not be executed
        Operation::Jumpdest { pc },
    ];
    run_program_assert_stack_top(program, a.into());
}

#[test]
fn test_iszero_true() {
    let program = vec![
        Operation::Push((1_u8, BigUint::from(0_u8))),
        Operation::IsZero,
    ];
    run_program_assert_stack_top(program, 1_u8.into());
}

#[test]
fn test_iszero_false() {
    let program = vec![
        Operation::Push((1_u8, BigUint::from(1_u8))),
        Operation::IsZero,
    ];
    run_program_assert_stack_top(program, 0_u8.into());
}

#[test]
fn test_iszero_stack_underflow() {
    let program = vec![Operation::IsZero];
    run_program_assert_halt(program);
}

#[test]
fn jump() {
    // this test is equivalent to the following bytecode program
    // the program executes sequentially until the JUMP where
    // it jumps to the opcode in the position 7 so the OPERATION::PUSH1 10
    // opcode is not executed => the return value should be equal
    // to the first operation::pushed value (a = 5)
    //
    // [00] OPERATION::PUSH1 5
    // [02] OPERATION::PUSH1 7  // operation::push pc
    // [04] JUMP
    // [05] OPERATION::PUSH1 10
    // [07] JUMPDEST
    let (a, b) = (5_u8, 10_u8);
    let pc: usize = 7;
    let program = vec![
        Operation::Push((1_u8, BigUint::from(a))),
        Operation::Push((1_u8, BigUint::from(pc as u8))),
        Operation::Jump,
        Operation::Push((1_u8, BigUint::from(b))), // this should not be executed
        Operation::Jumpdest { pc },
    ];
    run_program_assert_stack_top(program, a.into());
}

#[test]
fn jumpi_with_false_condition() {
    // this test is equivalent to the following bytecode program
    //
    // [00] OPERATION::PUSH1 5
    // [02] OPERATION::PUSH1 0  // operation::push condition
    // [04] OPERATION::PUSH1 9  // operation::push pc
    // [06] JUMPI
    // [07] OPERATION::PUSH1 10
    // [09] JUMPDEST
    let (a, b) = (5_u8, 10_u8);
    let condition: BigUint = BigUint::from(0_u8);
    let pc: usize = 9;
    let program = vec![
        Operation::Push((1_u8, BigUint::from(a))),
        Operation::Push((1_u8, condition)),
        Operation::Push((1_u8, BigUint::from(pc as u8))),
        Operation::Jumpi,
        Operation::Push((1_u8, BigUint::from(b))),
        Operation::Jumpdest { pc },
    ];
    run_program_assert_stack_top(program, b.into());
}

#[test]
fn jumpi_reverts_if_pc_is_wrong() {
    // if the pc given does not correspond to a jump destination then
    // the program should revert
    let pc = BigUint::from(7_u8);
    let condition = BigUint::from(1_u8);
    let program = vec![
        Operation::Push((1_u8, condition)),
        Operation::Push((1_u8, pc)),
        Operation::Jumpi,
        Operation::Jumpdest { pc: 83 },
    ];
    run_program_assert_halt(program);
}

#[test]
fn jump_reverts_if_pc_is_wrong() {
    // if the pc given does not correspond to a jump destination then
    // the program should revert
    let pc = BigUint::from(7_u8);
    let program = vec![
        Operation::Push((1_u8, pc)),
        Operation::Jump,
        Operation::Jumpdest { pc: 83 },
    ];
    run_program_assert_halt(program);
}

#[test]
fn jumpi_does_not_revert_if_pc_is_wrong_but_branch_is_not_taken() {
    // if the pc given does not correspond to a jump destination
    // but the branch is not taken then the program should not revert
    let pc = BigUint::from(7_u8);
    let condition = BigUint::from(0_u8);
    let a = 10_u8;
    let program = vec![
        Operation::Push((1_u8, condition)),
        Operation::Push((1_u8, pc)),
        Operation::Jumpi,
        Operation::Push((1_u8, BigUint::from(a))),
        Operation::Jumpdest { pc: 83 },
    ];
    run_program_assert_stack_top(program, a.into());
}

#[test]
fn pc_with_previous_push() {
    let pc = 33;
    let program = vec![
        Operation::Push((1_u8, BigUint::from(8_u8))), // <No collapse>
        Operation::PC { pc },                         // <No collapse>
    ];
    run_program_assert_stack_top(program, pc.into())
}

#[test]
fn pc_with_no_previous_operation() {
    let pc = 0;
    let program = vec![
        Operation::PC { pc }, // <No collapse>
    ];
    run_program_assert_stack_top(program, pc.into())
}

#[test]
fn pc_gas_should_revert() {
    let program = vec![Operation::Push0, Operation::PC { pc: 0 }];
    let needed_gas = gas_cost::PUSH0 + gas_cost::PC;
    run_program_assert_gas_exact(program, needed_gas as _);
}

#[test]
fn check_initial_memory_size() {
    let program = vec![Operation::Msize];

    run_program_assert_stack_top(program, BigUint::ZERO)
}

#[test]
fn check_memory_size_after_store() {
    let a = (BigUint::from(1_u8) << 256) - 1_u8;
    let b = (BigUint::from(1_u8) << 256) - 1_u8;
    let program = vec![
        Operation::Push((32, a)),
        Operation::Push0,
        Operation::Mstore,
        Operation::Push((32, b)),
        Operation::Push((1, 32_u8.into())),
        Operation::Mstore,
        Operation::Msize,
    ];

    run_program_assert_stack_top(program, 64_u8.into());
}

#[test]
fn msize_out_of_gas() {
    let program = vec![Operation::Msize];
    let gas_needed = gas_cost::MSIZE;

    run_program_assert_gas_exact(program, gas_needed as _);
}

#[test]
fn test_and() {
    let (a, b) = (BigUint::from(0b1010_u8), BigUint::from(0b1100_u8));
    let expected_result = 0b1000_u8;
    let program = vec![
        Operation::Push((1_u8, a)),
        Operation::Push((1_u8, b)),
        Operation::And,
    ];
    run_program_assert_stack_top(program, expected_result.into());
}

#[test]
fn test_and_with_zero() {
    let a = BigUint::from(0_u8);
    let b = BigUint::from(0xFF_u8);
    let expected_result = 0_u8;
    let program = vec![
        Operation::Push((1_u8, a)),
        Operation::Push((1_u8, b)),
        Operation::And,
    ];
    run_program_assert_stack_top(program, expected_result.into());
}

#[test]
fn and_with_stack_underflow() {
    run_program_assert_halt(vec![Operation::And]);
}

#[test]
fn mod_with_non_zero_result() {
    let (num, den) = (BigUint::from(31_u8), BigUint::from(10_u8));
    let expected_result = &num % &den;

    let program = vec![
        Operation::Push((1_u8, den)),
        Operation::Push((1_u8, num)),
        Operation::Mod,
    ];
    run_program_assert_stack_top(program, expected_result);
}

#[test]
fn mod_with_result_zero() {
    let (num, den) = (BigUint::from(10_u8), BigUint::from(2_u8));
    let expected_result = &num % &den;

    let program = vec![
        Operation::Push((1_u8, den)),
        Operation::Push((1_u8, num)),
        Operation::Mod,
    ];
    run_program_assert_stack_top(program, expected_result);
}

#[test]
fn mod_with_zero_denominator() {
    let (num, den) = (BigUint::from(10_u8), BigUint::from(0_u8));

    let program = vec![
        Operation::Push((1_u8, den)),
        Operation::Push((1_u8, num)),
        Operation::Mod,
    ];
    run_program_assert_stack_top(program, 0_u8.into());
}

#[test]
fn mod_with_zero_numerator() {
    let (num, den) = (BigUint::from(0_u8), BigUint::from(25_u8));

    let program = vec![
        Operation::Push((1_u8, den)),
        Operation::Push((1_u8, num)),
        Operation::Mod,
    ];
    run_program_assert_stack_top(program, 0_u8.into());
}

#[test]
fn mod_with_stack_underflow() {
    run_program_assert_halt(vec![Operation::Mod]);
}

#[test]
fn mod_reverts_when_program_runs_out_of_gas() {
    let (a, b) = (5_u8, 10_u8);
    let program: Vec<Operation> = vec![
        Operation::Push((1_u8, BigUint::from(b))),
        Operation::Push((1_u8, BigUint::from(a))),
        Operation::Mod,
    ];
    let initial_gas = gas_cost::PUSHN * 2 + gas_cost::MOD;
    run_program_assert_gas_exact(program, initial_gas as _);
}

#[test]
fn smod_with_negative_operands() {
    // -8 mod -3 = -2
    let num = biguint_256_from_bigint(BigInt::from(-8_i8));
    let den = biguint_256_from_bigint(BigInt::from(-3_i8));

    let expected_result = biguint_256_from_bigint(BigInt::from(-2_i8));

    let program = vec![
        Operation::Push((1_u8, den)),
        Operation::Push((1_u8, num)),
        Operation::SMod,
    ];
    run_program_assert_stack_top(program, expected_result);
}

#[test]
fn smod_with_negative_denominator() {
    // 8 mod -3 = 2
    let num = BigUint::from(8_u8);
    let den = biguint_256_from_bigint(BigInt::from(-3_i8));

    let expected_result = BigUint::from(2_u8);

    let program = vec![
        Operation::Push((32, den)),
        Operation::Push((32, num)),
        Operation::SMod,
    ];
    run_program_assert_stack_top(program, expected_result);
}

#[test]
fn smod_with_negative_numerator() {
    // -8 mod 3 = -2
    let num = biguint_256_from_bigint(BigInt::from(-8_i8));
    let den = BigUint::from(3_u8);

    let expected_result = biguint_256_from_bigint(BigInt::from(-2_i8));

    let program = vec![
        Operation::Push((1_u8, den)),
        Operation::Push((1_u8, num)),
        Operation::SMod,
    ];
    run_program_assert_stack_top(program, expected_result);
}

#[test]
fn smod_with_positive_operands() {
    let (num, den) = (BigUint::from(31_u8), BigUint::from(10_u8));
    let expected_result = &num % &den;

    let program = vec![
        Operation::Push((1_u8, den)),
        Operation::Push((1_u8, num)),
        Operation::SMod,
    ];
    run_program_assert_stack_top(program, expected_result);
}

#[test]
fn smod_with_zero_denominator() {
    let (num, den) = (BigUint::from(10_u8), BigUint::from(0_u8));

    let program = vec![
        Operation::Push((1_u8, den)),
        Operation::Push((1_u8, num)),
        Operation::SMod,
    ];
    run_program_assert_stack_top(program, 0_u8.into());
}

#[test]
fn smod_with_stack_underflow() {
    run_program_assert_halt(vec![Operation::SMod]);
}

#[test]
fn smod_reverts_when_program_runs_out_of_gas() {
    let (a, b) = (5_u8, 10_u8);
    let program = vec![
        Operation::Push((1_u8, BigUint::from(b))),
        Operation::Push((1_u8, BigUint::from(a))),
        Operation::SMod,
    ];
    let needed_gas = gas_cost::PUSHN * 2 + gas_cost::SMOD;
    run_program_assert_gas_exact(program, needed_gas as _);
}

#[test]
fn addmod_with_non_zero_result() {
    let (a, b, den) = (
        BigUint::from(13_u8),
        BigUint::from(30_u8),
        BigUint::from(10_u8),
    );

    let program = vec![
        Operation::Push((1_u8, den.clone())),
        Operation::Push((1_u8, b.clone())),
        Operation::Push((1_u8, a.clone())),
        Operation::Addmod,
    ];
    run_program_assert_stack_top(program, (a + b) % den);
}

#[test]
fn addmod_with_stack_underflow() {
    run_program_assert_halt(vec![Operation::Addmod]);
}

#[test]
fn addmod_with_zero_denominator() {
    let program = vec![
        Operation::Push((1_u8, BigUint::from(0_u8))),
        Operation::Push((1_u8, BigUint::from(31_u8))),
        Operation::Push((1_u8, BigUint::from(11_u8))),
        Operation::Addmod,
    ];
    run_program_assert_stack_top(program, 0_u8.into());
}

#[test]
fn addmod_with_overflowing_add() {
    let (a, b, den) = (
        BigUint::from_bytes_be(&[0xff; 32]),
        BigUint::from(1_u8),
        BigUint::from(10_u8),
    );

    let program = vec![
        Operation::Push((1_u8, den.clone())),
        Operation::Push((1_u8, b.clone())),
        Operation::Push((32_u8, a.clone())),
        Operation::Addmod,
    ];
    run_program_assert_stack_top(program, (a + b) % den);
}

#[test]
fn addmod_reverts_when_program_runs_out_of_gas() {
    let (a, b, den) = (
        BigUint::from(5_u8),
        BigUint::from(10_u8),
        BigUint::from(2_u8),
    );

    let program = vec![
        Operation::Push((1_u8, den.clone())),
        Operation::Push((1_u8, b.clone())),
        Operation::Push((1_u8, a.clone())),
        Operation::Addmod,
    ];

    let needed_gas = gas_cost::PUSHN * 3 + gas_cost::ADDMOD;

    run_program_assert_gas_exact(program, needed_gas as _);
}

#[test]
fn test_gt_less_than() {
    let a = BigUint::from(8_u8);
    let b = BigUint::from(9_u8);
    let program = vec![
        Operation::Push((1_u8, b.clone())),
        Operation::Push((1_u8, a.clone())),
        Operation::Gt,
    ];
    let expected_result = a > b;
    run_program_assert_stack_top(program, expected_result.into());
}

#[test]
fn test_gt_greater_than() {
    let a = BigUint::from(9_u64 << 20);
    let b = BigUint::from(8_u64 << 20);
    let program = vec![
        Operation::Push((32_u8, b.clone())),
        Operation::Push((32_u8, a.clone())),
        Operation::Gt,
    ];
    let expected_result = a > b;
    run_program_assert_stack_top(program, expected_result.into());
}

#[test]
fn test_gt_equal() {
    let a = BigUint::from(10_u64 << 30);
    let b = BigUint::from(10_u64 << 30);
    let program = vec![
        Operation::Push((32_u8, b.clone())),
        Operation::Push((32_u8, a.clone())),
        Operation::Gt,
    ];
    let expected_result = a > b;
    run_program_assert_stack_top(program, expected_result.into());
}

#[test]
fn gt_with_stack_underflow() {
    run_program_assert_halt(vec![Operation::Gt]);
}

#[test]
fn mulmod_with_non_zero_result() {
    let (a, b, den) = (
        BigUint::from(13_u8),
        BigUint::from(30_u8),
        BigUint::from(10_u8),
    );

    let program = vec![
        Operation::Push((1_u8, den.clone())),
        Operation::Push((1_u8, b.clone())),
        Operation::Push((1_u8, a.clone())),
        Operation::Mulmod,
    ];
    run_program_assert_stack_top(program, (a * b) % den);
}

#[test]
fn mulmod_with_stack_underflow() {
    run_program_assert_halt(vec![Operation::Mulmod]);
}

#[test]
fn mulmod_with_zero_denominator() {
    let program = vec![
        Operation::Push((1_u8, BigUint::from(0_u8))),
        Operation::Push((1_u8, BigUint::from(31_u8))),
        Operation::Push((1_u8, BigUint::from(11_u8))),
        Operation::Addmod,
    ];
    run_program_assert_stack_top(program, 0_u8.into());
}

#[test]
fn mulmod_with_overflow() {
    let (a, b, den) = (
        BigUint::from_bytes_be(&[0xff; 32]),
        BigUint::from_bytes_be(&[0xff; 32]),
        BigUint::from(10_u8),
    );

    let program = vec![
        Operation::Push((1_u8, den.clone())),
        Operation::Push((32_u8, b.clone())),
        Operation::Push((32_u8, a.clone())),
        Operation::Mulmod,
    ];
    run_program_assert_stack_top(program, (a * b) % den);
}

#[test]
fn mulmod_reverts_when_program_runs_out_of_gas() {
    let (a, b, den) = (
        BigUint::from(13_u8),
        BigUint::from(30_u8),
        BigUint::from(10_u8),
    );

    let program = vec![
        Operation::Push((1_u8, den.clone())),
        Operation::Push((1_u8, b.clone())),
        Operation::Push((1_u8, a.clone())),
        Operation::Mulmod,
    ];

    let needed_gas = gas_cost::PUSHN * 3 + gas_cost::MULMOD;

    run_program_assert_gas_exact(program, needed_gas as _);
}

#[test]
fn test_sgt_positive_greater_than() {
    let a = BigUint::from(2_u8);
    let b = BigUint::from(1_u8);

    let program = vec![
        Operation::Push((1_u8, a.clone())),
        Operation::Push((1_u8, b.clone())),
        Operation::Sgt,
    ];
    run_program_assert_stack_top(program, 0_u8.into());
}

#[test]
fn test_sgt_positive_less_than() {
    let a = BigUint::from(0_u8);
    let b = BigUint::from(2_u8);

    let program = vec![
        Operation::Push((1_u8, a.clone())),
        Operation::Push((1_u8, b.clone())),
        Operation::Sgt,
    ];
    run_program_assert_stack_top(program, 1_u8.into());
}

#[test]
fn test_sgt_signed_less_than() {
    let (a, b) = (BigInt::from(-3), BigInt::from(2));

    let expected_result = BigUint::from((a > b) as u8);

    let program = vec![
        Operation::Push((1_u8, biguint_256_from_bigint(b))),
        Operation::Push((1_u8, biguint_256_from_bigint(a))),
        Operation::Sgt,
    ];

    run_program_assert_stack_top(program, expected_result);
}

#[test]
fn test_sgt_signed_greater_than() {
    let a = BigUint::from(2_u8);
    let mut b = BigUint::from(3_u8);
    b.set_bit(255, true);

    let program = vec![
        Operation::Push((1_u8, a.clone())),
        Operation::Push((1_u8, b.clone())),
        Operation::Sgt,
    ];
    run_program_assert_stack_top(program, 0_u8.into());
}

#[test]
fn test_sgt_equal() {
    let a = BigUint::from(2_u8);
    let b = BigUint::from(2_u8);

    let program = vec![
        Operation::Push((1_u8, a.clone())),
        Operation::Push((1_u8, b.clone())),
        Operation::Sgt,
    ];
    run_program_assert_stack_top(program, 0_u8.into());
}

#[test]
fn test_sgt_stack_underflow() {
    let program = vec![Operation::Sgt];
    run_program_assert_halt(program);
}

#[test]
fn test_lt_false() {
    let program = vec![
        Operation::Push((1_u8, BigUint::from(1_u8))),
        Operation::Push((1_u8, BigUint::from(2_u8))),
        Operation::Lt,
    ];
    run_program_assert_stack_top(program, 0_u8.into());
}

#[test]
fn test_lt_true() {
    let program = vec![
        Operation::Push((1_u8, BigUint::from(2_u8))),
        Operation::Push((1_u8, BigUint::from(1_u8))),
        Operation::Lt,
    ];
    run_program_assert_stack_top(program, 1_u8.into());
}

#[test]
fn test_lt_equal() {
    let program = vec![
        Operation::Push((1_u8, BigUint::from(1_u8))),
        Operation::Push((1_u8, BigUint::from(1_u8))),
        Operation::Lt,
    ];
    run_program_assert_stack_top(program, 0_u8.into());
}

#[test]
fn test_lt_stack_underflow() {
    let program = vec![Operation::Lt];
    run_program_assert_halt(program);
}

#[test]
fn test_gas_with_add_should_revert() {
    let x = 1_u8;

    let program = vec![
        Operation::Push((1_u8, BigUint::from(x))),
        Operation::Push((1_u8, BigUint::from(x))),
        Operation::Add,
    ];
    let needed_gas = gas_cost::PUSHN * 2 + gas_cost::ADD;
    run_program_assert_gas_exact(program, needed_gas as _);
}

#[test]
fn stop() {
    // the operation::push operation should not be executed
    let program = vec![
        Operation::Stop,
        Operation::Push((1_u8, BigUint::from(10_u8))),
    ];
    // the push operation should not be executed
    run_program_assert_result(program, &[]);
}

#[test]
fn push_push_exp() {
    let (a, b) = (BigUint::from(3_u8), 3_u32);
    let program = vec![
        Operation::Push((1_u8, BigUint::from(b))),
        Operation::Push((1_u8, a.clone())),
        Operation::Exp,
    ];

    let expected_result = a.pow(b);

    run_program_assert_stack_top(program, expected_result);
}

#[test]
fn exp_with_overflow_should_wrap() {
    let a = BigUint::from(3_u8);
    let b = BigUint::from(300_u32);
    let modulus = BigUint::from(1_u32) << 256;
    let program = vec![
        Operation::Push((1, b.clone())),
        Operation::Push((1, a.clone())),
        Operation::Exp,
    ];

    let expected_result = a.modpow(&b, &modulus);
    run_program_assert_stack_top(program, expected_result);
}

#[test]
fn test_exp_dynamic_gas_with_exponent_lower_than_256() {
    let a = BigUint::from(3_u8);
    let b = BigUint::from(255_u16);
    let program = vec![
        Operation::Push((1, b.clone())),
        Operation::Push((1, a.clone())),
        Operation::Exp,
    ];
    let dynamic_gas_cost = gas_cost::PUSHN * 2 + gas_cost::exp_dynamic_cost(255);
    let result = run_program_get_result_with_gas(program, 1000);
    assert_eq!(
        result,
        ExecutionResult::Success {
            logs: vec![],
            reason: SuccessReason::Stop,
            gas_used: dynamic_gas_cost as u64,
            gas_refunded: 0,
            output: Output::Call(Bytes::new()),
        }
    );
}

#[test]
fn test_exp_dynamic_gas_with_exponent_greater_than_256() {
    let a = BigUint::from(3_u8);
    let b = BigUint::from(256_u16);
    let program = vec![
        Operation::Push((1, b.clone())),
        Operation::Push((1, a.clone())),
        Operation::Exp,
    ];
    let dynamic_gas_cost = gas_cost::PUSHN * 2 + gas_cost::exp_dynamic_cost(256);
    let result = run_program_get_result_with_gas(program, 1000);
    assert_eq!(
        result,
        ExecutionResult::Success {
            logs: vec![],
            reason: SuccessReason::Stop,
            gas_used: dynamic_gas_cost as u64,
            gas_refunded: 0,
            output: Output::Call(Bytes::new()),
        }
    );
}

#[test]
fn test_exp_dynamic_gas_with_exponent_lower_than_65536() {
    let a = BigUint::from(3_u8);
    let b = BigUint::from(65535_u16);
    let program = vec![
        Operation::Push((1, b.clone())),
        Operation::Push((1, a.clone())),
        Operation::Exp,
    ];
    let dynamic_gas_cost = gas_cost::PUSHN * 2 + gas_cost::exp_dynamic_cost(65535);
    let result = run_program_get_result_with_gas(program, 1000);
    assert_eq!(
        result,
        ExecutionResult::Success {
            logs: vec![],
            reason: SuccessReason::Stop,
            gas_used: dynamic_gas_cost as u64,
            gas_refunded: 0,
            output: Output::Call(Bytes::new()),
        }
    );
}

#[test]
fn test_exp_dynamic_gas_with_exponent_greater_than_65536() {
    let a = BigUint::from(3_u8);
    let b = BigUint::from(65536_u32);
    let program = vec![
        Operation::Push((1, b.clone())),
        Operation::Push((1, a.clone())),
        Operation::Exp,
    ];
    let dynamic_gas_cost = gas_cost::PUSHN * 2 + gas_cost::exp_dynamic_cost(65536);
    let result = run_program_get_result_with_gas(program, 1000);
    assert_eq!(
        result,
        ExecutionResult::Success {
            logs: vec![],
            reason: SuccessReason::Stop,
            gas_used: dynamic_gas_cost as u64,
            gas_refunded: 0,
            output: Output::Call(Bytes::new()),
        }
    );
}

#[test]
fn exp_with_stack_underflow() {
    let program = vec![Operation::Exp];
    run_program_assert_halt(program);
}

#[test]
fn sar_reverts_when_program_runs_out_of_gas() {
    let (value, shift) = (2_u8, 1_u8);
    let program: Vec<Operation> = vec![
        Operation::Push((32_u8, BigUint::from(value))),
        Operation::Push((1_u8, BigUint::from(shift))),
        Operation::Sar,
    ];
    let needed_gas = gas_cost::PUSHN + gas_cost::PUSHN + gas_cost::ADD;
    run_program_assert_gas_exact(program, needed_gas as _);
}

#[test]
fn pop_reverts_when_program_runs_out_of_gas() {
    let expected_result = 33_u8;
    let program = vec![
        Operation::Push((1_u8, BigUint::from(expected_result))),
        Operation::Push((1_u8, BigUint::from(expected_result + 1))),
        Operation::Pop,
    ];
    let needed_gas = gas_cost::PUSHN * 2 + gas_cost::POP;
    run_program_assert_gas_exact(program, needed_gas as _);
}

#[test]
fn signextend_one_byte_negative_value() {
    let value = BigUint::from(0xFF_u8);
    let value_bytes_size = BigUint::from(0_u8);

    let expected_result = biguint_256_from_bigint(BigInt::from(-1_i8));

    let program = vec![
        Operation::Push((1_u8, value)),            // <No collapse>
        Operation::Push((1_u8, value_bytes_size)), // <No collapse>
        Operation::SignExtend,                     // <No collapse>
    ];
    run_program_assert_stack_top(program, expected_result);
}

#[test]
fn signextend_one_byte_positive_value() {
    /*
    Since we are constrained by the output size u8, in order to check that the result
    was correctly sign extended (completed with 0s), we have to divide by 2 so we can check
    that the first byte is 0x3F = [0, 0, 1, 1, 1, 1, 1, 1]
    */
    let value = BigUint::from(0x7F_u8);
    let value_bytes_size = BigUint::from(0_u8);
    let denominator = BigUint::from(2_u8);

    let expected_result = 0x3F_u8;

    let program = vec![
        Operation::Push((1_u8, denominator)),      // <No collapse>
        Operation::Push((1_u8, value)),            // <No collapse>
        Operation::Push((1_u8, value_bytes_size)), // <No collapse>
        Operation::SignExtend,                     // <No collapse>
        Operation::Div,
    ];

    run_program_assert_stack_top(program, expected_result.into());
}

#[test]
fn signextend_with_stack_underflow() {
    let program = vec![Operation::SignExtend];
    run_program_assert_halt(program);
}

#[test]
fn jumpi_with_gas_cost() {
    // this test is equivalent to the following program
    // [00] PUSH1 0
    // [02] PUSH1 1
    // [04] PUSH1 9
    // [06] JUMPI
    // [07] PUSH1 10   // this should not be executed
    // [09] JUMPDEST
    let pc = 9;
    let condition = BigUint::from(1_u8);
    let expected_result: u8 = 0;
    let program = vec![
        Operation::Push((1_u8, BigUint::from(expected_result))),
        Operation::Push((1_u8, condition)),
        Operation::Push((1_u8, BigUint::from(pc))),
        Operation::Jumpi,
        Operation::Push((1_u8, BigUint::from(10_u8))), // this should not be executed
        Operation::Jumpdest { pc },
    ];
    let needed_gas = gas_cost::PUSHN * 3 + gas_cost::JUMPI + gas_cost::JUMPDEST;
    run_program_assert_gas_exact(program, needed_gas as _);
}

#[test]
fn signextend_gas_should_revert() {
    let value = BigUint::from(0x7F_u8);
    let value_bytes_size = BigUint::from(0_u8);
    let program = vec![
        Operation::Push((1_u8, value.clone())),
        Operation::Push((1_u8, value_bytes_size.clone())),
        Operation::SignExtend,
    ];
    let needed_gas = gas_cost::PUSHN * 2 + gas_cost::SIGNEXTEND;
    run_program_assert_gas_exact(program, needed_gas as _);
}

#[test]
fn gas_get_starting_value() {
    let initial_gas = 30;

    let expected_result = BigUint::from((initial_gas - gas_cost::GAS) as u64);

    let program = vec![
        Operation::Gas, // <No collapse>
    ];

    run_program_assert_stack_top_with_gas(program, expected_result, initial_gas as _);
}

#[test]
fn gas_value_after_operations() {
    let initial_gas = 30;

    let gas_consumption = gas_cost::PUSHN * 3 + gas_cost::ADD * 2 + gas_cost::GAS;
    let expected_result = BigUint::from((initial_gas - gas_consumption) as u64);

    let program = vec![
        Operation::Push((1_u8, BigUint::ZERO)), // <No collapse>
        Operation::Push((1_u8, BigUint::ZERO)), // <No collapse>
        Operation::Push((1_u8, BigUint::ZERO)), // <No collapse>
        Operation::Add,                         // <No collapse>
        Operation::Add,                         // <No collapse>
        Operation::Gas,                         // <No collapse>
    ];

    run_program_assert_stack_top_with_gas(program, expected_result, initial_gas as _);
}

#[test]
fn gas_without_enough_gas_revert() {
    let gas_consumption = gas_cost::PUSHN * 3 + gas_cost::ADD * 2 + gas_cost::GAS;

    let program = vec![
        Operation::Push((1_u8, BigUint::ZERO)), // <No collapse>
        Operation::Push((1_u8, BigUint::ZERO)), // <No collapse>
        Operation::Push((1_u8, BigUint::ZERO)), // <No collapse>
        Operation::Add,                         // <No collapse>
        Operation::Add,                         // <No collapse>
        Operation::Gas,                         // <No collapse>
    ];

    run_program_assert_gas_exact(program, gas_consumption as _);
}

#[test]
fn byte_gas_cost() {
    let value: [u8; 32] = [0xff; 32];
    let offset = BigUint::from(16_u8);
    let program: Vec<Operation> = vec![
        Operation::Push((32_u8, BigUint::from_bytes_be(&value))),
        Operation::Push((1_u8, offset)),
        Operation::Byte,
    ];
    let needed_gas = gas_cost::PUSHN * 2 + gas_cost::BYTE;
    run_program_assert_gas_exact(program, needed_gas as _);
}

#[test]
fn and_reverts_when_program_run_out_of_gas() {
    let (a, b) = (BigUint::from(0_u8), BigUint::from(1_u8));
    let program = vec![
        Operation::Push((1_u8, a.clone())),
        Operation::Push((1_u8, b.clone())),
        Operation::And,
    ];
    let needed_gas = gas_cost::PUSHN * 2 + gas_cost::AND;

    run_program_assert_gas_exact(program, needed_gas as _);
}

#[test]
fn exp_reverts_when_program_runs_out_of_gas() {
    let a = BigUint::from(3_u8);
    let b = BigUint::from(256_u16);
    let program = vec![
        Operation::Push((1, b.clone())),
        Operation::Push((1, a.clone())),
        Operation::Exp,
    ];

    let needed_gas = gas_cost::PUSHN * 2 + gas_cost::exp_dynamic_cost(256);
    run_program_assert_gas_exact(program, needed_gas as _);
}

#[test]
fn lt_reverts_when_program_runs_out_of_gas() {
    let (a, b) = (BigUint::from(0_u8), BigUint::from(1_u8));
    let program = vec![
        Operation::Push((1_u8, a.clone())),
        Operation::Push((1_u8, b.clone())),
        Operation::Lt,
    ];
    let needed_gas = gas_cost::PUSHN * 2 + gas_cost::LT;
    run_program_assert_gas_exact(program, needed_gas as _);
}

#[test]
fn sgt_reverts_when_program_runs_out_of_gas() {
    let (a, b) = (BigUint::from(0_u8), BigUint::from(1_u8));
    let program = vec![
        Operation::Push((1_u8, a.clone())),
        Operation::Push((1_u8, b.clone())),
        Operation::Sgt,
    ];
    let needed_gas = gas_cost::PUSHN * 2 + gas_cost::SGT;
    run_program_assert_gas_exact(program, needed_gas as _);
}

#[test]
fn gt_reverts_when_program_runs_out_of_gas() {
    let (a, b) = (BigUint::from(0_u8), BigUint::from(1_u8));
    let program = vec![
        Operation::Push((1_u8, a.clone())),
        Operation::Push((1_u8, b.clone())),
        Operation::Gt,
    ];
    let needed_gas = gas_cost::PUSHN * 2 + gas_cost::GT;
    run_program_assert_gas_exact(program, needed_gas as _);
}

#[test]
fn eq_reverts_when_program_runs_out_of_gas() {
    let (a, b) = (BigUint::from(0_u8), BigUint::from(1_u8));
    let program = vec![
        Operation::Push((1_u8, a.clone())),
        Operation::Push((1_u8, b.clone())),
        Operation::Eq,
    ];
    let needed_gas = gas_cost::PUSHN * 2 + gas_cost::EQ;
    run_program_assert_gas_exact(program, needed_gas as _);
}

#[test]
fn iszero_reverts_when_program_runs_out_of_gas() {
    let a = BigUint::from(0_u8);
    let program = vec![Operation::Push((1_u8, a.clone())), Operation::IsZero];
    let needed_gas = gas_cost::PUSHN + gas_cost::ISZERO;
    run_program_assert_gas_exact(program, needed_gas as _);
}

#[test]
fn or_reverts_when_program_runs_out_of_gas() {
    let (a, b) = (BigUint::from(0_u8), BigUint::from(1_u8));
    let program = vec![
        Operation::Push((1_u8, a.clone())),
        Operation::Push((1_u8, b.clone())),
        Operation::Or,
    ];
    let needed_gas = gas_cost::PUSHN * 2 + gas_cost::OR;
    run_program_assert_gas_exact(program, needed_gas as _);
}

#[test]
fn slt_positive_less_than() {
    let a = BigInt::from(1_u8);
    let b = BigInt::from(2_u8);

    let expected_result = (a < b) as u8;

    let program = vec![
        Operation::Push((1_u8, biguint_256_from_bigint(b))),
        Operation::Push((1_u8, biguint_256_from_bigint(a))),
        Operation::Slt,
    ];

    run_program_assert_stack_top(program, expected_result.into());
}

#[test]
fn slt_positive_greater_than() {
    let a = BigInt::from(2_u8);
    let b = BigInt::from(1_u8);

    let expected_result = (a < b) as u8;

    let program = vec![
        Operation::Push((1_u8, biguint_256_from_bigint(b))),
        Operation::Push((1_u8, biguint_256_from_bigint(a))),
        Operation::Slt,
    ];

    run_program_assert_stack_top(program, expected_result.into());
}

#[test]
fn slt_negative_less_than() {
    let a = BigInt::from(-3_i8);
    let b = BigInt::from(-1_i8);

    let expected_result = (a < b) as u8;

    let program = vec![
        Operation::Push((1_u8, biguint_256_from_bigint(b))),
        Operation::Push((1_u8, biguint_256_from_bigint(a))),
        Operation::Slt,
    ];

    run_program_assert_stack_top(program, expected_result.into());
}

#[test]
fn slt_negative_greater_than() {
    let a = BigInt::from(0_i8);
    let b = BigInt::from(-1_i8);

    let expected_result = (a < b) as u8;

    let program = vec![
        Operation::Push((1_u8, biguint_256_from_bigint(b))),
        Operation::Push((1_u8, biguint_256_from_bigint(a))),
        Operation::Slt,
    ];

    run_program_assert_stack_top(program, expected_result.into());
}

#[test]
fn slt_equal() {
    let a = BigInt::from(-4_i8);
    let b = BigInt::from(-4_i8);

    let expected_result = (a < b) as u8;

    let program = vec![
        Operation::Push((1_u8, biguint_256_from_bigint(b))),
        Operation::Push((1_u8, biguint_256_from_bigint(a))),
        Operation::Slt,
    ];

    run_program_assert_stack_top(program, expected_result.into());
}

#[test]
fn slt_gas_should_revert() {
    let a = BigInt::from(1_u8);
    let b = BigInt::from(2_u8);

    let needed_gas = gas_cost::PUSHN * 2 + gas_cost::SLT;

    let program = vec![
        Operation::Push((1_u8, biguint_256_from_bigint(b))),
        Operation::Push((1_u8, biguint_256_from_bigint(a))),
        Operation::Slt,
    ];

    run_program_assert_gas_exact(program, needed_gas as _);
}

#[test]
fn slt_stack_underflow() {
    let program = vec![Operation::Slt];
    run_program_assert_halt(program);
}

#[test]
fn jump_with_gas_cost() {
    // this test is equivalent to the following bytecode program
    //
    // [00] PUSH1 3
    // [02] JUMP
    // [03] JUMPDEST
    let jumpdest: u8 = 3;
    let program = vec![
        Operation::Push((1_u8, BigUint::from(0_u8))),
        Operation::Push((1_u8, BigUint::from(jumpdest))),
        Operation::Jump,
        Operation::Jumpdest {
            pc: jumpdest as usize,
        },
    ];
    let needed_gas = gas_cost::PUSHN * 2 + gas_cost::JUMPDEST + gas_cost::JUMP;
    run_program_assert_gas_exact(program, needed_gas as _);
}

#[test]
fn mload_with_stack_underflow() {
    let program = vec![Operation::Mload];
    run_program_assert_halt(program);
}

#[test]
fn mstore_with_stack_underflow() {
    let program = vec![Operation::Mstore];
    run_program_assert_halt(program);
}

#[test]
fn mstore8_with_stack_underflow() {
    let program = vec![Operation::Mstore8];
    run_program_assert_halt(program);
}

#[test]
fn mstore8_mload_with_zero_address() {
    let stored_value = BigUint::from(44_u8);
    let program = vec![
        Operation::Push((1_u8, stored_value.clone())), // value
        Operation::Push((1_u8, BigUint::from(31_u8))), // offset
        Operation::Mstore8,
        Operation::Push0, // offset
        Operation::Mload,
    ];
    run_program_assert_stack_top(program, stored_value);
}

#[test]
fn mstore_mload_with_zero_address() {
    let stored_value = BigUint::from(10_u8);
    let program = vec![
        Operation::Push((1_u8, stored_value.clone())), // value
        Operation::Push0,                              // offset
        Operation::Mstore,
        Operation::Push0, // offset
        Operation::Mload,
    ];
    run_program_assert_stack_top(program, stored_value);
}

#[test]
fn mstore_mload_with_memory_extension() {
    let stored_value = BigUint::from(25_u8);
    let program = vec![
        Operation::Push((1_u8, stored_value.clone())), // value
        Operation::Push((1_u8, BigUint::from(32_u8))), // offset
        Operation::Mstore,
        Operation::Push((1_u8, BigUint::from(32_u8))), // offset
        Operation::Mload,
    ];
    run_program_assert_stack_top(program, stored_value);
}

#[test]
fn mload_not_allocated_address() {
    // When offset for MLOAD is bigger than the current memory size, memory is extended with zeros
    let program = vec![
        Operation::Push((1_u8, BigUint::from(32_u8))), // offset
        Operation::Mload,
    ];
    run_program_assert_stack_top(program, 0_u8.into());
}

#[test]
fn not_happy_path() {
    let program = vec![Operation::Push0, Operation::Not];
    let expected_result = BigUint::from_bytes_be(&[0xff; 32]);
    run_program_assert_stack_top(program, expected_result);
}

#[test]
fn not_with_stack_underflow() {
    run_program_assert_halt(vec![Operation::Not]);
}

#[test]
fn not_gas_check() {
    let program = vec![Operation::Push0, Operation::Not];
    let needed_gas = gas_cost::PUSH0 + gas_cost::NOT;
    run_program_assert_gas_exact(program, needed_gas as _);
}

#[test]
fn mstore_gas_cost_with_memory_extension() {
    let program = vec![
        Operation::Push((1_u8, BigUint::from(10_u8))), // value
        Operation::Push((1_u8, BigUint::from(64_u8))), // offset
        Operation::Mstore,
    ];
    let dynamic_gas = gas_cost::memory_expansion_cost(0, 96);
    let needed_gas = gas_cost::PUSHN * 2 + gas_cost::MSTORE + dynamic_gas;
    run_program_assert_gas_exact(program, needed_gas as _);
}

#[test]
fn mstore8_gas_cost_with_memory_extension() {
    let program = vec![
        Operation::Push((1_u8, BigUint::from(10_u8))), // value
        Operation::Push((1_u8, BigUint::from(31_u8))), // offset
        Operation::Mstore8,
    ];
    let dynamic_gas = gas_cost::memory_expansion_cost(0, 32);
    let needed_gas = gas_cost::PUSHN * 2 + gas_cost::MSTORE8 + dynamic_gas;
    run_program_assert_gas_exact(program, needed_gas as _);
}

#[test]
fn mload_gas_cost_with_memory_extension() {
    let program = vec![
        Operation::Push0, // offset
        Operation::Mload,
    ];
    let dynamic_gas = gas_cost::memory_expansion_cost(0, 32);
    let needed_gas = gas_cost::PUSH0 + gas_cost::MLOAD + dynamic_gas;
    run_program_assert_gas_exact(program, needed_gas as _);
}

#[test]
fn mload_gas_cost_with_memory_extension2() {
    let program = vec![
        Operation::Push((1_u8, BigUint::from(1_u8))), // offset
        Operation::Mload,
    ];
    let dynamic_gas = gas_cost::memory_expansion_cost(0, 64);
    let needed_gas = gas_cost::PUSHN + gas_cost::MLOAD + dynamic_gas;
    run_program_assert_gas_exact(program, needed_gas as _);
}

#[test]
#[ignore]
fn mload_out_of_gas() {
    // TODO: offset gets truncated to 32 bits, so the program doesnt halt. Fix this
    let program = vec![
        Operation::Push((32_u8, BigUint::from_bytes_be(&[0xff; 32]))), // offset
        Operation::Mload,
    ];
    run_program_assert_halt(program);
}

#[test]
fn mstore_mcopy_mload_with_zero_address_and_gas() {
    let program = vec![
        Operation::Push((1_u8, BigUint::from(10_u8))),
        Operation::Push0,
        Operation::Mstore,
        Operation::Push((1_u8, BigUint::from(32_u8))),
        Operation::Push0,
        Operation::Push((1_u8, BigUint::from(32_u8))),
        Operation::Mcopy,
        Operation::Push((1_u8, BigUint::from(32_u8))),
        Operation::Mload,
    ];
    let dynamic_gas = gas_cost::memory_expansion_cost(0, 64) + gas_cost::memory_copy_cost(32);
    let gas_needed = gas_cost::PUSH0 * 2
        + gas_cost::PUSHN * 4
        + gas_cost::MCOPY
        + gas_cost::MLOAD
        + gas_cost::MSTORE
        + dynamic_gas;

    run_program_assert_gas_exact(program, gas_needed as _);
}

#[test]
fn mcopy_dynamic_gas() {
    let program = vec![
        Operation::Push((1_u8, BigUint::from(10_u8))),
        Operation::Push0,
        Operation::Mstore,
        Operation::Push((1_u8, BigUint::from(1_u8))),
        Operation::Push0,
        Operation::Push((1_u8, BigUint::from(32_u8))),
        Operation::Mcopy,
        Operation::Push((1_u8, BigUint::from(32_u8))),
        Operation::Mload,
    ];
    let dynamic_gas = gas_cost::memory_expansion_cost(0, 64) + gas_cost::memory_copy_cost(1);
    let gas_needed = gas_cost::PUSH0 * 2
        + gas_cost::PUSHN * 4
        + gas_cost::MLOAD
        + gas_cost::MSTORE
        + gas_cost::MCOPY
        + dynamic_gas;

    run_program_assert_gas_exact(program, gas_needed as _);
}

#[test]
fn mcopy_gas_zero_byte_copy() {
    let program = vec![
        Operation::Push((1_u8, BigUint::from(10_u8))),
        Operation::Push0,
        Operation::Mstore,
        Operation::Push0,
        Operation::Push0,
        Operation::Push0,
        Operation::Mcopy,
        Operation::Push0,
        Operation::Mload,
    ];
    let dynamic_gas = gas_cost::memory_expansion_cost(0, 32) + gas_cost::memory_copy_cost(0);
    let gas_needed = gas_cost::PUSH0 * 5
        + gas_cost::PUSHN
        + gas_cost::MLOAD
        + gas_cost::MSTORE
        + gas_cost::MCOPY
        + dynamic_gas;

    run_program_assert_gas_exact(program, gas_needed as _);
}

#[test]
fn mstore_mcopy_mload_with_zero_address() {
    let value = BigUint::from(10_u8);
    let value1 = BigUint::from(2_u8);
    let program = vec![
        Operation::Push((1_u8, value)),
        Operation::Push0,
        Operation::Mstore,
        Operation::Push((1_u8, value1)),
        Operation::Push((1_u8, BigUint::from(32_u8))),
        Operation::Mstore,
        Operation::Push((1_u8, BigUint::from(32_u8))),
        Operation::Push0,
        Operation::Push((1_u8, BigUint::from(32_u8))),
        Operation::Mcopy,
        Operation::Push((1_u8, BigUint::from(32_u8))),
        Operation::Mload,
    ];

    run_program_assert_stack_top(program, 10_u8.into());
}

#[test]
fn mcopy_offset_equals_dest_offset() {
    let value = BigUint::from(123_u8);
    let program = vec![
        Operation::Push((1_u8, value)),
        Operation::Push((1_u8, BigUint::from(32_u8))),
        Operation::Mstore,
        Operation::Push((1_u8, BigUint::from(32_u8))),
        Operation::Push((1_u8, BigUint::from(32_u8))),
        Operation::Push((1_u8, BigUint::from(32_u8))),
        Operation::Mcopy,
        Operation::Push((1_u8, BigUint::from(32_u8))),
        Operation::Mload,
    ];

    run_program_assert_stack_top(program, 123_u8.into());
}

#[test]
fn mstore_mcopy_mload_with_zero_address_arbitrary_size() {
    let value = BigUint::from(1_u8) << 24;
    let value1 = BigUint::from(2_u8) << 24;
    let program = vec![
        Operation::Push((1_u8, value1)),
        Operation::Push((1_u8, BigUint::from(32_u8))),
        Operation::Mstore,
        Operation::Push((1_u8, value)),
        Operation::Push0,
        Operation::Mstore,
        Operation::Push((1_u8, BigUint::from(4_u8))),
        Operation::Push((1_u8, BigUint::from(32_u8))),
        Operation::Push0,
        Operation::Mcopy,
        Operation::Push((1_u8, BigUint::from(32_u8))),
        Operation::Mload,
    ];
    let result = (16777216_u32 * 2).into();
    run_program_assert_stack_top(program, result);
}

#[test]
fn mcopy_with_stack_underflow() {
    let program = vec![Operation::Mcopy];

    run_program_assert_halt(program);
}

#[test]
fn codecopy_with_stack_underflow() {
    let program = vec![Operation::Codecopy];
    run_program_assert_halt(program);
}

#[test]
fn codecopy_with_gas_cost() {
    let size = 7_u8;
    let offset = 0_u8;
    let dest_offset = 0_u8;
    let program = vec![
        Operation::Push((1_u8, BigUint::from(size))),
        Operation::Push((1_u8, BigUint::from(offset))),
        Operation::Push((1_u8, BigUint::from(dest_offset))),
        Operation::Codecopy,
    ];

    let static_gas = gas_cost::CODECOPY + gas_cost::PUSHN * 3;
    let dynamic_gas = gas_cost::memory_copy_cost(size.into())
        + gas_cost::memory_expansion_cost(0, (dest_offset + size) as u32);
    let expected_gas = static_gas + dynamic_gas;
    run_program_assert_gas_exact(program, expected_gas as _);
}

#[rstest]
#[case(0)]
#[case(1)]
#[case(2)]
#[case(3)]
#[case(4)]
fn log_with_gas_cost(#[case] n: u8) {
    // static_gas = 375
    // dynamic_gas = 375 * topic_count + 8 * size + memory_expansion_cost
    let size = 32_u8;
    let offset = 0_u8;
    let topic = BigUint::from_bytes_be(&[0xff; 32]);
    let mut program = vec![];
    for _ in 0..n {
        program.push(Operation::Push((32_u8, topic.clone())));
    }
    program.push(Operation::Push((1_u8, BigUint::from(size))));
    program.push(Operation::Push((1_u8, BigUint::from(offset))));
    program.push(Operation::Log(n));
    let topic_count = n as i64;
    let static_gas = gas_cost::LOG + gas_cost::PUSHN * (2 + topic_count);
    let dynamic_gas = log_dynamic_gas_cost(size as u32, topic_count as u32)
        + gas_cost::memory_expansion_cost(0, 32_u32);
    let gas_needed = static_gas + dynamic_gas;
    run_program_assert_gas_exact(program, gas_needed as _);
}

#[test]
fn log_with_stack_underflow() {
    for n in 0..5 {
        let program = vec![Operation::Log(n)];
        run_program_assert_halt(program);
    }
}

#[test]
fn extcodecopy_with_stack_underflow() {
    let program = vec![Operation::ExtcodeCopy];
    run_program_assert_halt(program);
}

#[test]
fn extcodecopy_gas_check() {
    let size = 9_u8;
    let offset = 0_u8;
    let dest_offset = 0_u8;
    let address = 100_u8;
    let program = vec![
        Operation::Push((1_u8, BigUint::from(size))),
        Operation::Push((1_u8, BigUint::from(offset))),
        Operation::Push((1_u8, BigUint::from(dest_offset))),
        Operation::Push((1_u8, BigUint::from(address))),
        Operation::ExtcodeCopy,
    ];

    let static_gas = gas_cost::PUSHN * 4;
    let dynamic_gas = gas_cost::memory_copy_cost(size.into())
        + gas_cost::memory_expansion_cost(0, (dest_offset + size) as u32)
        + gas_cost::EXTCODECOPY_COLD;
    let expected_gas = static_gas + dynamic_gas;
    run_program_assert_gas_exact(program, expected_gas as _);
}

#[test]
fn invalid_gas_check() {
    let program = vec![
        Operation::Invalid,
        // none of the operations below should be executed
        Operation::Push((1_u8, 10_u8.into())),
        Operation::Push0,
        Operation::Mstore,
        Operation::Push((1, 32_u8.into())),
        Operation::Push0,
        Operation::Return,
    ];

    let gas = 999;
    let result = run_program_get_result_with_gas(program.clone(), gas as _);
    let expected_result = ExecutionResult::Halt {
        reason: HaltReason::OpcodeNotFound, //TODO: Modify in the future to proper reason
        gas_used: gas,
    };
    assert_eq!(result, expected_result);
}
