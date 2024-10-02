use bytes::Bytes;
use ethereum_types::{Address, U256};
use levm::{
    constants::{
        BSIZE_END, ECRECOVER_ADDRESS, ECRECOVER_COST, ESIZE_END, IDENTITY_ADDRESS,
        IDENTITY_STATIC_COST, MIN_MODEXP_COST, MODEXP_ADDRESS, MSIZE_END, MXP_PARAMS_OFFSET,
        REVERT_FOR_CALL, RIPEMD_160_ADDRESS, RIPEMD_160_STATIC_COST, RIPEMD_PADDING_LEN,
        SHA2_256_ADDRESS, SHA2_256_STATIC_COST, SUCCESS_FOR_CALL,
    },
    precompiles::execute_precompile,
};

// example from evm.codes https://www.evm.codes/playground?unit=Wei&codeType=Mnemonic&code=%27jFirsNplace_parameters%20in%20memoryZ456e9aea5e197a1f1af7a3e85a3212fa4049a3ba34c2289b4c860fc0b0c64ef3whash~Y~28wvX2YZ9242685bf161793cc25603c231bc2f568eb630ea16aa137d2664ac8038825608wrX4YZ4f8ae3bd7535248d0bd448298cc2e2071e56992d0774dc340c368ae950852adawsX6YqqjDo_call~32JSizeX80JOffsetX8VSize~VOffset~1waddressW4QFFFFFFFFwgasqSTATICCALLqqjPut_resulNalonKon_stackqPOPX80qMLOAD%27~W1%20w%20jq%5Cnj%2F%2F%20_%20thKZW32QY0qMSTOREX~0xWqPUSHV0wargsQ%200xNt%20Ke%20Jwret%01JKNQVWXYZ_jqw~_
#[test]
fn ecrecover_precompile_happy_path() {
    let hash =
        hex::decode("456e9aea5e197a1f1af7a3e85a3212fa4049a3ba34c2289b4c860fc0b0c64ef3").unwrap();
    let v =
        hex::decode("000000000000000000000000000000000000000000000000000000000000001c").unwrap();
    let r =
        hex::decode("9242685bf161793cc25603c231bc2f568eb630ea16aa137d2664ac8038825608").unwrap();
    let s =
        hex::decode("4f8ae3bd7535248d0bd448298cc2e2071e56992d0774dc340c368ae950852ada").unwrap();
    let calldata = vec![hash, v, r, s].concat();
    let calldata = Bytes::from(calldata);
    let expected_public_address_output =
        Bytes::from(hex::decode("7156526fbd7a3c72969b54f64e42c10fbb768c8a").unwrap());
    let gas_limit = ECRECOVER_COST + 1;
    let mut consumed_gas = 0;
    let expected_cost = ECRECOVER_COST;
    let (result, data) = execute_precompile(
        Address::from_low_u64_be(ECRECOVER_ADDRESS),
        calldata.clone(),
        gas_limit,
        &mut consumed_gas,
    );
    assert_eq!(data[12..], expected_public_address_output);
    assert_eq!(consumed_gas, expected_cost);
    assert_eq!(result, SUCCESS_FOR_CALL);
}

#[test]
fn ecrecover_precompile_wrong_recovery_identifier() {
    let hash =
        hex::decode("456e9aea5e197a1f1af7a3e85a3212fa4049a3ba34c2289b4c860fc0b0c64ef3").unwrap();
    let v =
        hex::decode("0000000000000000000000000000000000000000000000000000000000000010").unwrap();
    let r =
        hex::decode("9242685bf161793cc25603c231bc2f568eb630ea16aa137d2664ac8038825608").unwrap();
    let s =
        hex::decode("4f8ae3bd7535248d0bd448298cc2e2071e56992d0774dc340c368ae950852ada").unwrap();
    let calldata = vec![hash, v, r, s].concat();
    let calldata = Bytes::from(calldata);

    let gas_limit = ECRECOVER_COST + 1;
    let mut consumed_gas = 0;
    let (result, data) = execute_precompile(
        Address::from_low_u64_be(ECRECOVER_ADDRESS),
        calldata.clone(),
        gas_limit,
        &mut consumed_gas,
    );
    assert_eq!(data, Bytes::new());
    assert_eq!(consumed_gas, gas_limit);
    assert_eq!(result, REVERT_FOR_CALL);
}

#[test]
fn ecrecover_precompile_out_of_gas() {
    let hash =
        hex::decode("456e9aea5e197a1f1af7a3e85a3212fa4049a3ba34c2289b4c860fc0b0c64ef3").unwrap();
    let v =
        hex::decode("000000000000000000000000000000000000000000000000000000000000001c").unwrap();
    let r =
        hex::decode("9242685bf161793cc25603c231bc2f568eb630ea16aa137d2664ac8038825608").unwrap();
    let s =
        hex::decode("4f8ae3bd7535248d0bd448298cc2e2071e56992d0774dc340c368ae950852ada").unwrap();
    let calldata = vec![hash, v, r, s].concat();
    let calldata = Bytes::from(calldata);
    let mut consumed_gas = 0;
    let expected_cost = ECRECOVER_COST;
    let gas_limit = expected_cost - 1;
    let (result, data) = execute_precompile(
        Address::from_low_u64_be(ECRECOVER_ADDRESS),
        calldata.clone(),
        gas_limit,
        &mut consumed_gas,
    );
    assert_eq!(data, Bytes::new());
    assert_eq!(result, REVERT_FOR_CALL);
    assert_eq!(consumed_gas, gas_limit);
}

// example from evm.codes https://www.evm.codes/playground?unit=Wei&codeType=Mnemonic&code=%27wFirsWplaceqparameters%20in%20memorybFFjdata~0vMSTOREvvwDoqcallZSizeZ_1XSizeb1FX_2jaddressY4%200xFFFFFFFFjgasvSTATICCALLvvwPutqresulWalonVonqstackvPOPb20vMLOAD%27~Y1j%2F%2F%20v%5Cnq%20thVj%20wb~0x_Offset~Zb20jretYvPUSHXjargsWt%20Ve%20%01VWXYZ_bjqvw~_
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
    assert_eq!(consumed_gas, gas_limit);
}

// example from evm.codes https://www.evm.codes/playground?unit=Wei&codeType=Mnemonic&code=%27wFirsWplaceqparameters%20in%20memorybFFjdata~0vMSTOREvvwDoqcallZSizeZ_1XSizeb1FX_3jaddressY4%200xFFFFFFFFjgasvSTATICCALLvvwPutqresulWalonVonqstackvPOPb20vMLOAD%27~Y1j%2F%2F%20v%5Cnq%20thVj%20wb~0x_Offset~Zb20jretYvPUSHXjargsWt%20Ve%20%01VWXYZ_bjqvw~_
#[test]
fn ripemd_160_precompile_happy_path() {
    let calldata = Bytes::from(vec![0xFF]);
    let expected_output =
        Bytes::from(hex::decode("2c0c45d3ecab80fe060e5f1d7057cd2f8de5e557").unwrap());
    let gas_limit = 1000;
    let mut consumed_gas = 0;
    let expected_cost = RIPEMD_160_STATIC_COST + 120 * 1;
    let (result, data) = execute_precompile(
        Address::from_low_u64_be(RIPEMD_160_ADDRESS),
        calldata.clone(),
        gas_limit,
        &mut consumed_gas,
    );
    assert_eq!(data[RIPEMD_PADDING_LEN..], expected_output);
    assert_eq!(consumed_gas, expected_cost);
    assert_eq!(result, SUCCESS_FOR_CALL);
}

#[test]
fn ripemd_160_precompile_out_of_gas() {
    let calldata = Bytes::from(vec![0xFF]);
    let expected_cost = RIPEMD_160_STATIC_COST + 120 * 1;
    let gas_limit = expected_cost - 1;
    let mut consumed_gas = 0;
    let (result, data) = execute_precompile(
        Address::from_low_u64_be(RIPEMD_160_ADDRESS),
        calldata.clone(),
        gas_limit,
        &mut consumed_gas,
    );
    assert_eq!(data, Bytes::new());
    assert_eq!(result, REVERT_FOR_CALL);
    assert_eq!(consumed_gas, gas_limit);
}

// example from evm.codes https://www.evm.codes/playground?unit=Wei&codeType=Mnemonic&code=%27wFirsWplaceqparameters%20in%20memorybFFjdata~0vMSTOREvvwDoqcall~1QX3FQ_1YX1FY_4jaddressZ4%200xFFFFFFFFjgasvSTATICCALLvvwPutqresulWalonVonqstackvPOPb20vMLOAD%27~Z1j%2F%2F%20v%5Cnq%20thVj%20wb~0x_Offset~ZvPUSHYjargsXSizebWt%20Ve%20Qjret%01QVWXYZ_bjqvw~_

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
    assert_eq!(consumed_gas, gas_limit);
}

fn calldata_for_modexp(b_size: u16, e_size: u16, m_size: u16, b: u8, e: u8, m: u8) -> Bytes {
    let calldata_size = (b_size + e_size + m_size + MXP_PARAMS_OFFSET as u16) as usize;
    let b_data_size = U256::from(b_size);
    let e_data_size = U256::from(e_size);
    let m_data_size = U256::from(m_size);
    let e_size = e_size as usize;
    let m_size = m_size as usize;

    let mut calldata = vec![0_u8; calldata_size];
    let calldata_slice = calldata.as_mut_slice();
    b_data_size.to_big_endian(&mut calldata_slice[..BSIZE_END]);
    e_data_size.to_big_endian(&mut calldata_slice[BSIZE_END..ESIZE_END]);
    m_data_size.to_big_endian(&mut calldata_slice[ESIZE_END..MSIZE_END]);
    calldata_slice[calldata_size - m_size - e_size - 1] = b;
    calldata_slice[calldata_size - m_size - 1] = e;
    calldata_slice[calldata_size - 1] = m;

    Bytes::from(calldata_slice.to_vec())
}

#[test]
fn modexp_min_gas_cost() {
    let callee_address = Address::from_low_u64_be(MODEXP_ADDRESS);
    let calldata = calldata_for_modexp(1, 1, 1, 8, 9, 10);
    let gas_limit = 100_000_000;
    let mut consumed_gas = 0;

    let (return_code, return_data) =
        execute_precompile(callee_address, calldata, gas_limit, &mut consumed_gas);

    assert_eq!(return_code, SUCCESS_FOR_CALL);
    assert_eq!(return_data, Bytes::from(8_u8.to_be_bytes().to_vec()));
    assert_eq!(consumed_gas, MIN_MODEXP_COST);
}

#[test]
fn modexp_variable_gas_cost() {
    let callee_address = Address::from_low_u64_be(MODEXP_ADDRESS);
    let calldata = calldata_for_modexp(256, 1, 1, 8, 6, 10);
    let gas_limit = 100_000_000;
    let mut consumed_gas = 0;

    let (return_code, return_data) =
        execute_precompile(callee_address, calldata, gas_limit, &mut consumed_gas);

    assert_eq!(return_code, SUCCESS_FOR_CALL);
    assert_eq!(return_data, Bytes::from(4_u8.to_be_bytes().to_vec()));
    assert_eq!(consumed_gas, 682);
}

#[test]
fn modexp_not_enought_gas() {
    let callee_address = Address::from_low_u64_be(MODEXP_ADDRESS);
    let calldata = calldata_for_modexp(1, 1, 1, 8, 9, 10);
    let gas_limit = 199;
    let mut consumed_gas = 0;

    let (return_code, return_data) =
        execute_precompile(callee_address, calldata, gas_limit, &mut consumed_gas);

    assert_eq!(return_code, REVERT_FOR_CALL);
    assert_eq!(return_data, Bytes::new());
    assert_eq!(consumed_gas, gas_limit);
}

#[test]
fn modexp_zero_modulo() {
    let callee_address = Address::from_low_u64_be(MODEXP_ADDRESS);
    let calldata = calldata_for_modexp(1, 1, 1, 8, 9, 0);
    let gas_limit = 100_000_000;
    let mut consumed_gas = 0;

    let (return_code, return_data) =
        execute_precompile(callee_address, calldata, gas_limit, &mut consumed_gas);

    assert_eq!(return_code, SUCCESS_FOR_CALL);
    assert_eq!(return_data, Bytes::from(0_u8.to_be_bytes().to_vec()));
    assert_eq!(consumed_gas, MIN_MODEXP_COST);
}

#[test]
fn modexp_bigger_msize_than_necessary() {
    let callee_address = Address::from_low_u64_be(MODEXP_ADDRESS);
    let calldata = calldata_for_modexp(1, 1, 32, 8, 6, 10);
    let gas_limit = 100_000_000;
    let mut consumed_gas = 0;

    let (return_code, return_data) =
        execute_precompile(callee_address, calldata, gas_limit, &mut consumed_gas);

    let mut expected_return_data = 4_u8.to_be_bytes().to_vec();
    expected_return_data.resize(32, 0);
    expected_return_data.reverse();
    assert_eq!(return_code, SUCCESS_FOR_CALL);
    assert_eq!(return_data, Bytes::from(expected_return_data));
}

#[test]
fn modexp_big_sizes_for_values() {
    let callee_address = Address::from_low_u64_be(MODEXP_ADDRESS);
    let calldata = calldata_for_modexp(256, 255, 255, 8, 6, 10);
    let gas_limit = 100_000_000;
    let mut consumed_gas = 0;

    let (return_code, return_data) =
        execute_precompile(callee_address, calldata, gas_limit, &mut consumed_gas);

    let mut expected_return_data = 4_u8.to_be_bytes().to_vec();
    expected_return_data.resize(255, 0);
    expected_return_data.reverse();
    assert_eq!(return_code, SUCCESS_FOR_CALL);
    assert_eq!(return_data, Bytes::from(expected_return_data));
}

#[test]
fn modexp_with_empty_calldata() {
    let callee_address = Address::from_low_u64_be(MODEXP_ADDRESS);
    let calldata = Bytes::new();
    let gas_limit = 100_000_000;
    let mut consumed_gas = 0;

    let (return_code, return_data) =
        execute_precompile(callee_address, calldata, gas_limit, &mut consumed_gas);

    assert_eq!(return_code, SUCCESS_FOR_CALL);
    assert_eq!(return_data, Bytes::new());
    assert_eq!(consumed_gas, MIN_MODEXP_COST);
}
