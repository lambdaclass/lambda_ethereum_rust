use bytes::Bytes;
use ethereum_types::U256;
use levm::{operations::Operation, vm::VM};

pub fn new_vm_with_ops(operations: &[Operation]) -> VM {
    let bytecode = operations
        .iter()
        .flat_map(Operation::to_bytecode)
        .collect::<Bytes>();

    VM::new(bytecode)
}

#[test]
fn push0_ok() {
    let mut vm = new_vm_with_ops(&[Operation::Push0, Operation::Stop]);

    vm.execute();

    assert_eq!(vm.current_call_frame().stack[0], U256::zero());
    assert_eq!(vm.current_call_frame().pc(), 2);
}

#[test]
fn push1_ok() {
    let to_push = U256::from_big_endian(&[0xff]);

    let operations = [Operation::Push((1, to_push)), Operation::Stop];
    let mut vm = new_vm_with_ops(&operations);

    vm.execute();

    assert_eq!(vm.current_call_frame().stack[0], to_push);
    assert_eq!(vm.current_call_frame().pc(), 3);
}

#[test]
fn push5_ok() {
    let to_push = U256::from_big_endian(&[0xff, 0xff, 0xff, 0xff, 0xff]);

    let operations = [Operation::Push((5, to_push)), Operation::Stop];
    let mut vm = new_vm_with_ops(&operations);

    vm.execute();

    assert_eq!(vm.current_call_frame().stack[0], to_push);
    assert_eq!(vm.current_call_frame().pc(), 7);
}

#[test]
fn push31_ok() {
    let to_push = U256::from_big_endian(&[0xff; 31]);

    let operations = [Operation::Push((31, to_push)), Operation::Stop];
    let mut vm = new_vm_with_ops(&operations);

    vm.execute();

    assert_eq!(vm.current_call_frame().stack[0], to_push);
    assert_eq!(vm.current_call_frame().pc(), 33);
}

#[test]
fn push32_ok() {
    let to_push = U256::from_big_endian(&[0xff; 32]);

    let operations = [Operation::Push32(to_push), Operation::Stop];
    let mut vm = new_vm_with_ops(&operations);

    vm.execute();

    assert_eq!(vm.current_call_frame().stack[0], to_push);
    assert_eq!(vm.current_call_frame().pc(), 34);
}

#[test]
fn dup1_ok() {
    let value = U256::one();

    let operations = [
        Operation::Push((1, value)),
        Operation::Dup(1),
        Operation::Stop,
    ];
    let mut vm = new_vm_with_ops(&operations);

    vm.execute();

    let stack_len = vm.current_call_frame().stack.len();

    assert_eq!(stack_len, 2);
    assert_eq!(vm.current_call_frame().pc(), 4);
    assert_eq!(vm.current_call_frame().stack[stack_len - 1], value);
    assert_eq!(vm.current_call_frame().stack[stack_len - 2], value);
}

#[test]
fn dup16_ok() {
    let value = U256::one();

    let mut operations = vec![Operation::Push((1, value))];
    operations.extend(vec![Operation::Push0; 15]);
    operations.extend(vec![Operation::Dup(16), Operation::Stop]);

    let mut vm = new_vm_with_ops(&operations);

    vm.execute();

    let stack_len = vm.current_call_frame().stack.len();

    assert_eq!(stack_len, 17);
    assert_eq!(vm.current_call_frame().pc, 19);
    assert_eq!(vm.current_call_frame().stack[stack_len - 1], value);
    assert_eq!(vm.current_call_frame().stack[stack_len - 17], value);
}

#[test]
#[should_panic]
fn dup_panics_if_stack_underflow() {
    let operations = [Operation::Dup(5), Operation::Stop];
    let mut vm = new_vm_with_ops(&operations);

    vm.execute();
}

#[test]
fn swap1_ok() {
    let bottom = U256::from_big_endian(&[0xff]);
    let top = U256::from_big_endian(&[0xee]);

    let operations = [
        Operation::Push((1, bottom)),
        Operation::Push((1, top)),
        Operation::Swap(1),
        Operation::Stop,
    ];
    let mut vm = new_vm_with_ops(&operations);
    vm.execute();

    assert_eq!(vm.current_call_frame().stack.len(), 2);
    assert_eq!(vm.current_call_frame().pc(), 6);
    assert_eq!(vm.current_call_frame().stack[0], top);
    assert_eq!(vm.current_call_frame().stack[1], bottom);
}

#[test]
fn swap16_ok() {
    let bottom = U256::from_big_endian(&[0xff]);
    let top = U256::from_big_endian(&[0xee]);

    let mut operations = vec![Operation::Push((1, bottom))];
    operations.extend(vec![Operation::Push0; 15]);
    operations.extend(vec![Operation::Push((1, top))]);
    operations.extend(vec![Operation::Swap(16), Operation::Stop]);

    let mut vm = new_vm_with_ops(&operations);

    vm.execute();
    let stack_len = vm.current_call_frame().stack.len();

    assert_eq!(stack_len, 17);
    assert_eq!(vm.current_call_frame().pc(), 21);
    assert_eq!(vm.current_call_frame().stack[stack_len - 1], bottom);
    assert_eq!(vm.current_call_frame().stack[stack_len - 1 - 16], top);
}

#[test]
#[should_panic]
fn swap_panics_if_stack_underflow() {
    let operations = [Operation::Swap(5), Operation::Stop];
    let mut vm = new_vm_with_ops(&operations);

    vm.execute();
}
