use bytes::Bytes;
use ethereum_types::Address;
use levm::{
    constants::{
        IDENTITY_ADDRESS, IDENTITY_STATIC_COST, REVERT_FOR_CALL, SHA2_256_ADDRESS,
        SHA2_256_STATIC_COST, SUCCESS_FOR_CALL,
    },
    precompiles::execute_precompile,
};

#[test]
fn identity_precompile_happy_path() {
    let calldata = Bytes::from(vec![0x01]);
    let gas_limit = 100;
    let mut consumed_gas = 0;
    let expected_cost = IDENTITY_STATIC_COST + 3 * 1;
    let (result, data) = execute_precompile(
        Address::from_low_u64_be(IDENTITY_ADDRESS),
        calldata.clone(),
        gas_limit,
        &mut consumed_gas,
    );
    assert_eq!(data, calldata);
    assert_eq!(consumed_gas, expected_cost);
    assert_eq!(result, SUCCESS_FOR_CALL);
}

#[test]
fn identity_precompile_out_of_gas() {
    let calldata = Bytes::from(vec![0x01]);
    let expected_cost = IDENTITY_STATIC_COST + 3 * 1;
    let gas_limit = expected_cost - 1;
    let mut consumed_gas = 0;
    let (result, data) = execute_precompile(
        Address::from_low_u64_be(IDENTITY_ADDRESS),
        calldata.clone(),
        gas_limit,
        &mut consumed_gas,
    );
    assert_eq!(data, Bytes::new());
    assert_eq!(result, REVERT_FOR_CALL);
}

// example output from https://www.evm.codes/precompiled for sha2_256
#[test]
fn sha2_256_precompile_happy_path() {
    let calldata = Bytes::from(vec![0xFF]);
    let expected_output = Bytes::from(
        hex::decode("a8100ae6aa1940d0b663bb31cd466142ebbdbd5187131b92d93818987832eb89").unwrap(),
    );
    let gas_limit = 100;
    let mut consumed_gas = 0;
    let expected_cost = SHA2_256_STATIC_COST + 12 * 1;
    let (result, data) = execute_precompile(
        Address::from_low_u64_be(SHA2_256_ADDRESS),
        calldata.clone(),
        gas_limit,
        &mut consumed_gas,
    );
    assert_eq!(data, expected_output);
    assert_eq!(consumed_gas, expected_cost);
    assert_eq!(result, SUCCESS_FOR_CALL);
}

#[test]
fn sha2_256_precompile_out_of_gas() {
    let calldata = Bytes::from(vec![0xFF]);
    let expected_cost = SHA2_256_STATIC_COST + 12 * 1;
    let gas_limit = expected_cost - 1;
    let mut consumed_gas = 0;
    let (result, data) = execute_precompile(
        Address::from_low_u64_be(SHA2_256_ADDRESS),
        calldata.clone(),
        gas_limit,
        &mut consumed_gas,
    );
    assert_eq!(data, Bytes::new());
    assert_eq!(result, REVERT_FOR_CALL);
}
