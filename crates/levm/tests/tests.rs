use levm::{
    operations::Operation,
    primitives::{Address, Bytes, U256},
    vm::{Account, VM},
};

// cargo test -p 'levm'

pub fn new_vm_with_ops(operations: &[Operation]) -> VM {
    new_vm_with_ops_addr_bal(operations, Address::zero(), U256::zero())
}

pub fn new_vm_with_ops_addr_bal(operations: &[Operation], address: Address, balance: U256) -> VM {
    let bytecode = operations
        .iter()
        .flat_map(Operation::to_bytecode)
        .collect::<Bytes>();

    VM::new(bytecode, address, balance)
}

fn callee_return_bytecode(return_value: U256) -> Bytes {
    let ops = vec![
        Operation::Push32(return_value), // value
        Operation::Push32(U256::zero()), // offset
        Operation::Mstore,
        Operation::Push32(U256::from(32)), // size
        Operation::Push32(U256::zero()),   // offset
        Operation::Return,
    ];

    ops.iter()
        .flat_map(Operation::to_bytecode)
        .collect::<Bytes>()
}

#[test]
fn add_op() {
    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::one()),
        Operation::Push32(U256::zero()),
        Operation::Add,
        Operation::Stop,
    ]);

    vm.execute();

    assert!(vm.current_call_frame_mut().stack.pop().unwrap() == U256::one());
    assert!(vm.current_call_frame_mut().pc() == 68);
}

#[test]
fn and_basic() {
    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::from(0b1010)),
        Operation::Push32(U256::from(0b1100)),
        Operation::And,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame_mut().stack.pop().unwrap();
    assert_eq!(result, U256::from(0b1000));
}

#[test]
fn and_binary_with_zero() {
    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::from(0b1010)),
        Operation::Push32(U256::zero()),
        Operation::And,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame_mut().stack.pop().unwrap();
    assert_eq!(result, U256::zero());
}

#[test]
fn and_with_hex_numbers() {
    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::from(0xFFFF)),
        Operation::Push32(U256::from(0xF0F0)),
        Operation::And,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame_mut().stack.pop().unwrap();
    assert_eq!(result, U256::from(0xF0F0));

    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::from(0xF000)),
        Operation::Push32(U256::from(0xF0F0)),
        Operation::And,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame_mut().stack.pop().unwrap();
    assert_eq!(result, U256::from(0xF000));

    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::from(0xB020)),
        Operation::Push32(U256::from(0x1F0F)),
        Operation::And,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame_mut().stack.pop().unwrap();
    assert_eq!(result, U256::from(0b1000000000000));
}

#[test]
fn or_basic() {
    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::from(0b1010)),
        Operation::Push32(U256::from(0b1100)),
        Operation::Or,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame_mut().stack.pop().unwrap();
    assert_eq!(result, U256::from(0b1110));

    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::from(0b1010)),
        Operation::Push32(U256::zero()),
        Operation::Or,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame_mut().stack.pop().unwrap();
    assert_eq!(result, U256::from(0b1010));

    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::from(u64::MAX)),
        Operation::Push32(U256::zero()),
        Operation::Or,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame_mut().stack.pop().unwrap();
    assert_eq!(result, U256::from(0xFFFFFFFFFFFFFFFF_u64));
}

#[test]
fn or_with_hex_numbers() {
    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::from(0xFFFF)),
        Operation::Push32(U256::from(0xF0F0)),
        Operation::Or,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame_mut().stack.pop().unwrap();
    assert_eq!(result, U256::from(0xFFFF));

    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::from(0xF000)),
        Operation::Push32(U256::from(0xF0F0)),
        Operation::Or,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame_mut().stack.pop().unwrap();
    assert_eq!(result, U256::from(0xF0F0));

    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::from(0xB020)),
        Operation::Push32(U256::from(0x1F0F)),
        Operation::Or,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame_mut().stack.pop().unwrap();
    assert_eq!(result, U256::from(0b1011111100101111));
}

#[test]
fn xor_basic() {
    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::from(0b1010)),
        Operation::Push32(U256::from(0b1100)),
        Operation::Xor,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame_mut().stack.pop().unwrap();
    assert_eq!(result, U256::from(0b110));

    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::from(0b1010)),
        Operation::Push32(U256::zero()),
        Operation::Xor,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame_mut().stack.pop().unwrap();
    assert_eq!(result, U256::from(0b1010));

    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::from(u64::MAX)),
        Operation::Push32(U256::zero()),
        Operation::Xor,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame_mut().stack.pop().unwrap();
    assert_eq!(result, U256::from(u64::MAX));

    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::from(u64::MAX)),
        Operation::Push32(U256::from(u64::MAX)),
        Operation::Xor,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame_mut().stack.pop().unwrap();
    assert_eq!(result, U256::zero());
}

#[test]
fn xor_with_hex_numbers() {
    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::from(0xF0)),
        Operation::Push32(U256::from(0xF)),
        Operation::Xor,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame_mut().stack.pop().unwrap();
    assert_eq!(result, U256::from(0xFF));

    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::from(0xFF)),
        Operation::Push32(U256::from(0xFF)),
        Operation::Xor,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame_mut().stack.pop().unwrap();
    assert_eq!(result, U256::zero());

    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::from(0xFFFF)),
        Operation::Push32(U256::from(0xF0F0)),
        Operation::Xor,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame_mut().stack.pop().unwrap();
    assert_eq!(result, U256::from(0xF0F));

    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::from(0xF000)),
        Operation::Push32(U256::from(0xF0F0)),
        Operation::Xor,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame_mut().stack.pop().unwrap();
    assert_eq!(result, U256::from(0xF0));

    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::from(0x4C0F)),
        Operation::Push32(U256::from(0x3A4B)),
        Operation::Xor,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame_mut().stack.pop().unwrap();
    assert_eq!(result, U256::from(0b111011001000100));
}

#[test]
fn not() {
    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::from(0b1010)),
        Operation::Not,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame_mut().stack.pop().unwrap();
    let expected = !U256::from(0b1010);
    assert_eq!(result, expected);

    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::MAX),
        Operation::Not,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame_mut().stack.pop().unwrap();
    assert_eq!(result, U256::zero());

    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::zero()),
        Operation::Not,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame_mut().stack.pop().unwrap();
    assert_eq!(result, U256::MAX);

    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::from(1)),
        Operation::Not,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame_mut().stack.pop().unwrap();
    assert_eq!(result, U256::MAX - 1);
}

#[test]
fn byte_basic() {
    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::from(0xF0F1)),
        Operation::Push32(U256::from(31)),
        Operation::Byte,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame_mut().stack.pop().unwrap();
    assert_eq!(result, U256::from(0xF1));

    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::from(0x33ED)),
        Operation::Push32(U256::from(30)),
        Operation::Byte,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame_mut().stack.pop().unwrap();
    assert_eq!(result, U256::from(0x33));
}

#[test]
fn byte_edge_cases() {
    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::MAX),
        Operation::Push32(U256::from(0)),
        Operation::Byte,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame_mut().stack.pop().unwrap();
    assert_eq!(result, U256::from(0xFF));

    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::MAX),
        Operation::Push32(U256::from(12)),
        Operation::Byte,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame_mut().stack.pop().unwrap();
    assert_eq!(result, U256::from(0xFF));

    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::from(0x00E0D0000)),
        Operation::Push32(U256::from(29)),
        Operation::Byte,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame_mut().stack.pop().unwrap();
    assert_eq!(result, U256::from(0x0D));

    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::from(0xFDEA179)),
        Operation::Push32(U256::from(50)),
        Operation::Byte,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame_mut().stack.pop().unwrap();
    assert_eq!(result, U256::zero());

    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::from(0xFDEA179)),
        Operation::Push32(U256::from(32)),
        Operation::Byte,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame_mut().stack.pop().unwrap();
    assert_eq!(result, U256::zero());

    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::zero()),
        Operation::Push32(U256::from(15)),
        Operation::Byte,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame_mut().stack.pop().unwrap();
    assert_eq!(result, U256::zero());

    let word = U256::from_big_endian(&[
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x57, 0x08, 0x09, 0x90, 0x0B, 0x0C, 0x0D, 0x0E,
        0x0F, 0x10, 0x11, 0x12, 0xDD, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1A, 0x1B, 0x1C, 0x1D,
        0x1E, 0x40,
    ]);

    let mut vm = new_vm_with_ops(&[
        Operation::Push32(word),
        Operation::Push32(U256::from(10)),
        Operation::Byte,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame_mut().stack.pop().unwrap();
    assert_eq!(result, U256::from(0x90));

    let mut vm = new_vm_with_ops(&[
        Operation::Push32(word),
        Operation::Push32(U256::from(7)),
        Operation::Byte,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame_mut().stack.pop().unwrap();
    assert_eq!(result, U256::from(0x57));

    let mut vm = new_vm_with_ops(&[
        Operation::Push32(word),
        Operation::Push32(U256::from(19)),
        Operation::Byte,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame_mut().stack.pop().unwrap();
    assert_eq!(result, U256::from(0xDD));

    let mut vm = new_vm_with_ops(&[
        Operation::Push32(word),
        Operation::Push32(U256::from(31)),
        Operation::Byte,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame_mut().stack.pop().unwrap();
    assert_eq!(result, U256::from(0x40));
}

#[test]
fn shl_basic() {
    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::from(0xDDDD)),
        Operation::Push32(U256::from(0)),
        Operation::Shl,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame_mut().stack.pop().unwrap();
    assert_eq!(result, U256::from(0xDDDD));

    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::from(0x12345678)),
        Operation::Push32(U256::from(1)),
        Operation::Shl,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame_mut().stack.pop().unwrap();
    assert_eq!(result, U256::from(0x2468acf0));

    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::from(0x12345678)),
        Operation::Push32(U256::from(4)),
        Operation::Shl,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame_mut().stack.pop().unwrap();
    assert_eq!(result, U256::from(4886718336_u64));

    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::from(0xFF)),
        Operation::Push32(U256::from(4)),
        Operation::Shl,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame_mut().stack.pop().unwrap();
    assert_eq!(result, U256::from(0xFF << 4));
}

#[test]
fn shl_edge_cases() {
    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::from(0x1)),
        Operation::Push32(U256::from(256)),
        Operation::Shl,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame_mut().stack.pop().unwrap();
    assert_eq!(result, U256::zero());

    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::zero()),
        Operation::Push32(U256::from(200)),
        Operation::Shl,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame_mut().stack.pop().unwrap();
    assert_eq!(result, U256::zero());

    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::MAX),
        Operation::Push32(U256::from(1)),
        Operation::Shl,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame_mut().stack.pop().unwrap();
    assert_eq!(result, U256::MAX - 1);
}

#[test]
fn shr_basic() {
    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::from(0xDDDD)),
        Operation::Push32(U256::from(0)),
        Operation::Shr,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame_mut().stack.pop().unwrap();
    assert_eq!(result, U256::from(0xDDDD));

    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::from(0x12345678)),
        Operation::Push32(U256::from(1)),
        Operation::Shr,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame_mut().stack.pop().unwrap();
    assert_eq!(result, U256::from(0x91a2b3c));

    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::from(0x12345678)),
        Operation::Push32(U256::from(4)),
        Operation::Shr,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame_mut().stack.pop().unwrap();
    assert_eq!(result, U256::from(0x1234567));

    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::from(0xFF)),
        Operation::Push32(U256::from(4)),
        Operation::Shr,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame_mut().stack.pop().unwrap();
    assert_eq!(result, U256::from(0xF));
}

#[test]
fn shr_edge_cases() {
    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::from(0x1)),
        Operation::Push32(U256::from(256)),
        Operation::Shr,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame_mut().stack.pop().unwrap();
    assert_eq!(result, U256::zero());

    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::zero()),
        Operation::Push32(U256::from(200)),
        Operation::Shr,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame_mut().stack.pop().unwrap();
    assert_eq!(result, U256::zero());

    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::MAX),
        Operation::Push32(U256::from(1)),
        Operation::Shr,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame_mut().stack.pop().unwrap();
    assert_eq!(result, U256::MAX >> 1);
}

#[test]
fn sar_shift_by_0() {
    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::from(0x12345678)),
        Operation::Push32(U256::from(0)),
        Operation::Sar,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame_mut().stack.pop().unwrap();
    assert_eq!(result, U256::from(0x12345678));
}

#[test]
fn sar_shifting_large_value_with_all_bits_set() {
    let word = U256::from_big_endian(&[
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xff,
    ]);

    let mut vm = new_vm_with_ops(&[
        Operation::Push32(word),
        Operation::Push32(U256::from(8)),
        Operation::Sar,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame_mut().stack.pop().unwrap();
    let expected = U256::from_big_endian(&[
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xff,
    ]);
    assert_eq!(result, expected);
}

#[test]
fn sar_shifting_negative_value_and_small_shift() {
    let word_neg = U256::from_big_endian(&[
        0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00,
    ]);

    let mut vm = new_vm_with_ops(&[
        Operation::Push32(word_neg),
        Operation::Push32(U256::from(4)),
        Operation::Sar,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame_mut().stack.pop().unwrap();
    let expected = U256::from_big_endian(&[
        0xf8, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00,
    ]);
    assert_eq!(result, expected);
}

#[test]
fn sar_shift_positive_value() {
    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::from(0x7FFFFF)),
        Operation::Push32(U256::from(4)),
        Operation::Sar,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame_mut().stack.pop().unwrap();
    assert_eq!(result, U256::from(0x07FFFF));
}

#[test]
fn sar_shift_negative_value() {
    let word_neg = U256::from_big_endian(&[
        0x8f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xff,
    ]);

    let mut vm = new_vm_with_ops(&[
        Operation::Push32(word_neg),
        Operation::Push32(U256::from(4)),
        Operation::Sar,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame_mut().stack.pop().unwrap();
    let expected = U256::from_big_endian(&[
        0xf8, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xff,
    ]);
    // change 0x8f to 0xf8
    assert_eq!(result, expected);
}

#[test]
fn keccak256_zero_offset_size_four() {
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

    let mut vm = new_vm_with_ops(&operations);

    vm.execute();

    assert!(
        vm.current_call_frame_mut().stack.pop().unwrap()
            == U256::from("0x29045a592007d0c246ef02c2223570da9522d0cf0f73282c79a1bc8f0bb2c238")
    );
    assert!(vm.current_call_frame_mut().pc() == 40);
}

#[test]
fn keccak256_zero_offset_size_bigger_than_actual_memory() {
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

    let mut vm = new_vm_with_ops(&operations);

    vm.execute();

    assert!(
        vm.current_call_frame_mut().stack.pop().unwrap()
            == U256::from("0xae75624a7d0413029c1e0facdd38cc8e177d9225892e2490a69c2f1f89512061")
    );
    assert!(vm.current_call_frame_mut().pc() == 40);
}

#[test]
fn keccak256_zero_offset_zero_size() {
    let operations = [
        Operation::Push0, // size
        Operation::Push0, // offset
        Operation::Keccak256,
        Operation::Stop,
    ];

    let mut vm = new_vm_with_ops(&operations);

    vm.execute();

    assert!(
        vm.current_call_frame_mut().stack.pop().unwrap()
            == U256::from("0xc5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470")
    );
    assert!(vm.current_call_frame_mut().pc() == 4);
}

#[test]
fn keccak256_offset_four_size_four() {
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

    let mut vm = new_vm_with_ops(&operations);

    vm.execute();

    assert!(
        vm.current_call_frame_mut().stack.pop().unwrap()
            == U256::from("0xe8e77626586f73b955364c7b4bbf0bb7f7685ebd40e852b164633a4acbd3244c")
    );
    assert!(vm.current_call_frame_mut().pc() == 41);
}

#[test]
fn mstore() {
    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::from(0x33333)),
        Operation::Push32(U256::zero()),
        Operation::Mstore,
        Operation::Msize,
        Operation::Stop,
    ]);

    vm.execute();

    assert_eq!(
        vm.current_call_frame_mut().stack.pop().unwrap(),
        U256::from(32)
    );
    assert_eq!(vm.current_call_frame_mut().pc(), 69);
}

#[test]
fn mstore_saves_correct_value() {
    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::from(0x33333)), // value
        Operation::Push32(U256::zero()),        // offset
        Operation::Mstore,
        Operation::Msize,
        Operation::Stop,
    ]);

    vm.execute();

    let stored_value = vm.current_call_frame_mut().memory.load(0);

    assert_eq!(stored_value, U256::from(0x33333));

    let memory_size = vm.current_call_frame_mut().stack.pop().unwrap();
    assert_eq!(memory_size, U256::from(32));
}

#[test]
fn mstore8() {
    let operations = [
        Operation::Push32(U256::from(0xAB)), // value
        Operation::Push32(U256::zero()),     // offset
        Operation::Mstore8,
        Operation::Stop,
    ];

    let bytecode = operations
        .iter()
        .flat_map(Operation::to_bytecode)
        .collect::<Bytes>();

    let mut vm = VM::new(bytecode, Address::zero(), U256::zero());

    vm.execute();

    let stored_value = vm.current_call_frame_mut().memory.load(0);

    let mut value_bytes = [0u8; 32];
    stored_value.to_big_endian(&mut value_bytes);

    assert_eq!(value_bytes[0..1], [0xAB]);
}

#[test]
fn mcopy() {
    let operations = [
        Operation::Push32(U256::from(32)),      // size
        Operation::Push32(U256::from(0)),       // source offset
        Operation::Push32(U256::from(64)),      // destination offset
        Operation::Push32(U256::from(0x33333)), // value
        Operation::Push32(U256::from(0)),       // offset
        Operation::Mstore,
        Operation::Mcopy,
        Operation::Msize,
        Operation::Stop,
    ];

    let bytecode = operations
        .iter()
        .flat_map(Operation::to_bytecode)
        .collect::<Bytes>();

    let mut vm = VM::new(bytecode, Address::zero(), U256::zero());

    vm.execute();

    let copied_value = vm.current_call_frame_mut().memory.load(64);
    assert_eq!(copied_value, U256::from(0x33333));

    let memory_size = vm.current_call_frame_mut().stack.pop().unwrap();
    assert_eq!(memory_size, U256::from(96));
}

#[test]
fn mload() {
    let operations = [
        Operation::Push32(U256::from(0x33333)), // value
        Operation::Push32(U256::zero()),        // offset
        Operation::Mstore,
        Operation::Push32(U256::zero()), // offset
        Operation::Mload,
        Operation::Stop,
    ];

    let bytecode = operations
        .iter()
        .flat_map(Operation::to_bytecode)
        .collect::<Bytes>();

    let mut vm = VM::new(bytecode, Address::zero(), U256::zero());

    vm.execute();

    let loaded_value = vm.current_call_frame_mut().stack.pop().unwrap();
    assert_eq!(loaded_value, U256::from(0x33333));
}

#[test]
fn msize() {
    let operations = [Operation::Msize, Operation::Stop];

    let bytecode = operations
        .iter()
        .flat_map(Operation::to_bytecode)
        .collect::<Bytes>();

    let mut vm = VM::new(bytecode, Address::zero(), U256::zero());

    vm.execute();

    let initial_size = vm.current_call_frame_mut().stack.pop().unwrap();
    assert_eq!(initial_size, U256::from(0));

    let operations = [
        Operation::Push32(U256::from(0x33333)), // value
        Operation::Push32(U256::zero()),        // offset
        Operation::Mstore,
        Operation::Msize,
        Operation::Stop,
    ];

    let bytecode = operations
        .iter()
        .flat_map(Operation::to_bytecode)
        .collect::<Bytes>();

    vm = VM::new(bytecode, Address::zero(), U256::zero());

    vm.execute();

    let after_store_size = vm.current_call_frame_mut().stack.pop().unwrap();
    assert_eq!(after_store_size, U256::from(32));

    let operations = [
        Operation::Push32(U256::from(0x55555)), // value
        Operation::Push32(U256::from(64)),      // offset
        Operation::Mstore,
        Operation::Msize,
        Operation::Stop,
    ];

    let bytecode = operations
        .iter()
        .flat_map(Operation::to_bytecode)
        .collect::<Bytes>();

    vm = VM::new(bytecode, Address::zero(), U256::zero());

    vm.execute();

    let final_size = vm.current_call_frame_mut().stack.pop().unwrap();
    assert_eq!(final_size, U256::from(96));
}

#[test]
fn mstore_mload_offset_not_multiple_of_32() {
    let operations = [
        Operation::Push32(0xabcdef.into()), // value
        Operation::Push32(10.into()),       // offset
        Operation::Mstore,
        Operation::Push32(10.into()), // offset
        Operation::Mload,
        Operation::Msize,
        Operation::Stop,
    ];

    let bytecode = operations
        .iter()
        .flat_map(Operation::to_bytecode)
        .collect::<Bytes>();

    let mut vm = VM::new(bytecode, Address::zero(), U256::zero());

    vm.execute();

    let memory_size = vm.current_call_frame_mut().stack.pop().unwrap();
    let loaded_value = vm.current_call_frame_mut().stack.pop().unwrap();

    assert_eq!(loaded_value, U256::from(0xabcdef));
    assert_eq!(memory_size, U256::from(64));

    //check with big offset

    let operations = [
        Operation::Push32(0x123456.into()), // value
        Operation::Push32(2000.into()),     // offset
        Operation::Mstore,
        Operation::Push32(2000.into()), // offset
        Operation::Mload,
        Operation::Msize,
        Operation::Stop,
    ];

    let bytecode = operations
        .iter()
        .flat_map(Operation::to_bytecode)
        .collect::<Bytes>();

    vm = VM::new(bytecode, Address::zero(), U256::zero());

    vm.execute();

    let memory_size = vm.current_call_frame_mut().stack.pop().unwrap();
    let loaded_value = vm.current_call_frame_mut().stack.pop().unwrap();

    assert_eq!(loaded_value, U256::from(0x123456));
    assert_eq!(memory_size, U256::from(2048));
}

#[test]
fn mload_uninitialized_memory() {
    let operations = [
        Operation::Push32(50.into()), // offset
        Operation::Mload,
        Operation::Msize,
        Operation::Stop,
    ];

    let bytecode = operations
        .iter()
        .flat_map(Operation::to_bytecode)
        .collect::<Bytes>();

    let mut vm = VM::new(bytecode, Address::zero(), U256::zero());

    vm.execute();

    let memory_size = vm.current_call_frame_mut().stack.pop().unwrap();
    let loaded_value = vm.current_call_frame_mut().stack.pop().unwrap();

    assert_eq!(loaded_value, U256::zero());
    assert_eq!(memory_size, U256::from(96));
}

#[test]
fn call_returns_if_bytecode_empty() {
    let callee_bytecode = vec![].into();

    let callee_address = Address::from_low_u64_be(U256::from(2).low_u64());
    let callee_address_u256 = U256::from(2);
    let callee_account = Account::new(U256::from(500000), callee_bytecode);

    let caller_ops = vec![
        Operation::Push32(U256::from(100_000)), // gas
        Operation::Push32(callee_address_u256), // address
        Operation::Push32(U256::zero()),        // value
        Operation::Push32(U256::from(0)),       // args_offset
        Operation::Push32(U256::from(0)),       // args_size
        Operation::Push32(U256::from(0)),       // ret_offset
        Operation::Push32(U256::from(32)),      // ret_size
        Operation::Call,
        Operation::Stop,
    ];

    let mut vm = new_vm_with_ops_addr_bal(
        &caller_ops,
        Address::from_low_u64_be(U256::from(1).low_u64()),
        U256::zero(),
    );

    vm.add_account(callee_address, callee_account);
    println!("to excec");
    vm.execute();

    let success = vm.current_call_frame_mut().stack.pop().unwrap();
    assert_eq!(success, U256::one());
}

#[test]
fn call_changes_callframe_and_stores() {
    let callee_return_value = U256::from(0xAAAAAAA);
    let callee_bytecode = callee_return_bytecode(callee_return_value);
    let callee_address = Address::from_low_u64_be(U256::from(2).low_u64());
    let callee_address_u256 = U256::from(2);
    let callee_account = Account::new(U256::from(500000), callee_bytecode);

    let caller_ops = vec![
        Operation::Push32(U256::from(32)),      // ret_size
        Operation::Push32(U256::from(0)),       // ret_offset
        Operation::Push32(U256::from(0)),       // args_size
        Operation::Push32(U256::from(0)),       // args_offset
        Operation::Push32(U256::zero()),        // value
        Operation::Push32(callee_address_u256), // address
        Operation::Push32(U256::from(100_000)), // gas
        Operation::Call,
        Operation::Stop,
    ];

    let mut vm = new_vm_with_ops_addr_bal(
        &caller_ops,
        Address::from_low_u64_be(U256::from(1).low_u64()),
        U256::zero(),
    );

    vm.add_account(callee_address, callee_account);

    vm.execute();

    let current_call_frame = vm.current_call_frame_mut();

    let success = current_call_frame.stack.pop().unwrap() == U256::one();
    assert!(success);

    let ret_offset = 0;
    let ret_size = 32;
    let return_data = current_call_frame
        .returndata
        .load_range(ret_offset, ret_size);

    assert_eq!(U256::from_big_endian(&return_data), U256::from(0xAAAAAAA));
}

#[test]
fn nested_calls() {
    let callee3_return_value = U256::from(0xAAAAAAA);
    let callee3_bytecode = callee_return_bytecode(callee3_return_value);
    let callee3_address = Address::from_low_u64_be(U256::from(3).low_u64());
    let callee3_address_u256 = U256::from(3);
    let callee3_account = Account::new(U256::from(300_000), callee3_bytecode);

    let mut callee2_ops = vec![
        Operation::Push32(U256::from(32)),       // ret_size
        Operation::Push32(U256::from(0)),        // ret_offset
        Operation::Push32(U256::from(0)),        // args_size
        Operation::Push32(U256::from(0)),        // args_offset
        Operation::Push32(U256::zero()),         // value
        Operation::Push32(callee3_address_u256), // address
        Operation::Push32(U256::from(100_000)),  // gas
        Operation::Call,
    ];

    let callee2_return_value = U256::from(0xBBBBBBB);

    let callee2_return_bytecode = vec![
        Operation::Push32(callee2_return_value), // value
        Operation::Push32(U256::from(32)),       // offset
        Operation::Mstore,
        Operation::Push32(U256::from(32)), // size
        Operation::Push32(U256::zero()),   // returndata_offset
        Operation::Push32(U256::zero()),   // dest_offset
        Operation::ReturnDataCopy,
        Operation::Push32(U256::from(64)), // size
        Operation::Push32(U256::zero()),   // offset
        Operation::Return,
    ];

    callee2_ops.extend(callee2_return_bytecode);

    let callee2_bytecode = callee2_ops
        .iter()
        .flat_map(|op| op.to_bytecode())
        .collect::<Bytes>();

    let callee2_address = Address::from_low_u64_be(U256::from(2).low_u64());
    let callee2_address_u256 = U256::from(2);

    let callee2_account = Account::new(U256::from(300_000), callee2_bytecode);

    let caller_ops = vec![
        Operation::Push32(U256::from(64)),       // ret_size
        Operation::Push32(U256::from(0)),        // ret_offset
        Operation::Push32(U256::from(0)),        // args_size
        Operation::Push32(U256::from(0)),        // args_offset
        Operation::Push32(U256::zero()),         // value
        Operation::Push32(callee2_address_u256), // address
        Operation::Push32(U256::from(100_000)),  // gas
        Operation::Call,
        Operation::Stop,
    ];

    let caller_address = Address::from_low_u64_be(U256::from(1).low_u64());
    let caller_balance = U256::from(1_000_000);

    let mut vm = new_vm_with_ops_addr_bal(&caller_ops, caller_address, caller_balance);

    vm.add_account(callee2_address, callee2_account);
    vm.add_account(callee3_address, callee3_account);

    vm.execute();

    let current_call_frame = vm.current_call_frame_mut();

    let success = current_call_frame.stack.pop().unwrap();
    assert_eq!(success, U256::one());

    let ret_offset = 0;
    let ret_size = 64;
    let return_data = current_call_frame
        .returndata
        .load_range(ret_offset, ret_size);

    let mut expected_bytes = vec![0u8; 64];
    // place 0xAAAAAAA at 0..32
    let mut callee3_return_value_bytes = [0u8; 32];
    callee3_return_value.to_big_endian(&mut callee3_return_value_bytes);
    expected_bytes[..32].copy_from_slice(&callee3_return_value_bytes);

    // place 0xBBBBBBB at 32..64
    let mut callee2_return_value_bytes = [0u8; 32];
    callee2_return_value.to_big_endian(&mut callee2_return_value_bytes);
    expected_bytes[32..].copy_from_slice(&callee2_return_value_bytes);

    assert_eq!(return_data, expected_bytes);
}

#[test]
fn pop_op() {
    let operations = [
        Operation::Push32(U256::one()),
        Operation::Push32(U256::from(100)),
        Operation::Pop,
        Operation::Stop,
    ];

    let mut vm = new_vm_with_ops(&operations);

    vm.execute();

    assert!(vm.current_call_frame_mut().stack.pop().unwrap() == U256::one());
}

// TODO: when adding error handling this should return an error, not panic
#[test]
#[should_panic]
fn pop_on_empty_stack() {
    let operations = [Operation::Pop, Operation::Stop];

    let mut vm = new_vm_with_ops(&operations);

    vm.execute();

    assert!(vm.current_call_frame_mut().stack.pop().unwrap() == U256::one());
}

#[test]
fn pc_op() {
    let operations = [Operation::PC, Operation::Stop];
    let mut vm = new_vm_with_ops(&operations);

    vm.execute();

    assert!(vm.current_call_frame_mut().stack.pop().unwrap() == U256::from(0));
}

#[test]
fn pc_op_with_push_offset() {
    let operations = [
        Operation::Push32(U256::one()),
        Operation::PC,
        Operation::Stop,
    ];

    let mut vm = new_vm_with_ops(&operations);

    vm.execute();

    assert!(vm.current_call_frame_mut().stack.pop().unwrap() == U256::from(33));
}

#[test]
fn jump_op() {
    let operations = [
        Operation::Push32(U256::from(35)),
        Operation::Jump,
        Operation::Stop, // should skip this one
        Operation::Jumpdest,
        Operation::Push32(U256::from(10)),
        Operation::Stop,
    ];

    let mut vm = new_vm_with_ops(&operations);

    vm.execute();

    assert!(vm.current_call_frame_mut().stack.pop().unwrap() == U256::from(10));
    assert_eq!(vm.current_call_frame_mut().pc(), 70);
}

#[test]
#[should_panic]
fn jump_not_jumpdest_position() {
    let operations = [
        Operation::Push32(U256::from(36)),
        Operation::Jump,
        Operation::Stop,
        Operation::Push32(U256::from(10)),
        Operation::Stop,
    ];

    let mut vm = new_vm_with_ops(&operations);

    vm.execute();
    assert_eq!(vm.current_call_frame_mut().pc, 35);
}

#[test]
#[should_panic]
fn jump_position_bigger_than_program_bytecode_size() {
    let operations = [
        Operation::Push32(U256::from(5000)),
        Operation::Jump,
        Operation::Stop,
        Operation::Push32(U256::from(10)),
        Operation::Stop,
    ];

    let mut vm = new_vm_with_ops(&operations);

    vm.execute();
    assert_eq!(vm.current_call_frame_mut().pc(), 35);
}

#[test]
fn jumpi_not_zero() {
    let operations = [
        Operation::Push32(U256::one()),
        Operation::Push32(U256::from(68)),
        Operation::Jumpi,
        Operation::Stop, // should skip this one
        Operation::Jumpdest,
        Operation::Push32(U256::from(10)),
        Operation::Stop,
    ];
    let mut vm = new_vm_with_ops(&operations);

    vm.execute();

    assert!(vm.current_call_frame_mut().stack.pop().unwrap() == U256::from(10));
}

#[test]
fn jumpi_for_zero() {
    let operations = [
        Operation::Push32(U256::from(100)),
        Operation::Push32(U256::zero()),
        Operation::Push32(U256::from(100)),
        Operation::Jumpi,
        Operation::Stop,
        Operation::Jumpdest,
        Operation::Push32(U256::from(10)),
        Operation::Stop,
    ];

    let mut vm = new_vm_with_ops(&operations);

    vm.execute();

    assert!(vm.current_call_frame_mut_mut().stack.pop().unwrap() == U256::from(100));
}

#[test]
fn calldataload() {
    let calldata = Memory::new_from_vec(vec![
        0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF,
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E,
        0x0F, 0x10,
    ]);
    let ops = vec![
        Operation::Push32(U256::from(0)), // offset
        Operation::CallDataLoad,
        Operation::Stop,
    ];
    let mut vm = new_vm_with_ops(&ops);

    vm.current_call_frame_mut().calldata = calldata;
    vm.execute();

    let current_call_frame = vm.current_call_frame_mut();

    let top_of_stack = current_call_frame.stack.pop().unwrap();
    assert_eq!(
        top_of_stack,
        U256::from_big_endian(&[
            0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE,
            0xFF, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C,
            0x0D, 0x0E, 0x0F, 0x10
        ])
    );
}

#[test]
fn calldataload_being_set_by_parent() {
    let ops = vec![
        Operation::Push32(U256::zero()), // offset
        Operation::CallDataLoad,
        Operation::Push32(U256::from(0)), // offset
        Operation::Mstore,
        Operation::Push32(U256::from(32)), // size
        Operation::Push32(U256::zero()),   // offset
        Operation::Return,
    ];

    let callee_bytecode = ops
        .iter()
        .flat_map(Operation::to_bytecode)
        .collect::<Bytes>();

    let callee_address = Address::from_low_u64_be(U256::from(2).low_u64());
    let callee_address_u256 = U256::from(2);
    let callee_account = Account::new(U256::from(500000), callee_bytecode);

    let calldata = vec![
        0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF,
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E,
        0x0F, 0x10,
    ];

    let caller_ops = vec![
        Operation::Push32(U256::from_big_endian(&calldata[..32])), // value
        Operation::Push32(U256::from(0)),                          // offset
        Operation::Mstore,
        Operation::Push32(U256::from(32)),      // ret_size
        Operation::Push32(U256::from(0)),       // ret_offset
        Operation::Push32(U256::from(32)),      // args_size
        Operation::Push32(U256::from(0)),       // args_offset
        Operation::Push32(U256::zero()),        // value
        Operation::Push32(callee_address_u256), // address
        Operation::Push32(U256::from(100_000)), // gas
        Operation::Call,
        Operation::Stop,
    ];

    let mut vm = new_vm_with_ops_addr_bal(
        &caller_ops,
        Address::from_low_u64_be(U256::from(1).low_u64()),
        U256::zero(),
    );

    vm.add_account(callee_address, callee_account);

    vm.execute();

    let current_call_frame = vm.current_call_frame_mut();

    let calldata = vec![
        0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF,
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E,
        0x0F, 0x10,
    ];

    let expected_data = U256::from_big_endian(&calldata[..32]);

    assert_eq!(expected_data, current_call_frame.memory.load(0));
}

#[test]
fn calldatasize() {
    let calldata = Memory::new_from_vec(vec![0x11, 0x22, 0x33]);
    let ops = vec![Operation::CallDataSize, Operation::Stop];
    let mut vm = new_vm_with_ops(&ops);

    vm.current_call_frame_mut().calldata = calldata;

    vm.execute();

    let current_call_frame = vm.current_call_frame_mut();
    let top_of_stack = current_call_frame.stack.pop().unwrap();
    assert_eq!(top_of_stack, U256::from(3));
}

#[test]
fn calldatacopy() {
    let calldata = Memory::new_from_vec(vec![0x11, 0x22, 0x33, 0x44, 0x55]);
    let ops = vec![
        Operation::Push32(U256::from(2)), // size
        Operation::Push32(U256::from(1)), // calldata_offset
        Operation::Push32(U256::from(0)), // dest_offset
        Operation::CallDataCopy,
        Operation::Stop,
    ];
    let mut vm = new_vm_with_ops(&ops);

    vm.current_call_frame_mut().calldata = calldata;

    vm.execute();

    let current_call_frame = vm.current_call_frame_mut();
    let memory = current_call_frame.memory.load_range(0, 2);
    println!("{:?}", current_call_frame);
    assert_eq!(memory, vec![0x22, 0x33]);
}

#[test]
fn returndatasize() {
    let returndata = Memory::new_from_vec(vec![0xAA, 0xBB, 0xCC]);
    let ops = vec![Operation::ReturnDataSize, Operation::Stop];
    let mut vm = new_vm_with_ops(&ops);

    vm.current_call_frame_mut().returndata = returndata;

    vm.execute();

    let current_call_frame = vm.current_call_frame_mut();
    let top_of_stack = current_call_frame.stack.pop().unwrap();
    assert_eq!(top_of_stack, U256::from(3));
}

#[test]
fn returndatacopy() {
    let returndata = Memory::new_from_vec(vec![0xAA, 0xBB, 0xCC, 0xDD]);
    let ops = vec![
        Operation::Push32(U256::from(2)), // size
        Operation::Push32(U256::from(1)), // returndata_offset
        Operation::Push32(U256::from(0)), // dest_offset
        Operation::ReturnDataCopy,
        Operation::Stop,
    ];
    let mut vm = new_vm_with_ops(&ops);

    vm.current_call_frame_mut().returndata = returndata;

    vm.execute();

    let current_call_frame = vm.current_call_frame_mut();
    let memory = current_call_frame.memory.load_range(0, 2);
    assert_eq!(memory, vec![0xBB, 0xCC]);
}

#[test]
fn returndatacopy_being_set_by_parent() {
    let callee_bytecode = callee_return_bytecode(U256::from(0xAAAAAAA));

    let callee_address = Address::from_low_u64_be(U256::from(2).low_u64());
    let callee_account = Account::new(U256::from(500000), callee_bytecode);

    let caller_ops = vec![
        Operation::Push32(U256::from(0)),       // ret_offset
        Operation::Push32(U256::from(32)),      // ret_size
        Operation::Push32(U256::from(0)),       // args_size
        Operation::Push32(U256::from(0)),       // args_offset
        Operation::Push32(U256::zero()),        // value
        Operation::Push32(U256::from(2)),       // callee address
        Operation::Push32(U256::from(100_000)), // gas
        Operation::Call,
        Operation::Push32(U256::from(32)), // size
        Operation::Push32(U256::from(32)), // returndata offset
        Operation::Push32(U256::from(0)),  // dest offset
        Operation::ReturnDataCopy,
        Operation::Stop,
    ];

    let mut vm = new_vm_with_ops_addr_bal(
        &caller_ops,
        Address::from_low_u64_be(U256::from(1).low_u64()),
        U256::zero(),
    );

    vm.add_account(callee_address, callee_account);

    vm.execute();

    let current_call_frame = vm.current_call_frame_mut();
    println!("{:?}", current_call_frame);

    let result = current_call_frame.memory.load(0);

    assert_eq!(result, U256::from(0xAAAAAAA));
}

#[test]
fn call_returns_if_bytecode_empty() {
    let callee_bytecode = vec![].into();

    let callee_address = Address::from_low_u64_be(U256::from(2).low_u64());
    let callee_address_u256 = U256::from(2);
    let callee_account = Account::new(U256::from(500000), callee_bytecode);

    let caller_ops = vec![
        Operation::Push32(U256::from(100_000)), // gas
        Operation::Push32(callee_address_u256), // address
        Operation::Push32(U256::zero()),        // value
        Operation::Push32(U256::from(0)),       // args_offset
        Operation::Push32(U256::from(0)),       // args_size
        Operation::Push32(U256::from(0)),       // ret_offset
        Operation::Push32(U256::from(32)),      // ret_size
        Operation::Call,
        Operation::Stop,
    ];

    let mut vm = new_vm_with_ops_addr_bal(
        &caller_ops,
        Address::from_low_u64_be(U256::from(1).low_u64()),
        U256::zero(),
    );

    vm.add_account(callee_address, callee_account);
    println!("to excec");
    vm.execute();

    let success = vm.current_call_frame_mut().stack.pop().unwrap();
    assert_eq!(success, U256::one());
}

#[test]
fn call_changes_callframe_and_stores() {
    let callee_return_value = U256::from(0xAAAAAAA);
    let callee_bytecode = callee_return_bytecode(callee_return_value);
    let callee_address = Address::from_low_u64_be(U256::from(2).low_u64());
    let callee_address_u256 = U256::from(2);
    let callee_account = Account::new(U256::from(500000), callee_bytecode);

    let caller_ops = vec![
        Operation::Push32(U256::from(32)),      // ret_size
        Operation::Push32(U256::from(0)),       // ret_offset
        Operation::Push32(U256::from(0)),       // args_size
        Operation::Push32(U256::from(0)),       // args_offset
        Operation::Push32(U256::zero()),        // value
        Operation::Push32(callee_address_u256), // address
        Operation::Push32(U256::from(100_000)), // gas
        Operation::Call,
        Operation::Stop,
    ];

    let mut vm = new_vm_with_ops_addr_bal(
        &caller_ops,
        Address::from_low_u64_be(U256::from(1).low_u64()),
        U256::zero(),
    );

    vm.add_account(callee_address, callee_account);

    vm.execute();

    let current_call_frame = vm.current_call_frame_mut();

    let success = current_call_frame.stack.pop().unwrap() == U256::one();
    assert!(success);

    let ret_offset = 0;
    let ret_size = 32;
    let return_data = current_call_frame.memory.load_range(ret_offset, ret_size);

    assert_eq!(U256::from_big_endian(&return_data), U256::from(0xAAAAAAA));
}

#[test]
fn nested_calls() {
    let callee3_return_value = U256::from(0xAAAAAAA);
    let callee3_bytecode = callee_return_bytecode(callee3_return_value);
    let callee3_address = Address::from_low_u64_be(U256::from(3).low_u64());
    let callee3_address_u256 = U256::from(3);
    let callee3_account = Account::new(U256::from(300_000), callee3_bytecode);

    let mut callee2_ops = vec![
        Operation::Push32(U256::from(32)),       // ret_size
        Operation::Push32(U256::from(0)),        // ret_offset
        Operation::Push32(U256::from(0)),        // args_size
        Operation::Push32(U256::from(0)),        // args_offset
        Operation::Push32(U256::zero()),         // value
        Operation::Push32(callee3_address_u256), // address
        Operation::Push32(U256::from(100_000)),  // gas
        Operation::Call,
    ];

    let callee2_return_value = U256::from(0xBBBBBBB);

    let callee2_return_bytecode = vec![
        Operation::Push32(callee2_return_value), // value
        Operation::Push32(U256::from(32)),       // offset
        Operation::Mstore,
        Operation::Push32(U256::from(64)), // size
        Operation::Push32(U256::zero()),   // offset
        Operation::Return,
    ];

    callee2_ops.extend(callee2_return_bytecode);

    let callee2_bytecode = callee2_ops
        .iter()
        .flat_map(|op| op.to_bytecode())
        .collect::<Bytes>();

    let callee2_address = Address::from_low_u64_be(U256::from(2).low_u64());
    let callee2_address_u256 = U256::from(2);

    let callee2_account = Account::new(U256::from(300_000), callee2_bytecode);

    let caller_ops = vec![
        Operation::Push32(U256::from(64)),       // ret_size
        Operation::Push32(U256::from(0)),        // ret_offset
        Operation::Push32(U256::from(0)),        // args_size
        Operation::Push32(U256::from(0)),        // args_offset
        Operation::Push32(U256::zero()),         // value
        Operation::Push32(callee2_address_u256), // address
        Operation::Push32(U256::from(100_000)),  // gas
        Operation::Call,
        Operation::Stop,
    ];

    let caller_address = Address::from_low_u64_be(U256::from(1).low_u64());
    let caller_balance = U256::from(1_000_000);

    let mut vm = new_vm_with_ops_addr_bal(&caller_ops, caller_address, caller_balance);

    vm.add_account(callee2_address, callee2_account);
    vm.add_account(callee3_address, callee3_account);

    vm.execute();

    let current_call_frame = vm.current_call_frame_mut();

    let success = current_call_frame.stack.pop().unwrap();
    assert_eq!(success, U256::one());

    let ret_offset = 0;
    let ret_size = 64;
    let return_data = current_call_frame.memory.load_range(ret_offset, ret_size);

    let mut expected_bytes = vec![0u8; 64];
    // place 0xAAAAAAA at 0..32
    let mut callee3_return_value_bytes = [0u8; 32];
    callee3_return_value.to_big_endian(&mut callee3_return_value_bytes);
    expected_bytes[..32].copy_from_slice(&callee3_return_value_bytes);

    // place 0xBBBBBBB at 32..64
    let mut callee2_return_value_bytes = [0u8; 32];
    callee2_return_value.to_big_endian(&mut callee2_return_value_bytes);
    expected_bytes[32..].copy_from_slice(&callee2_return_value_bytes);

    assert_eq!(return_data, expected_bytes);
}
