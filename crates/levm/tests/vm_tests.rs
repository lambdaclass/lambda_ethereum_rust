use ethereum_types::U256;
use levm::{operations::Operation, vm::VM};

#[test]
fn push0_ok() {
    let mut vm = VM::default();

    let operations = [Operation::Push0, Operation::Stop];
    let bytecode = operations.iter().flat_map(Operation::to_bytecode).collect();

    vm.execute(bytecode);

    assert_eq!(vm.stack[0], U256::zero());
    assert_eq!(vm.pc, 2);
}

#[test]
fn push1_ok() {
    let mut vm = VM::default();

    let to_push = U256::from_big_endian(&[0xff]);

    let operations = [Operation::Push((1, to_push)), Operation::Stop];
    let bytecode = operations.iter().flat_map(Operation::to_bytecode).collect();

    vm.execute(bytecode);

    assert_eq!(vm.stack[0], to_push);
    assert_eq!(vm.pc, 3);
}

#[test]
fn push5_ok() {
    let mut vm = VM::default();

    let to_push = U256::from_big_endian(&[0xff, 0xff, 0xff, 0xff, 0xff]);

    let operations = [Operation::Push((5, to_push)), Operation::Stop];
    let bytecode = operations.iter().flat_map(Operation::to_bytecode).collect();

    vm.execute(bytecode);

    assert_eq!(vm.stack[0], to_push);
    assert_eq!(vm.pc, 7);
}

#[test]
fn push31_ok() {
    let mut vm = VM::default();

    let to_push = U256::from_big_endian(&[0xff; 31]);

    let operations = [Operation::Push((31, to_push)), Operation::Stop];
    let bytecode = operations.iter().flat_map(Operation::to_bytecode).collect();

    vm.execute(bytecode);

    assert_eq!(vm.stack[0], to_push);
    assert_eq!(vm.pc, 33);
}

#[test]
fn push32_ok() {
    let mut vm = VM::default();

    let to_push = U256::from_big_endian(&[0xff; 32]);

    let operations = [Operation::Push32(to_push), Operation::Stop];
    let bytecode = operations.iter().flat_map(Operation::to_bytecode).collect();

    vm.execute(bytecode);

    assert_eq!(vm.stack[0], to_push);
    assert_eq!(vm.pc, 34);
}

#[test]
fn dup1_ok() {
    let mut vm = VM::default();
    let value = U256::one();

    let operations = [
        Operation::Push((1, value)),
        Operation::Dup(1),
        Operation::Stop,
    ];
    let bytecode = operations.iter().flat_map(Operation::to_bytecode).collect();

    vm.execute(bytecode);

    assert_eq!(vm.stack.len(), 2);
    assert_eq!(vm.pc, 4);
    assert_eq!(vm.stack[vm.stack.len() - 1], value);
    assert_eq!(vm.stack[vm.stack.len() - 2], value);
}

#[test]
fn dup16_ok() {
    let mut vm = VM::default();
    let value = U256::one();

    let mut operations = vec![Operation::Push((1, value))];
    operations.extend(vec![Operation::Push0; 15]);
    operations.extend(vec![Operation::Dup(16), Operation::Stop]);

    let bytecode = operations.iter().flat_map(Operation::to_bytecode).collect();

    vm.execute(bytecode);

    assert_eq!(vm.stack.len(), 17);
    assert_eq!(vm.pc, 19);
    assert_eq!(vm.stack[vm.stack.len() - 1], value);
    assert_eq!(vm.stack[vm.stack.len() - 17], value);
}

#[test]
#[should_panic]
fn dup_panics_if_stack_underflow() {
    let mut vm = VM::default();

    let operations = [Operation::Dup(5), Operation::Stop];
    let bytecode = operations.iter().flat_map(Operation::to_bytecode).collect();

    vm.execute(bytecode);
}

#[test]
fn swap1_ok() {
    let mut vm = VM::default();
    let bottom = U256::from_big_endian(&[0xff]);
    let top = U256::from_big_endian(&[0xee]);

    let operations = [
        Operation::Push((1, bottom)),
        Operation::Push((1, top)),
        Operation::Swap(1),
        Operation::Stop,
    ];
    let bytecode = operations.iter().flat_map(Operation::to_bytecode).collect();

    vm.execute(bytecode);

    assert_eq!(vm.stack.len(), 2);
    assert_eq!(vm.pc, 6);
    assert_eq!(vm.stack[0], top);
    assert_eq!(vm.stack[1], bottom);
}

#[test]
fn swap16_ok() {
    let mut vm = VM::default();
    let bottom = U256::from_big_endian(&[0xff]);
    let top = U256::from_big_endian(&[0xee]);

    let mut operations = vec![Operation::Push((1, bottom))];
    operations.extend(vec![Operation::Push0; 15]);
    operations.extend(vec![Operation::Push((1, top))]);
    operations.extend(vec![Operation::Swap(16), Operation::Stop]);

    let bytecode = operations.iter().flat_map(Operation::to_bytecode).collect();

    vm.execute(bytecode);

    assert_eq!(vm.stack.len(), 17);
    assert_eq!(vm.pc, 21);
    assert_eq!(vm.stack[vm.stack.len() - 1], bottom);
    assert_eq!(vm.stack[vm.stack.len() - 1 - 16], top);
}

#[test]
#[should_panic]
fn swap_panics_if_stack_underflow() {
    let mut vm = VM::default();

    let operations = [Operation::Swap(5), Operation::Stop];
    let bytecode = operations.iter().flat_map(Operation::to_bytecode).collect();

    vm.execute(bytecode);
}

#[test]
fn transient_store() {
    let mut vm = VM::default();

    assert!(vm.transient_storage.is_empty());

    let value = U256::from_big_endian(&[0xaa; 3]);
    let key = U256::from_big_endian(&[0xff; 2]);

    let operations = [
        Operation::Push32(value),
        Operation::Push32(key),
        Operation::Tstore,
        Operation::Stop,
    ];

    let bytecode = operations.iter().flat_map(Operation::to_bytecode).collect();

    vm.execute(bytecode);

    assert_eq!(vm.transient_storage.get(vm.caller, key), value)
}

#[test]
#[should_panic]
fn transient_store_no_values_panics() {
    let mut vm = VM::default();

    assert!(vm.transient_storage.is_empty());

    let operations = [Operation::Tstore, Operation::Stop];

    let bytecode = operations.iter().flat_map(Operation::to_bytecode).collect();

    vm.execute(bytecode);
}

#[test]
fn transient_load() {
    let value = U256::from_big_endian(&[0xaa; 3]);
    let key = U256::from_big_endian(&[0xff; 2]);

    let mut vm = VM::default();

    vm.transient_storage.set(vm.caller, key, value);

    let operations = [Operation::Push32(key), Operation::Tload, Operation::Stop];

    let bytecode = operations.iter().flat_map(Operation::to_bytecode).collect();

    vm.execute(bytecode);

    assert_eq!(vm.stack.pop().unwrap(), value)
}
