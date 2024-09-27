use bytes::Bytes;
use ethereum_types::{Address, H256, U256};
use levm::{block::TARGET_BLOB_GAS_PER_BLOCK, operations::Operation, vm::VM};

// cargo test -p 'levm'

pub fn new_vm_with_ops(operations: &[Operation]) -> VM {
    let bytecode = operations
        .iter()
        .flat_map(Operation::to_bytecode)
        .collect::<Bytes>();

    VM::new(bytecode)
}

#[test]
fn test() {
    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::one()),
        Operation::Push32(U256::zero()),
        Operation::Add,
        Operation::Stop,
    ]);

    vm.execute();

    assert!(vm.current_call_frame().stack.pop().unwrap() == U256::one());
    assert!(vm.current_call_frame().pc() == 68);

    println!("{vm:?}");
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

    let result = vm.current_call_frame().stack.pop().unwrap();
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

    let result = vm.current_call_frame().stack.pop().unwrap();
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

    let result = vm.current_call_frame().stack.pop().unwrap();
    assert_eq!(result, U256::from(0xF0F0));

    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::from(0xF000)),
        Operation::Push32(U256::from(0xF0F0)),
        Operation::And,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame().stack.pop().unwrap();
    assert_eq!(result, U256::from(0xF000));

    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::from(0xB020)),
        Operation::Push32(U256::from(0x1F0F)),
        Operation::And,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame().stack.pop().unwrap();
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

    let result = vm.current_call_frame().stack.pop().unwrap();
    assert_eq!(result, U256::from(0b1110));

    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::from(0b1010)),
        Operation::Push32(U256::zero()),
        Operation::Or,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame().stack.pop().unwrap();
    assert_eq!(result, U256::from(0b1010));

    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::from(u64::MAX)),
        Operation::Push32(U256::zero()),
        Operation::Or,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame().stack.pop().unwrap();
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

    let result = vm.current_call_frame().stack.pop().unwrap();
    assert_eq!(result, U256::from(0xFFFF));

    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::from(0xF000)),
        Operation::Push32(U256::from(0xF0F0)),
        Operation::Or,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame().stack.pop().unwrap();
    assert_eq!(result, U256::from(0xF0F0));

    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::from(0xB020)),
        Operation::Push32(U256::from(0x1F0F)),
        Operation::Or,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame().stack.pop().unwrap();
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

    let result = vm.current_call_frame().stack.pop().unwrap();
    assert_eq!(result, U256::from(0b110));

    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::from(0b1010)),
        Operation::Push32(U256::zero()),
        Operation::Xor,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame().stack.pop().unwrap();
    assert_eq!(result, U256::from(0b1010));

    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::from(u64::MAX)),
        Operation::Push32(U256::zero()),
        Operation::Xor,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame().stack.pop().unwrap();
    assert_eq!(result, U256::from(u64::MAX));

    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::from(u64::MAX)),
        Operation::Push32(U256::from(u64::MAX)),
        Operation::Xor,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame().stack.pop().unwrap();
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

    let result = vm.current_call_frame().stack.pop().unwrap();
    assert_eq!(result, U256::from(0xFF));

    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::from(0xFF)),
        Operation::Push32(U256::from(0xFF)),
        Operation::Xor,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame().stack.pop().unwrap();
    assert_eq!(result, U256::zero());

    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::from(0xFFFF)),
        Operation::Push32(U256::from(0xF0F0)),
        Operation::Xor,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame().stack.pop().unwrap();
    assert_eq!(result, U256::from(0xF0F));

    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::from(0xF000)),
        Operation::Push32(U256::from(0xF0F0)),
        Operation::Xor,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame().stack.pop().unwrap();
    assert_eq!(result, U256::from(0xF0));

    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::from(0x4C0F)),
        Operation::Push32(U256::from(0x3A4B)),
        Operation::Xor,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame().stack.pop().unwrap();
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

    let result = vm.current_call_frame().stack.pop().unwrap();
    let expected = !U256::from(0b1010);
    assert_eq!(result, expected);

    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::MAX),
        Operation::Not,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame().stack.pop().unwrap();
    assert_eq!(result, U256::zero());

    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::zero()),
        Operation::Not,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame().stack.pop().unwrap();
    assert_eq!(result, U256::MAX);

    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::from(1)),
        Operation::Not,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame().stack.pop().unwrap();
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

    let result = vm.current_call_frame().stack.pop().unwrap();
    assert_eq!(result, U256::from(0xF1));

    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::from(0x33ED)),
        Operation::Push32(U256::from(30)),
        Operation::Byte,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame().stack.pop().unwrap();
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

    let result = vm.current_call_frame().stack.pop().unwrap();
    assert_eq!(result, U256::from(0xFF));

    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::MAX),
        Operation::Push32(U256::from(12)),
        Operation::Byte,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame().stack.pop().unwrap();
    assert_eq!(result, U256::from(0xFF));

    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::from(0x00E0D0000)),
        Operation::Push32(U256::from(29)),
        Operation::Byte,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame().stack.pop().unwrap();
    assert_eq!(result, U256::from(0x0D));

    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::from(0xFDEA179)),
        Operation::Push32(U256::from(50)),
        Operation::Byte,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame().stack.pop().unwrap();
    assert_eq!(result, U256::zero());

    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::from(0xFDEA179)),
        Operation::Push32(U256::from(32)),
        Operation::Byte,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame().stack.pop().unwrap();
    assert_eq!(result, U256::zero());

    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::zero()),
        Operation::Push32(U256::from(15)),
        Operation::Byte,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame().stack.pop().unwrap();
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

    let result = vm.current_call_frame().stack.pop().unwrap();
    assert_eq!(result, U256::from(0x90));

    let mut vm = new_vm_with_ops(&[
        Operation::Push32(word),
        Operation::Push32(U256::from(7)),
        Operation::Byte,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame().stack.pop().unwrap();
    assert_eq!(result, U256::from(0x57));

    let mut vm = new_vm_with_ops(&[
        Operation::Push32(word),
        Operation::Push32(U256::from(19)),
        Operation::Byte,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame().stack.pop().unwrap();
    assert_eq!(result, U256::from(0xDD));

    let mut vm = new_vm_with_ops(&[
        Operation::Push32(word),
        Operation::Push32(U256::from(31)),
        Operation::Byte,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame().stack.pop().unwrap();
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

    let result = vm.current_call_frame().stack.pop().unwrap();
    assert_eq!(result, U256::from(0xDDDD));

    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::from(0x12345678)),
        Operation::Push32(U256::from(1)),
        Operation::Shl,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame().stack.pop().unwrap();
    assert_eq!(result, U256::from(0x2468acf0));

    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::from(0x12345678)),
        Operation::Push32(U256::from(4)),
        Operation::Shl,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame().stack.pop().unwrap();
    assert_eq!(result, U256::from(4886718336_u64));

    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::from(0xFF)),
        Operation::Push32(U256::from(4)),
        Operation::Shl,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame().stack.pop().unwrap();
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

    let result = vm.current_call_frame().stack.pop().unwrap();
    assert_eq!(result, U256::zero());

    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::zero()),
        Operation::Push32(U256::from(200)),
        Operation::Shl,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame().stack.pop().unwrap();
    assert_eq!(result, U256::zero());

    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::MAX),
        Operation::Push32(U256::from(1)),
        Operation::Shl,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame().stack.pop().unwrap();
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

    let result = vm.current_call_frame().stack.pop().unwrap();
    assert_eq!(result, U256::from(0xDDDD));

    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::from(0x12345678)),
        Operation::Push32(U256::from(1)),
        Operation::Shr,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame().stack.pop().unwrap();
    assert_eq!(result, U256::from(0x91a2b3c));

    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::from(0x12345678)),
        Operation::Push32(U256::from(4)),
        Operation::Shr,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame().stack.pop().unwrap();
    assert_eq!(result, U256::from(0x1234567));

    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::from(0xFF)),
        Operation::Push32(U256::from(4)),
        Operation::Shr,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame().stack.pop().unwrap();
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

    let result = vm.current_call_frame().stack.pop().unwrap();
    assert_eq!(result, U256::zero());

    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::zero()),
        Operation::Push32(U256::from(200)),
        Operation::Shr,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame().stack.pop().unwrap();
    assert_eq!(result, U256::zero());

    let mut vm = new_vm_with_ops(&[
        Operation::Push32(U256::MAX),
        Operation::Push32(U256::from(1)),
        Operation::Shr,
        Operation::Stop,
    ]);

    vm.execute();

    let result = vm.current_call_frame().stack.pop().unwrap();
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

    let result = vm.current_call_frame().stack.pop().unwrap();
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

    let result = vm.current_call_frame().stack.pop().unwrap();
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

    let result = vm.current_call_frame().stack.pop().unwrap();
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

    let result = vm.current_call_frame().stack.pop().unwrap();
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

    let result = vm.current_call_frame().stack.pop().unwrap();
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
        vm.current_call_frame().stack.pop().unwrap()
            == U256::from("0x29045a592007d0c246ef02c2223570da9522d0cf0f73282c79a1bc8f0bb2c238")
    );
    assert!(vm.current_call_frame().pc() == 40);
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
        vm.current_call_frame().stack.pop().unwrap()
            == U256::from("0xae75624a7d0413029c1e0facdd38cc8e177d9225892e2490a69c2f1f89512061")
    );
    assert!(vm.current_call_frame().pc() == 40);
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
        vm.current_call_frame().stack.pop().unwrap()
            == U256::from("0xc5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470")
    );
    assert!(vm.current_call_frame().pc() == 4);
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
        vm.current_call_frame().stack.pop().unwrap()
            == U256::from("0xe8e77626586f73b955364c7b4bbf0bb7f7685ebd40e852b164633a4acbd3244c")
    );
    assert!(vm.current_call_frame().pc() == 41);
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

    assert_eq!(vm.current_call_frame().stack.pop().unwrap(), U256::from(32));
    assert_eq!(vm.current_call_frame().pc(), 69);
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

    let stored_value = vm.current_call_frame().memory.load(0);

    assert_eq!(stored_value, U256::from(0x33333));

    let memory_size = vm.current_call_frame().stack.pop().unwrap();
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

    let mut vm = VM::new(bytecode);

    vm.execute();

    let stored_value = vm.current_call_frame().memory.load(0);

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

    let mut vm = VM::new(bytecode);

    vm.execute();

    let copied_value = vm.current_call_frame().memory.load(64);
    assert_eq!(copied_value, U256::from(0x33333));

    let memory_size = vm.current_call_frame().stack.pop().unwrap();
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

    let mut vm = VM::new(bytecode);

    vm.execute();

    let loaded_value = vm.current_call_frame().stack.pop().unwrap();
    assert_eq!(loaded_value, U256::from(0x33333));
}

#[test]
fn msize() {
    let operations = [Operation::Msize, Operation::Stop];

    let bytecode = operations
        .iter()
        .flat_map(Operation::to_bytecode)
        .collect::<Bytes>();

    let mut vm = VM::new(bytecode);

    vm.execute();

    let initial_size = vm.current_call_frame().stack.pop().unwrap();
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

    vm = VM::new(bytecode);

    vm.execute();

    let after_store_size = vm.current_call_frame().stack.pop().unwrap();
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

    vm = VM::new(bytecode);

    vm.execute();

    let final_size = vm.current_call_frame().stack.pop().unwrap();
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

    let mut vm = VM::new(bytecode);

    vm.execute();

    let memory_size = vm.current_call_frame().stack.pop().unwrap();
    let loaded_value = vm.current_call_frame().stack.pop().unwrap();

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

    vm = VM::new(bytecode);

    vm.execute();

    let memory_size = vm.current_call_frame().stack.pop().unwrap();
    let loaded_value = vm.current_call_frame().stack.pop().unwrap();

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

    let mut vm = VM::new(bytecode);

    vm.execute();

    let memory_size = vm.current_call_frame().stack.pop().unwrap();
    let loaded_value = vm.current_call_frame().stack.pop().unwrap();

    assert_eq!(loaded_value, U256::zero());
    assert_eq!(memory_size, U256::from(96));
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
        .insert_block_hash(U256::from(block_number), H256::from_low_u64_be(block_hash));

    vm.execute();

    assert_eq!(
        vm.current_call_frame().stack.pop().unwrap(),
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
        .insert_block_hash(U256::from(block_number), H256::from_low_u64_be(block_hash));

    vm.execute();

    assert_eq!(
        vm.current_call_frame().stack.pop().unwrap(),
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
        .insert_block_hash(U256::from(block_number), H256::from_low_u64_be(block_hash));

    vm.execute();

    assert_eq!(
        vm.current_call_frame().stack.pop().unwrap(),
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
        vm.current_call_frame().stack.pop().unwrap(),
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

    assert_eq!(vm.current_call_frame().stack.pop().unwrap(), timestamp);
}

#[test]
fn number_op() {
    let block_number = U256::from(1000);

    let operations = [Operation::Number, Operation::Stop];

    let mut vm = new_vm_with_ops(&operations);
    vm.block_env.number = block_number;

    vm.execute();

    assert_eq!(vm.current_call_frame().stack.pop().unwrap(), block_number);
}

#[test]
fn prevrandao_op() {
    let prevrandao = H256::from_low_u64_be(2000);

    let operations = [Operation::Prevrandao, Operation::Stop];

    let mut vm = new_vm_with_ops(&operations);
    vm.block_env.prev_randao = Some(prevrandao);

    vm.execute();

    assert_eq!(
        vm.current_call_frame().stack.pop().unwrap(),
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
        vm.current_call_frame().stack.pop().unwrap(),
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
        vm.current_call_frame().stack.pop().unwrap(),
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
        vm.current_call_frame().stack.pop().unwrap(),
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

    assert_eq!(vm.current_call_frame().stack.pop().unwrap(), U256::from(2));
}

#[test]
fn blob_base_fee_minimun_cost() {
    let operations = [Operation::BlobBaseFee, Operation::Stop];

    let mut vm = new_vm_with_ops(&operations);
    vm.block_env.excess_blob_gas = Some(0);
    vm.block_env.blob_gas_used = Some(0);

    vm.execute();

    assert_eq!(vm.current_call_frame().stack.pop().unwrap(), U256::one());
}
