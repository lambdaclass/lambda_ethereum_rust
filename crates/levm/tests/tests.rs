use bytes::Bytes;
use ethereum_types::{Address, H256, U256};
use levm::{
    block::TARGET_BLOB_GAS_PER_BLOCK,
    constants::{MAX_CODE_SIZE, REVERT_FOR_CREATE, SUCCESS_FOR_RETURN},
    operations::Operation,
    vm::{Account, VM},
};

// cargo test -p 'levm'

fn word_to_address(word: U256) -> Address {
    let mut bytes = [0u8; 32];
    word.to_big_endian(&mut bytes);
    Address::from_slice(&bytes[12..])
}

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
    assert!(vm.current_call_frame().pc() == 68);
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

    assert!(vm.current_call_frame_mut().stack.pop().unwrap() == U256::from(100));
}

#[test]
fn call_returns_if_bytecode_empty() {
    let callee_bytecode = vec![].into();

    let callee_address = Address::from_low_u64_be(U256::from(2).low_u64());
    let callee_address_u256 = U256::from(2);
    let callee_account = Account::new(U256::from(500000), callee_bytecode, 0);

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
    let callee_account = Account::new(U256::from(500000), callee_bytecode, 0);

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
    let callee3_account = Account::new(U256::from(300_000), callee3_bytecode, 0);

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

    let callee2_account = Account::new(U256::from(300_000), callee2_bytecode, 0);

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

#[test]
fn block_hash_op() {
    let block_number = 1_u8;
    let block_hash = 12345678;
    let current_block_number = 3_u8;
    let expected_block_hash = U256::from(block_hash);

    let operations = [
        Operation::Push((1, U256::from(block_number))),
        Operation::BlockHash,
        Operation::Stop,
    ];

    let mut vm = new_vm_with_ops(&operations);
    vm.block_env.number = U256::from(current_block_number);
    vm.db
        .insert(U256::from(block_number), H256::from_low_u64_be(block_hash));

    vm.execute();

    assert_eq!(
        vm.current_call_frame_mut().stack.pop().unwrap(),
        expected_block_hash
    );
}

#[test]
fn block_hash_same_block_number() {
    let block_number = 1_u8;
    let block_hash = 12345678;
    let current_block_number = block_number;
    let expected_block_hash = U256::zero();

    let operations = [
        Operation::Push((1, U256::from(block_number))),
        Operation::BlockHash,
        Operation::Stop,
    ];

    let mut vm = new_vm_with_ops(&operations);
    vm.block_env.number = U256::from(current_block_number);
    vm.db
        .insert(U256::from(block_number), H256::from_low_u64_be(block_hash));

    vm.execute();

    assert_eq!(
        vm.current_call_frame_mut().stack.pop().unwrap(),
        expected_block_hash
    );
}

#[test]
fn block_hash_block_number_not_from_recent_256() {
    let block_number = 1_u8;
    let block_hash = 12345678;
    let current_block_number = 258;
    let expected_block_hash = U256::zero();

    let operations = [
        Operation::Push((1, U256::from(block_number))),
        Operation::BlockHash,
        Operation::Stop,
    ];

    let mut vm = new_vm_with_ops(&operations);
    vm.block_env.number = U256::from(current_block_number);
    vm.db
        .insert(U256::from(block_number), H256::from_low_u64_be(block_hash));

    vm.execute();

    assert_eq!(
        vm.current_call_frame_mut().stack.pop().unwrap(),
        expected_block_hash
    );
}

#[test]
fn coinbase_op() {
    let coinbase_address = 100;

    let operations = [Operation::Coinbase, Operation::Stop];

    let mut vm = new_vm_with_ops(&operations);
    vm.block_env.coinbase = Address::from_low_u64_be(coinbase_address);

    vm.execute();

    assert_eq!(
        vm.current_call_frame_mut().stack.pop().unwrap(),
        U256::from(coinbase_address)
    );
}

#[test]
fn timestamp_op() {
    let timestamp = U256::from(100000);

    let operations = [Operation::Timestamp, Operation::Stop];

    let mut vm = new_vm_with_ops(&operations);
    vm.block_env.timestamp = timestamp;

    vm.execute();

    assert_eq!(vm.current_call_frame_mut().stack.pop().unwrap(), timestamp);
}

#[test]
fn number_op() {
    let block_number = U256::from(1000);

    let operations = [Operation::Number, Operation::Stop];

    let mut vm = new_vm_with_ops(&operations);
    vm.block_env.number = block_number;

    vm.execute();

    assert_eq!(
        vm.current_call_frame_mut().stack.pop().unwrap(),
        block_number
    );
}

#[test]
fn prevrandao_op() {
    let prevrandao = H256::from_low_u64_be(2000);

    let operations = [Operation::Prevrandao, Operation::Stop];

    let mut vm = new_vm_with_ops(&operations);
    vm.block_env.prev_randao = Some(prevrandao);

    vm.execute();

    assert_eq!(
        vm.current_call_frame_mut().stack.pop().unwrap(),
        U256::from_big_endian(&prevrandao.0)
    );
}

#[test]
fn gaslimit_op() {
    let gas_limit = 1000;

    let operations = [Operation::Gaslimit, Operation::Stop];

    let mut vm = new_vm_with_ops(&operations);
    vm.block_env.gas_limit = gas_limit;

    vm.execute();

    assert_eq!(
        vm.current_call_frame_mut().stack.pop().unwrap(),
        U256::from(gas_limit)
    );
}

#[test]
fn chain_id_op() {
    let chain_id = 1;

    let operations = [Operation::Chainid, Operation::Stop];

    let mut vm = new_vm_with_ops(&operations);
    vm.block_env.chain_id = chain_id;

    vm.execute();

    assert_eq!(
        vm.current_call_frame_mut().stack.pop().unwrap(),
        U256::from(chain_id)
    );
}

#[test]
fn basefee_op() {
    let base_fee_per_gas = U256::from(1000);

    let operations = [Operation::Basefee, Operation::Stop];

    let mut vm = new_vm_with_ops(&operations);
    vm.block_env.base_fee_per_gas = base_fee_per_gas;

    vm.execute();

    assert_eq!(
        vm.current_call_frame_mut().stack.pop().unwrap(),
        base_fee_per_gas
    );
}

#[test]
fn blob_base_fee_op() {
    let operations = [Operation::BlobBaseFee, Operation::Stop];

    let mut vm = new_vm_with_ops(&operations);
    vm.block_env.excess_blob_gas = Some(TARGET_BLOB_GAS_PER_BLOCK * 8);
    vm.block_env.blob_gas_used = Some(0);

    vm.execute();

    assert_eq!(
        vm.current_call_frame_mut().stack.pop().unwrap(),
        U256::from(2)
    );
}

#[test]
fn blob_base_fee_minimun_cost() {
    let operations = [Operation::BlobBaseFee, Operation::Stop];

    let mut vm = new_vm_with_ops(&operations);
    vm.block_env.excess_blob_gas = Some(0);
    vm.block_env.blob_gas_used = Some(0);

    vm.execute();

    assert_eq!(
        vm.current_call_frame_mut().stack.pop().unwrap(),
        U256::one()
    );
}

#[test]
fn create_happy_path() {
    let value: u8 = 10;
    let offset: u8 = 19;
    let size: u8 = 13;
    let sender_nonce = 1;
    let sender_balance = U256::from(25);
    let sender_addr = Address::from_low_u64_be(40);

    // Code that returns the value 0xffffffff putting it in memory
    let initialization_code = hex::decode("63FFFFFFFF6000526004601CF3").unwrap();

    let operations = vec![
        // Store initialization code in memory
        Operation::Push((13, U256::from_big_endian(&initialization_code))),
        Operation::Push0,
        Operation::Mstore,
        // Create
        Operation::Push((1, U256::from(size))),
        Operation::Push((1, U256::from(offset))),
        Operation::Push((1, U256::from(value))),
        Operation::Create,
        Operation::Stop,
    ];

    let mut vm = new_vm_with_ops(&operations);
    vm.accounts.insert(
        sender_addr,
        Account::new(sender_balance, Bytes::new(), sender_nonce),
    );
    vm.current_call_frame_mut().msg_sender = sender_addr;

    vm.execute();

    let call_frame = vm.current_call_frame_mut();
    let return_of_created_callframe = call_frame.stack.pop().unwrap();
    assert_eq!(return_of_created_callframe, U256::from(SUCCESS_FOR_RETURN));
    let returned_addr = call_frame.stack.pop().unwrap();
    // check the created account is correct
    let new_account = vm.accounts.get(&word_to_address(returned_addr)).unwrap();
    assert_eq!(new_account.balance, U256::from(value));
    assert_eq!(new_account.nonce, 1);

    // Check that the sender account is updated
    let sender_account = vm.accounts.get(&sender_addr).unwrap();
    assert_eq!(sender_account.nonce, sender_nonce + 1);
    assert_eq!(sender_account.balance, sender_balance - value);
}

#[test]
fn create_with_size_longer_than_max() {
    let value: u8 = 10;
    let offset: u8 = 19;
    let size: usize = MAX_CODE_SIZE * 2 + 1;
    let sender_nonce = 1;
    let sender_balance = U256::from(25);
    let sender_addr = Address::from_low_u64_be(40);

    let operations = vec![
        // Create
        Operation::Push((16, U256::from(size))),
        Operation::Push((1, U256::from(offset))),
        Operation::Push((1, U256::from(value))),
        Operation::Create,
        Operation::Stop,
    ];

    let mut vm = new_vm_with_ops(&operations);
    vm.accounts.insert(
        sender_addr,
        Account::new(sender_balance, Bytes::new(), sender_nonce),
    );
    vm.current_call_frame_mut().msg_sender = sender_addr;

    vm.execute();

    let call_frame = vm.current_call_frame_mut();
    let create_return_value = call_frame.stack.pop().unwrap();
    assert_eq!(create_return_value, U256::from(REVERT_FOR_CREATE));

    // Check that the sender account is updated
    let sender_account = vm.accounts.get(&sender_addr).unwrap();
    assert_eq!(sender_account.nonce, sender_nonce);
    assert_eq!(sender_account.balance, sender_balance);
}
