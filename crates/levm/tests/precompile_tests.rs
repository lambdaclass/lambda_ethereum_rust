use bytes::Bytes;
use ethereum_types::Address;
use levm::{
    constants::{REVERT_FOR_CALL, SUCCESS_FOR_CALL},
    precompiles::{execute_precompile, IDENTITY_ADDRESS, IDENTITY_STATIC_COST},
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
    assert_eq!(result, SUCCESS_FOR_CALL as u8);
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
    assert_eq!(result, REVERT_FOR_CALL as u8);
}
