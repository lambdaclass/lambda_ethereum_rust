use bytes::Bytes;
use ethereum_types::U256;
use levm::{opcodes::Opcode, operations::Operation, vm::VM};

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
fn mstore() {
    let mut vm = VM::default();

    vm.stack.push(U256::from(0x33333)); // value
    vm.stack.push(U256::from(0)); // offset

    vm.execute(Bytes::from(vec![Opcode::MSTORE as u8, Opcode::STOP as u8]));

    let stored_value = vm.memory.load(0);

    assert_eq!(stored_value, U256::from(0x33333));
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
        Opcode::STOP as u8,
    ]));

    let copied_value = vm.memory.load(64);
    assert_eq!(copied_value, U256::from(0x33333));
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
        Opcode::STOP as u8,
    ]));

    let loaded_value = vm.stack.pop().unwrap();
    assert_eq!(loaded_value, U256::from(0xabcdef));

    //check with big offset

    vm.pc = 0;

    vm.stack.push(U256::from(2000)); // offset

    vm.stack.push(U256::from(0x123456)); // value
    vm.stack.push(U256::from(2000)); // offset

    vm.execute(Bytes::from(vec![
        Opcode::MSTORE as u8,
        Opcode::MLOAD as u8,
        Opcode::STOP as u8,
    ]));

    let loaded_value = vm.stack.pop().unwrap();
    assert_eq!(loaded_value, U256::from(0x123456));
}

#[test]
fn test_mload_uninitialized_memory() {
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
