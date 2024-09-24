use bytes::Bytes;
use ethereum_types::U256;
use levm::{operations::Operation, vm::VM};

#[test]
fn test() {
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

    let mut vm = VM::new(bytecode);

    vm.execute();

    assert!(vm.current_call_frame().stack.pop().unwrap() == U256::one());
    assert!(vm.current_call_frame().pc() == 68);

    println!("{vm:?}");
}

#[test]
fn mstore() {
    let operations = [
        Operation::Push32(U256::from(0x33333)),
        Operation::Push32(U256::zero()),
        Operation::Mstore,
        Operation::Msize,
        Operation::Stop,
    ];

    let bytecode = operations
        .iter()
        .flat_map(Operation::to_bytecode)
        .collect::<Bytes>();

    let mut vm = VM::new(bytecode);

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
