use levm::{
    operations::Operation,
    primitives::{Bytes, U256},
};

#[test]
fn push0_correct_bytecode() {
    let op = Operation::Push0.to_bytecode();
    assert_eq!(op, Bytes::from(vec![0x5f]))
}

#[test]
fn push1_correct_bytecode() {
    let op = Operation::Push((1, 0xff.into())).to_bytecode();
    assert_eq!(op, Bytes::from(vec![0x60, 0xff]))
}

#[test]
fn push31_correct_bytecode() {
    let op = Operation::Push((31, U256::from_big_endian(&[0xff; 31]))).to_bytecode();
    let mut expected = vec![0x7e];
    expected.extend_from_slice(&[0xff; 31]);
    assert_eq!(op, Bytes::from(expected))
}

#[test]
fn push32_correct_bytecode() {
    let op = Operation::Push((32, U256::MAX)).to_bytecode();
    let mut expected = vec![0x7f];
    expected.extend_from_slice(&[0xff; 32]);
    assert_eq!(op, Bytes::from(expected))
}

#[test]
#[should_panic]
fn push_value_greater_than_fits_in_pushn_panics() {
    Operation::Push((1, 0xfff.into())).to_bytecode();
}

#[test]
#[should_panic]
fn push_greater_than_32_panics() {
    Operation::Push((33, U256::zero())).to_bytecode();
}

#[test]
fn dup1_ok() {
    let op = Operation::Dup(1).to_bytecode();
    assert_eq!(op, Bytes::from(vec![0x80]))
}

#[test]
fn dup16_ok() {
    let op = Operation::Dup(16).to_bytecode();
    assert_eq!(op, Bytes::from(vec![0x8f]))
}

#[test]
#[should_panic]
fn dup_more_than_16_panics() {
    Operation::Dup(17).to_bytecode();
}

#[test]
fn swap1_ok() {
    let op = Operation::Swap(1).to_bytecode();
    assert_eq!(op, Bytes::from(vec![0x90]))
}

#[test]
fn swap16_ok() {
    let op = Operation::Swap(16).to_bytecode();
    assert_eq!(op, Bytes::from(vec![0x9f]))
}

#[test]
#[should_panic]
fn swap_more_than_16_panics() {
    Operation::Swap(17).to_bytecode();
}
