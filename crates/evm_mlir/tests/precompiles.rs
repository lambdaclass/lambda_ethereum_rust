mod evm;

use ethereum_rust_evm_mlir::{
    constants::precompiles::{
        BLAKE2F_ADDRESS, ECADD_ADDRESS, ECMUL_ADDRESS, ECPAIRING_ADDRESS, ECRECOVER_ADDRESS,
        IDENTITY_ADDRESS, MODEXP_ADDRESS, RIPEMD_160_ADDRESS, SHA2_256_ADDRESS,
    },
    db::{Bytecode, Db},
    env::TransactTo,
    primitives::{Address, Bytes},
    program::{Operation, Program},
    Env,
};

use crate::evm::{
    append_return_result_operations, run_program_assert_bytes_result, run_program_assert_halt,
    run_program_assert_revert,
};

use num_bigint::BigUint;

#[test]
fn staticcall_on_precompile_ecrecover_happy_path() {
    let gas = 100_000_000_u32;
    let args_offset = 0_u8;
    let args_size = 128_u8;
    let ret_offset = 128_u8;
    let ret_size = 32_u8;
    let hash =
        hex::decode("456e9aea5e197a1f1af7a3e85a3212fa4049a3ba34c2289b4c860fc0b0c64ef3").unwrap();
    let v: u8 = 28;
    let r =
        hex::decode("9242685bf161793cc25603c231bc2f568eb630ea16aa137d2664ac8038825608").unwrap();
    let s =
        hex::decode("4f8ae3bd7535248d0bd448298cc2e2071e56992d0774dc340c368ae950852ada").unwrap();
    let callee_address = Address::from_low_u64_be(ECRECOVER_ADDRESS);
    let caller_address = Address::from_low_u64_be(4040);

    let expected_result =
        hex::decode("0000000000000000000000007156526fbd7a3c72969b54f64e42c10fbb768c8a").unwrap();

    let caller_ops = vec![
        // Place the parameters in memory
        Operation::Push((32_u8, BigUint::from_bytes_be(&hash))),
        Operation::Push((1_u8, BigUint::ZERO)),
        Operation::Mstore,
        Operation::Push((1_u8, BigUint::from(v))),
        Operation::Push((1_u8, BigUint::from(0x20_u8))),
        Operation::Mstore,
        Operation::Push((32_u8, BigUint::from_bytes_be(&r))),
        Operation::Push((1_u8, BigUint::from(0x40_u8))),
        Operation::Mstore,
        Operation::Push((32_u8, BigUint::from_bytes_be(&s))),
        Operation::Push((1_u8, BigUint::from(0x60_u8))),
        Operation::Mstore,
        // Do the call
        Operation::Push((1_u8, BigUint::from(ret_size))), //Ret size
        Operation::Push((1_u8, BigUint::from(ret_offset))), //Ret offset
        Operation::Push((1_u8, BigUint::from(args_size))), //Args size
        Operation::Push((1_u8, BigUint::from(args_offset))), //Args offset
        Operation::Push((20_u8, BigUint::from_bytes_be(callee_address.as_bytes()))), //Address
        Operation::Push((32_u8, BigUint::from(gas))),     //Gas
        Operation::StaticCall,
        // Return
        Operation::Push((1_u8, ret_size.into())),
        Operation::Push((1_u8, ret_offset.into())),
        Operation::Return,
    ];

    let program = Program::from(caller_ops);
    let caller_bytecode = Bytecode::from(program.to_bytecode());
    let mut env = Env::default();
    let db = Db::new().with_contract(caller_address, caller_bytecode);
    env.tx.transact_to = TransactTo::Call(caller_address);

    run_program_assert_bytes_result(env, db, &expected_result);
}

#[test]
fn staticcall_on_precompile_ecrecover_without_gas() {
    let gas = 0_u32;
    let args_offset = 0_u8;
    let args_size = 128_u8;
    let ret_offset = 128_u8;
    let ret_size = 32_u8;
    let hash =
        hex::decode("456e9aea5e197a1f1af7a3e85a3212fa4049a3ba34c2289b4c860fc0b0c64ef3").unwrap();
    let v: u8 = 28;
    let r =
        hex::decode("9242685bf161793cc25603c231bc2f568eb630ea16aa137d2664ac8038825608").unwrap();
    let s =
        hex::decode("4f8ae3bd7535248d0bd448298cc2e2071e56992d0774dc340c368ae950852ada").unwrap();
    let callee_address = Address::from_low_u64_be(ECRECOVER_ADDRESS);
    let caller_address = Address::from_low_u64_be(4040);

    let expected_result = [0_u8; 32];

    let caller_ops = vec![
        // Place the parameters in memory
        Operation::Push((32_u8, BigUint::from_bytes_be(&hash))),
        Operation::Push((1_u8, BigUint::ZERO)),
        Operation::Mstore,
        Operation::Push((1_u8, BigUint::from(v))),
        Operation::Push((1_u8, BigUint::from(0x20_u8))),
        Operation::Mstore,
        Operation::Push((32_u8, BigUint::from_bytes_be(&r))),
        Operation::Push((1_u8, BigUint::from(0x40_u8))),
        Operation::Mstore,
        Operation::Push((32_u8, BigUint::from_bytes_be(&s))),
        Operation::Push((1_u8, BigUint::from(0x60_u8))),
        Operation::Mstore,
        // Do the call
        Operation::Push((1_u8, BigUint::from(ret_size))), //Ret size
        Operation::Push((1_u8, BigUint::from(ret_offset))), //Ret offset
        Operation::Push((1_u8, BigUint::from(args_size))), //Args size
        Operation::Push((1_u8, BigUint::from(args_offset))), //Args offset
        Operation::Push((20_u8, BigUint::from_bytes_be(callee_address.as_bytes()))), //Address
        Operation::Push((32_u8, BigUint::from(gas))),     //Gas
        Operation::StaticCall,
        // Return
        Operation::Push((1_u8, ret_size.into())),
        Operation::Push((1_u8, ret_offset.into())),
        Operation::Return,
    ];

    let program = Program::from(caller_ops);
    let caller_bytecode = Bytecode::from(program.to_bytecode());
    let mut env = Env::default();
    let db = Db::new().with_contract(caller_address, caller_bytecode);
    env.tx.transact_to = TransactTo::Call(caller_address);

    run_program_assert_bytes_result(env, db, &expected_result);
}

#[test]
fn staticcall_on_precompile_identity_happy_path() {
    let gas: u32 = 100_000_000;
    let args_offset: u8 = 31;
    let args_size: u8 = 1;
    let ret_offset: u8 = 63;
    let ret_size: u8 = 1;
    let data: u8 = 0xff;
    let callee_address = Address::from_low_u64_be(IDENTITY_ADDRESS);
    let caller_address = Address::from_low_u64_be(4040);

    let expected_result = [0xff];

    let caller_ops = vec![
        // Place the parameter in memory
        Operation::Push((1_u8, BigUint::from(data))),
        Operation::Push((1_u8, BigUint::ZERO)),
        Operation::Mstore,
        // Do the call
        Operation::Push((1_u8, BigUint::from(ret_size))), //Ret size
        Operation::Push((1_u8, BigUint::from(ret_offset))), //Ret offset
        Operation::Push((1_u8, BigUint::from(args_size))), //Args size
        Operation::Push((1_u8, BigUint::from(args_offset))), //Args offset
        Operation::Push((20_u8, BigUint::from_bytes_be(callee_address.as_bytes()))), //Address
        Operation::Push((32_u8, BigUint::from(gas))),     //Gas
        Operation::StaticCall,
        // Return
        Operation::Push((1_u8, ret_size.into())),
        Operation::Push((1_u8, ret_offset.into())),
        Operation::Return,
    ];

    let program = Program::from(caller_ops);
    let caller_bytecode = Bytecode::from(program.to_bytecode());
    let mut env = Env::default();
    let db = Db::new().with_contract(caller_address, caller_bytecode);
    env.tx.transact_to = TransactTo::Call(caller_address);

    run_program_assert_bytes_result(env, db, &expected_result);
}

#[test]
fn staticcall_on_precompile_sha2_256_happy_path() {
    let gas: u32 = 100_000_000;
    let args_offset: u8 = 31;
    let args_size: u8 = 1;
    let ret_offset: u8 = 32;
    let ret_size: u8 = 32;
    let data: u8 = 0xff;
    let callee_address = Address::from_low_u64_be(SHA2_256_ADDRESS);
    let caller_address = Address::from_low_u64_be(4040);

    let expected_result =
        hex::decode("a8100ae6aa1940d0b663bb31cd466142ebbdbd5187131b92d93818987832eb89").unwrap();

    let mut caller_ops = vec![
        // Place the parameter in memory
        Operation::Push((1_u8, BigUint::from(data))),
        Operation::Push((1_u8, BigUint::ZERO)),
        Operation::Mstore,
        // Do the call
        Operation::Push((1_u8, BigUint::from(ret_size))), //Ret size
        Operation::Push((1_u8, BigUint::from(ret_offset))), //Ret offset
        Operation::Push((1_u8, BigUint::from(args_size))), //Args size
        Operation::Push((1_u8, BigUint::from(args_offset))), //Args offset
        Operation::Push((20_u8, BigUint::from_bytes_be(callee_address.as_bytes()))), //Address
        Operation::Push((32_u8, BigUint::from(gas))),     //Gas
        Operation::StaticCall,
        // Return
        Operation::Push((1_u8, 32_u8.into())),
        Operation::Push((1_u8, 32_u8.into())),
        Operation::Return,
    ];

    append_return_result_operations(&mut caller_ops);

    let program = Program::from(caller_ops);
    let caller_bytecode = Bytecode::from(program.to_bytecode());
    let mut env = Env::default();
    let db = Db::new().with_contract(caller_address, caller_bytecode);
    env.tx.transact_to = TransactTo::Call(caller_address);

    run_program_assert_bytes_result(env, db, &expected_result);
}

#[test]
fn staticcall_on_precompile_ripemd_160_happy_path() {
    let gas: u32 = 100_000_000;
    let args_offset: u8 = 31;
    let args_size: u8 = 1;
    let ret_offset: u8 = 0;
    let ret_size: u8 = 32;
    let data: u8 = 0xff;
    let callee_address = Address::from_low_u64_be(RIPEMD_160_ADDRESS);
    let caller_address = Address::from_low_u64_be(4040);

    let expected_result = hex::decode("2c0c45d3ecab80fe060e5f1d7057cd2f8de5e557").unwrap();

    let mut caller_ops = vec![
        // Place the parameter in memory
        Operation::Push((1_u8, BigUint::from(data))),
        Operation::Push((1_u8, BigUint::ZERO)),
        Operation::Mstore,
        // Do the call
        Operation::Push((1_u8, BigUint::from(ret_size))), //Ret size
        Operation::Push((1_u8, BigUint::from(ret_offset))), //Ret offset
        Operation::Push((1_u8, BigUint::from(args_size))), //Args size
        Operation::Push((1_u8, BigUint::from(args_offset))), //Args offset
        Operation::Push((20_u8, BigUint::from_bytes_be(callee_address.as_bytes()))), //Address
        Operation::Push((32_u8, BigUint::from(gas))),     //Gas
        Operation::StaticCall,
        // Return
        Operation::Push((1_u8, 20_u8.into())),
        Operation::Push((1_u8, 12_u8.into())),
        Operation::Return,
    ];

    append_return_result_operations(&mut caller_ops);

    let program = Program::from(caller_ops);
    let caller_bytecode = Bytecode::from(program.to_bytecode());
    let mut env = Env::default();
    let db = Db::new().with_contract(caller_address, caller_bytecode);
    env.tx.transact_to = TransactTo::Call(caller_address);

    run_program_assert_bytes_result(env, db, &expected_result);
}

#[test]
fn staticcall_on_precompile_modexp_happy_path() {
    let ret_size: u8 = 2;
    let ret_offset: u8 = 159;
    let args_size: u8 = 100; // bsize (32) + esize (32) + msize (32) + b (1) + e (1) + m (2). In total, 100 bytes
    let args_offset: u8 = 0;
    let gas: u32 = 100_000_000;
    let callee_address = Address::from_low_u64_be(MODEXP_ADDRESS);
    let caller_address = Address::from_low_u64_be(4040);

    let b_size: u8 = 1;
    let e_size: u8 = 1;
    let m_size: u8 = 2;
    // Word with b = 8, e = 9, m = 501
    let params =
        &hex::decode("080901F500000000000000000000000000000000000000000000000000000000").unwrap();

    // 329 = (8 ^ 9) mod 501
    let expected_result = 329_u16.to_be_bytes();

    let caller_ops = vec![
        // Store the parameters in memory
        Operation::Push((1_u8, b_size.into())),
        Operation::Push((1_u8, 0_u8.into())),
        Operation::Mstore,
        Operation::Push((1_u8, e_size.into())),
        Operation::Push((1_u8, 0x20_u8.into())),
        Operation::Mstore,
        Operation::Push((1_u8, m_size.into())),
        Operation::Push((1_u8, 0x40_u8.into())),
        Operation::Mstore,
        Operation::Push((32_u8, BigUint::from_bytes_be(params))),
        Operation::Push((1_u8, 0x60_u8.into())),
        Operation::Mstore,
        // Do the call
        Operation::Push((1_u8, ret_size.into())), //Ret size
        Operation::Push((1_u8, ret_offset.into())), //Ret offset
        Operation::Push((1_u8, args_size.into())), //Args size
        Operation::Push((1_u8, args_offset.into())), //Args offset
        Operation::Push((20_u8, BigUint::from_bytes_be(callee_address.as_bytes()))), //Address
        Operation::Push((32_u8, gas.into())),     //Gas
        Operation::StaticCall,
        // Return
        Operation::Push((1_u8, ret_size.into())),
        Operation::Push((1_u8, ret_offset.into())),
        Operation::Return,
    ];

    let program = Program::from(caller_ops);
    let caller_bytecode = Bytecode::from(program.to_bytecode());
    let mut env = Env::default();
    let db = Db::new().with_contract(caller_address, caller_bytecode);
    env.tx.transact_to = TransactTo::Call(caller_address);

    run_program_assert_bytes_result(env, db, &expected_result);
}

#[test]
fn staticcall_on_precompile_ecadd_happy_path() {
    let ret_size: u8 = 64;
    let ret_offset: u8 = 128;
    let args_size: u8 = 128;
    let args_offset: u8 = 0;
    let gas: u32 = 100_000_000;
    let callee_address = Address::from_low_u64_be(ECADD_ADDRESS);
    let caller_address = Address::from_low_u64_be(4040);

    let x1: u8 = 1;
    let y1: u8 = 2;
    let x2: u8 = 1;
    let y2: u8 = 2;

    let expected_x =
        hex::decode("030644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd3").unwrap();
    let expected_y =
        hex::decode("15ed738c0e0a7c92e7845f96b2ae9c0a68a6a449e3538fc7ff3ebf7a5a18a2c4").unwrap();
    let expected_result = [expected_x, expected_y].concat();

    let caller_ops = vec![
        // Store the parameters in memory
        Operation::Push((32_u8, x1.into())),
        Operation::Push((1_u8, 0_u8.into())),
        Operation::Mstore,
        Operation::Push((32_u8, y1.into())),
        Operation::Push((1_u8, 0x20_u8.into())),
        Operation::Mstore,
        Operation::Push((32_u8, x2.into())),
        Operation::Push((1_u8, 0x40_u8.into())),
        Operation::Mstore,
        Operation::Push((32_u8, y2.into())),
        Operation::Push((1_u8, 0x60_u8.into())),
        Operation::Mstore,
        // Do the call
        Operation::Push((1_u8, ret_size.into())), //Ret size
        Operation::Push((1_u8, ret_offset.into())), //Ret offset
        Operation::Push((1_u8, args_size.into())), //Args size
        Operation::Push((1_u8, args_offset.into())), //Args offset
        Operation::Push((20_u8, BigUint::from_bytes_be(callee_address.as_bytes()))), //Address
        Operation::Push((32_u8, gas.into())),     //Gas
        Operation::StaticCall,
        // Return
        Operation::Push((1_u8, ret_size.into())),
        Operation::Push((1_u8, ret_offset.into())),
        Operation::Return,
    ];

    let program = Program::from(caller_ops);
    let caller_bytecode = Bytecode::from(program.to_bytecode());
    let mut env = Env::default();
    let db = Db::new().with_contract(caller_address, caller_bytecode);
    env.tx.transact_to = TransactTo::Call(caller_address);

    run_program_assert_bytes_result(env, db, &expected_result);
}

#[test]
fn staticcall_on_precompile_ecadd_infinity_with_valid_point() {
    let ret_size: u8 = 64;
    let ret_offset: u8 = 128;
    let args_size: u8 = 128;
    let args_offset: u8 = 0;
    let gas: u32 = 100_000_000;
    let callee_address = Address::from_low_u64_be(ECADD_ADDRESS);
    let caller_address = Address::from_low_u64_be(4040);

    let x1: u8 = 0;
    let y1: u8 = 0;
    let x2: u8 = 1;
    let y2: u8 = 2;

    let expected_x =
        hex::decode("0000000000000000000000000000000000000000000000000000000000000001").unwrap();
    let expected_y =
        hex::decode("0000000000000000000000000000000000000000000000000000000000000002").unwrap();
    let expected_result = [expected_x, expected_y].concat();

    let caller_ops = vec![
        // Store the parameters in memory
        Operation::Push((32_u8, x1.into())),
        Operation::Push((1_u8, 0_u8.into())),
        Operation::Mstore,
        Operation::Push((32_u8, y1.into())),
        Operation::Push((1_u8, 0x20_u8.into())),
        Operation::Mstore,
        Operation::Push((32_u8, x2.into())),
        Operation::Push((1_u8, 0x40_u8.into())),
        Operation::Mstore,
        Operation::Push((32_u8, y2.into())),
        Operation::Push((1_u8, 0x60_u8.into())),
        Operation::Mstore,
        // Do the call
        Operation::Push((1_u8, ret_size.into())), // Ret size
        Operation::Push((1_u8, ret_offset.into())), // Ret offset
        Operation::Push((1_u8, args_size.into())), // Args size
        Operation::Push((1_u8, args_offset.into())), // Args offset
        Operation::Push((20_u8, BigUint::from_bytes_be(callee_address.as_bytes()))), // Address
        Operation::Push((32_u8, gas.into())),     // Gas
        Operation::StaticCall,
        // Return
        Operation::Push((1_u8, ret_size.into())),
        Operation::Push((1_u8, ret_offset.into())),
        Operation::Return,
    ];

    let program = Program::from(caller_ops);
    let caller_bytecode = Bytecode::from(program.to_bytecode());
    let mut env = Env::default();
    let db = Db::new().with_contract(caller_address, caller_bytecode);
    env.tx.transact_to = TransactTo::Call(caller_address);

    run_program_assert_bytes_result(env, db, &expected_result);
}

#[test]
fn staticcall_on_precompile_ecadd_valid_point_with_infinity() {
    let ret_size: u8 = 64;
    let ret_offset: u8 = 128;
    let args_size: u8 = 128;
    let args_offset: u8 = 0;
    let gas: u32 = 100_000_000;
    let callee_address = Address::from_low_u64_be(ECADD_ADDRESS);
    let caller_address = Address::from_low_u64_be(4040);

    let x1: u8 = 1;
    let y1: u8 = 2;
    let x2: u8 = 0;
    let y2: u8 = 0;

    let expected_x =
        hex::decode("0000000000000000000000000000000000000000000000000000000000000001").unwrap();
    let expected_y =
        hex::decode("0000000000000000000000000000000000000000000000000000000000000002").unwrap();
    let expected_result = [expected_x, expected_y].concat();

    let caller_ops = vec![
        // Store the parameters in memory
        Operation::Push((32_u8, x1.into())),
        Operation::Push((1_u8, 0_u8.into())),
        Operation::Mstore,
        Operation::Push((32_u8, y1.into())),
        Operation::Push((1_u8, 0x20_u8.into())),
        Operation::Mstore,
        Operation::Push((32_u8, x2.into())),
        Operation::Push((1_u8, 0x40_u8.into())),
        Operation::Mstore,
        Operation::Push((32_u8, y2.into())),
        Operation::Push((1_u8, 0x60_u8.into())),
        Operation::Mstore,
        // Do the call
        Operation::Push((1_u8, ret_size.into())), // Ret size
        Operation::Push((1_u8, ret_offset.into())), // Ret offset
        Operation::Push((1_u8, args_size.into())), // Args size
        Operation::Push((1_u8, args_offset.into())), // Args offset
        Operation::Push((20_u8, BigUint::from_bytes_be(callee_address.as_bytes()))), // Address
        Operation::Push((32_u8, gas.into())),     // Gas
        Operation::StaticCall,
        // Return
        Operation::Push((1_u8, ret_size.into())),
        Operation::Push((1_u8, ret_offset.into())),
        Operation::Return,
    ];

    let program = Program::from(caller_ops);
    let caller_bytecode = Bytecode::from(program.to_bytecode());
    let mut env = Env::default();
    let db = Db::new().with_contract(caller_address, caller_bytecode);
    env.tx.transact_to = TransactTo::Call(caller_address);

    run_program_assert_bytes_result(env, db, &expected_result);
}

#[test]
fn staticcall_on_precompile_ecadd_infinity_twice() {
    let ret_size: u8 = 64;
    let ret_offset: u8 = 128;
    let args_size: u8 = 128;
    let args_offset: u8 = 0;
    let gas: u32 = 100_000_000;
    let callee_address = Address::from_low_u64_be(ECADD_ADDRESS);
    let caller_address = Address::from_low_u64_be(4040);

    let x1: u8 = 0;
    let y1: u8 = 0;
    let x2: u8 = 0;
    let y2: u8 = 0;

    let expected_result = Bytes::from([0u8; 64].to_vec());

    let caller_ops = vec![
        // Store the parameters in memory
        Operation::Push((32_u8, x1.into())),
        Operation::Push((1_u8, 0_u8.into())),
        Operation::Mstore,
        Operation::Push((32_u8, y1.into())),
        Operation::Push((1_u8, 0x20_u8.into())),
        Operation::Mstore,
        Operation::Push((32_u8, x2.into())),
        Operation::Push((1_u8, 0x40_u8.into())),
        Operation::Mstore,
        Operation::Push((32_u8, y2.into())),
        Operation::Push((1_u8, 0x60_u8.into())),
        Operation::Mstore,
        // Do the call
        Operation::Push((1_u8, ret_size.into())), // Ret size
        Operation::Push((1_u8, ret_offset.into())), // Ret offset
        Operation::Push((1_u8, args_size.into())), // Args size
        Operation::Push((1_u8, args_offset.into())), // Args offset
        Operation::Push((20_u8, BigUint::from_bytes_be(callee_address.as_bytes()))), // Address
        Operation::Push((32_u8, gas.into())),     // Gas
        Operation::StaticCall,
        // Return
        Operation::Push((1_u8, ret_size.into())),
        Operation::Push((1_u8, ret_offset.into())),
        Operation::Return,
    ];

    let program = Program::from(caller_ops);
    let caller_bytecode = Bytecode::from(program.to_bytecode());
    let mut env = Env::default();
    let db = Db::new().with_contract(caller_address, caller_bytecode);
    env.tx.transact_to = TransactTo::Call(caller_address);

    run_program_assert_bytes_result(env, db, &expected_result);
}

#[test]
fn staticcall_on_precompile_ecadd_with_invalid_first_point() {
    let ret_size: u8 = 64;
    let ret_offset: u8 = 128;
    let args_size: u8 = 128;
    let args_offset: u8 = 0;
    let gas: u32 = 100_000;
    let callee_address = Address::from_low_u64_be(ECADD_ADDRESS);
    let caller_address = Address::from_low_u64_be(4040);

    let x1: u8 = 1;
    let y1: u8 = 1;
    let x2: u8 = 1;
    let y2: u8 = 2;

    let mut jumpdest: u8 = (33 * 4) + (3 * 4); // parameters store
    jumpdest += (2 * 4) + 21 + 33 + 1; // call
    jumpdest += 1 + 2 + 1 + (2 * 2) + 1; // check and return

    let caller_ops = vec![
        // Store the parameters in memory
        Operation::Push((32_u8, x1.into())),
        Operation::Push((1_u8, 0_u8.into())),
        Operation::Mstore,
        Operation::Push((32_u8, y1.into())),
        Operation::Push((1_u8, 0x20_u8.into())),
        Operation::Mstore,
        Operation::Push((32_u8, x2.into())),
        Operation::Push((1_u8, 0x40_u8.into())),
        Operation::Mstore,
        Operation::Push((32_u8, y2.into())),
        Operation::Push((1_u8, 0x60_u8.into())),
        Operation::Mstore,
        // Do the call
        Operation::Push((1_u8, ret_size.into())), // Ret size
        Operation::Push((1_u8, ret_offset.into())), // Ret offset
        Operation::Push((1_u8, args_size.into())), // Args size
        Operation::Push((1_u8, args_offset.into())), // Args offset
        Operation::Push((20_u8, BigUint::from_bytes_be(callee_address.as_bytes()))), // Address
        Operation::Push((32_u8, gas.into())),     // Gas
        Operation::StaticCall,
        // Check if STATICCALL returned 0 (failure)
        Operation::IsZero,
        Operation::Push((1_u8, jumpdest.into())), // Push the location of revert
        Operation::Jumpi,
        // Continue execution if STATICCALL returned 1 (shouldn't happen)
        Operation::Push((1_u8, ret_size.into())), // Ret size
        Operation::Push((1_u8, ret_offset.into())), // Ret offset
        Operation::Return,
        // Revert
        Operation::Jumpdest {
            pc: jumpdest as usize,
        },
        Operation::Push0, // Ret size
        Operation::Push0, // Ret offset
        Operation::Revert,
    ];

    let program = Program::from(caller_ops);
    let caller_bytecode = Bytecode::from(program.to_bytecode());
    let mut env = Env::default();
    let db = Db::new().with_contract(caller_address, caller_bytecode);
    env.tx.transact_to = TransactTo::Call(caller_address);

    run_program_assert_revert(env, db);
}

#[test]
fn staticcall_on_precompile_ecadd_with_invalid_second_point() {
    let ret_size: u8 = 64;
    let ret_offset: u8 = 128;
    let args_size: u8 = 128;
    let args_offset: u8 = 0;
    let gas: u32 = 100_000;
    let callee_address = Address::from_low_u64_be(ECADD_ADDRESS);
    let caller_address = Address::from_low_u64_be(4040);

    let x1: u8 = 1;
    let y1: u8 = 2;
    let x2: u8 = 1;
    let y2: u8 = 1;

    let mut jumpdest: u8 = (33 * 4) + (3 * 4); // parameters store
    jumpdest += (2 * 4) + 21 + 33 + 1; // call
    jumpdest += 1 + 2 + 1 + (2 * 2) + 1; // check and return

    let caller_ops = vec![
        // Store the parameters in memory
        Operation::Push((32_u8, x1.into())),
        Operation::Push((1_u8, 0_u8.into())),
        Operation::Mstore,
        Operation::Push((32_u8, y1.into())),
        Operation::Push((1_u8, 0x20_u8.into())),
        Operation::Mstore,
        Operation::Push((32_u8, x2.into())),
        Operation::Push((1_u8, 0x40_u8.into())),
        Operation::Mstore,
        Operation::Push((32_u8, y2.into())),
        Operation::Push((1_u8, 0x60_u8.into())),
        Operation::Mstore,
        // Do the call
        Operation::Push((1_u8, ret_size.into())), // Ret size
        Operation::Push((1_u8, ret_offset.into())), // Ret offset
        Operation::Push((1_u8, args_size.into())), // Args size
        Operation::Push((1_u8, args_offset.into())), // Args offset
        Operation::Push((20_u8, BigUint::from_bytes_be(callee_address.as_bytes()))), // Address
        Operation::Push((32_u8, gas.into())),     // Gas
        Operation::StaticCall,
        // Check if STATICCALL returned 0 (failure)
        Operation::IsZero,
        Operation::Push((1_u8, jumpdest.into())), // Push the location of revert
        Operation::Jumpi,
        // Continue execution if STATICCALL returned 1 (shouldn't happen)
        Operation::Push((1_u8, ret_size.into())), // Ret size
        Operation::Push((1_u8, ret_offset.into())), // Ret offset
        Operation::Return,
        // Revert
        Operation::Jumpdest {
            pc: jumpdest as usize,
        },
        Operation::Push0, // Ret size
        Operation::Push0, // Ret offset
        Operation::Revert,
    ];

    let program = Program::from(caller_ops);
    let caller_bytecode = Bytecode::from(program.to_bytecode());
    let mut env = Env::default();
    let db = Db::new().with_contract(caller_address, caller_bytecode);
    env.tx.transact_to = TransactTo::Call(caller_address);

    run_program_assert_revert(env, db);
}

#[test]
fn staticcall_on_precompile_ecadd_with_missing_stack_parameter() {
    let ret_size: u8 = 64;
    let ret_offset: u8 = 128;
    let args_size: u8 = 128;
    let args_offset: u8 = 0;
    let gas: u32 = 100_000;
    let callee_address = Address::from_low_u64_be(ECADD_ADDRESS);
    let caller_address = Address::from_low_u64_be(4040);

    let x1: u8 = 1;
    let y1: u8 = 2;
    let x2: u8 = 1;
    let y2: u8 = 2;

    let caller_ops = vec![
        // Store the parameters in memory
        Operation::Push((32_u8, x1.into())),
        Operation::Push((1_u8, 0_u8.into())),
        Operation::Mstore,
        Operation::Push((32_u8, y1.into())),
        Operation::Push((1_u8, 0x20_u8.into())),
        Operation::Mstore,
        Operation::Push((32_u8, x2.into())),
        Operation::Push((1_u8, 0x40_u8.into())),
        Operation::Mstore,
        Operation::Push((32_u8, y2.into())),
        Operation::Push((1_u8, 0x60_u8.into())),
        Operation::Mstore,
        // Do the call
        // Operation::Push((1_u8, ret_size.into())), // Ret size missing
        Operation::Push((1_u8, ret_offset.into())), // Ret offset
        Operation::Push((1_u8, args_size.into())),  // Args size
        Operation::Push((1_u8, args_offset.into())), // Args offset
        Operation::Push((20_u8, BigUint::from_bytes_be(callee_address.as_bytes()))), // Address
        Operation::Push((32_u8, gas.into())),       // Gas
        Operation::StaticCall,
        // Return
        Operation::Push((1_u8, ret_size.into())),
        Operation::Push((1_u8, ret_offset.into())),
        Operation::Return,
    ];

    let program = Program::from(caller_ops);
    let caller_bytecode = Bytecode::from(program.to_bytecode());
    let mut env = Env::default();
    let db = Db::new().with_contract(caller_address, caller_bytecode);
    env.tx.transact_to = TransactTo::Call(caller_address);

    run_program_assert_halt(env, db);
}

#[test]
fn staticcall_on_precompile_ecadd_with_not_enough_gas() {
    let ret_size: u8 = 64;
    let ret_offset: u8 = 128;
    let args_size: u8 = 128;
    let args_offset: u8 = 0;
    let gas: u32 = 149;
    let callee_address = Address::from_low_u64_be(ECADD_ADDRESS);
    let caller_address = Address::from_low_u64_be(4040);

    let x1: u8 = 1;
    let y1: u8 = 2;
    let x2: u8 = 1;
    let y2: u8 = 2;

    let mut jumpdest: u8 = (33 * 4) + (3 * 4); // parameters store
    jumpdest += (2 * 4) + 21 + 33 + 1; // call
    jumpdest += 1 + 2 + 1 + (2 * 2) + 1; // check and return

    let caller_ops = vec![
        // Store the parameters in memory
        Operation::Push((32_u8, x1.into())),
        Operation::Push((1_u8, 0_u8.into())),
        Operation::Mstore,
        Operation::Push((32_u8, y1.into())),
        Operation::Push((1_u8, 0x20_u8.into())),
        Operation::Mstore,
        Operation::Push((32_u8, x2.into())),
        Operation::Push((1_u8, 0x40_u8.into())),
        Operation::Mstore,
        Operation::Push((32_u8, y2.into())),
        Operation::Push((1_u8, 0x60_u8.into())),
        Operation::Mstore,
        // Do the call
        Operation::Push((1_u8, ret_size.into())), // Ret size
        Operation::Push((1_u8, ret_offset.into())), // Ret offset
        Operation::Push((1_u8, args_size.into())), // Args size
        Operation::Push((1_u8, args_offset.into())), // Args offset
        Operation::Push((20_u8, BigUint::from_bytes_be(callee_address.as_bytes()))), // Address
        Operation::Push((32_u8, gas.into())),     // Gas
        Operation::StaticCall,
        // Check if STATICCALL returned 0 (failure)
        Operation::IsZero,
        Operation::Push((1_u8, jumpdest.into())), // Push the location of revert
        Operation::Jumpi,
        // Continue execution if STATICCALL returned 1 (shouldn't happen)
        Operation::Push((1_u8, ret_size.into())), // Ret size
        Operation::Push((1_u8, ret_offset.into())), // Ret offset
        Operation::Return,
        // Revert
        Operation::Jumpdest {
            pc: jumpdest as usize,
        },
        Operation::Push0, // Ret size
        Operation::Push0, // Ret offset
        Operation::Revert,
    ];

    let program = Program::from(caller_ops);
    let caller_bytecode = Bytecode::from(program.to_bytecode());
    let mut env = Env::default();
    let db = Db::new().with_contract(caller_address, caller_bytecode);
    env.tx.transact_to = TransactTo::Call(caller_address);

    run_program_assert_revert(env, db);
}

#[test]
fn staticcall_on_precompile_ecmul_happy_path() {
    let ret_size: u8 = 64;
    let ret_offset: u8 = 96;
    let args_size: u8 = 96;
    let args_offset: u8 = 0;
    let gas: u32 = 100_000_000;
    let callee_address = Address::from_low_u64_be(ECMUL_ADDRESS);
    let caller_address = Address::from_low_u64_be(4040);

    let x1: u8 = 1;
    let y1: u8 = 2;
    let s: u8 = 2;

    let expected_x =
        hex::decode("030644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd3").unwrap();
    let expected_y =
        hex::decode("15ed738c0e0a7c92e7845f96b2ae9c0a68a6a449e3538fc7ff3ebf7a5a18a2c4").unwrap();
    let expected_result = [expected_x, expected_y].concat();

    let caller_ops = vec![
        // Store the parameters in memory
        Operation::Push((32_u8, x1.into())),
        Operation::Push((1_u8, 0_u8.into())),
        Operation::Mstore,
        Operation::Push((32_u8, y1.into())),
        Operation::Push((1_u8, 0x20_u8.into())),
        Operation::Mstore,
        Operation::Push((32_u8, s.into())),
        Operation::Push((1_u8, 0x40_u8.into())),
        Operation::Mstore,
        // Do the call
        Operation::Push((1_u8, ret_size.into())), //Ret size
        Operation::Push((1_u8, ret_offset.into())), //Ret offset
        Operation::Push((1_u8, args_size.into())), //Args size
        Operation::Push((1_u8, args_offset.into())), //Args offset
        Operation::Push((20_u8, BigUint::from_bytes_be(callee_address.as_bytes()))), //Address
        Operation::Push((32_u8, gas.into())),     //Gas
        Operation::StaticCall,
        // Return
        Operation::Push((1_u8, ret_size.into())),
        Operation::Push((1_u8, ret_offset.into())),
        Operation::Return,
    ];

    let program = Program::from(caller_ops);
    let caller_bytecode = Bytecode::from(program.to_bytecode());
    let mut env = Env::default();
    let db = Db::new().with_contract(caller_address, caller_bytecode);
    env.tx.transact_to = TransactTo::Call(caller_address);

    run_program_assert_bytes_result(env, db, &expected_result);
}

#[test]
fn staticcall_on_precompile_ecmul_infinity() {
    let ret_size: u8 = 64;
    let ret_offset: u8 = 96;
    let args_size: u8 = 96;
    let args_offset: u8 = 0;
    let gas: u32 = 100_000_000;
    let callee_address = Address::from_low_u64_be(ECMUL_ADDRESS);
    let caller_address = Address::from_low_u64_be(4040);

    let x1: u8 = 0;
    let y1: u8 = 0;
    let s: u8 = 2;

    let expected_result = Bytes::from([0u8; 64].to_vec());

    let caller_ops = vec![
        // Store the parameters in memory
        Operation::Push((32_u8, x1.into())),
        Operation::Push((1_u8, 0_u8.into())),
        Operation::Mstore,
        Operation::Push((32_u8, y1.into())),
        Operation::Push((1_u8, 0x20_u8.into())),
        Operation::Mstore,
        Operation::Push((32_u8, s.into())),
        Operation::Push((1_u8, 0x40_u8.into())),
        Operation::Mstore,
        // Do the call
        Operation::Push((1_u8, ret_size.into())), // Ret size
        Operation::Push((1_u8, ret_offset.into())), // Ret offset
        Operation::Push((1_u8, args_size.into())), // Args size
        Operation::Push((1_u8, args_offset.into())), // Args offset
        Operation::Push((20_u8, BigUint::from_bytes_be(callee_address.as_bytes()))), // Address
        Operation::Push((32_u8, gas.into())),     // Gas
        Operation::StaticCall,
        // Return
        Operation::Push((1_u8, ret_size.into())),
        Operation::Push((1_u8, ret_offset.into())),
        Operation::Return,
    ];

    let program = Program::from(caller_ops);
    let caller_bytecode = Bytecode::from(program.to_bytecode());
    let mut env = Env::default();
    let db = Db::new().with_contract(caller_address, caller_bytecode);
    env.tx.transact_to = TransactTo::Call(caller_address);

    run_program_assert_bytes_result(env, db, &expected_result);
}

#[test]
fn staticcall_on_precompile_ecmul_by_zero() {
    let ret_size: u8 = 64;
    let ret_offset: u8 = 96;
    let args_size: u8 = 96;
    let args_offset: u8 = 0;
    let gas: u32 = 100_000_000;
    let callee_address = Address::from_low_u64_be(ECMUL_ADDRESS);
    let caller_address = Address::from_low_u64_be(4040);

    let x1: u8 = 1;
    let y1: u8 = 2;
    let s: u8 = 0;

    let expected_result = Bytes::from([0u8; 64].to_vec());

    let caller_ops = vec![
        // Store the parameters in memory
        Operation::Push((32_u8, x1.into())),
        Operation::Push((1_u8, 0_u8.into())),
        Operation::Mstore,
        Operation::Push((32_u8, y1.into())),
        Operation::Push((1_u8, 0x20_u8.into())),
        Operation::Mstore,
        Operation::Push((32_u8, s.into())),
        Operation::Push((1_u8, 0x40_u8.into())),
        Operation::Mstore,
        // Do the call
        Operation::Push((1_u8, ret_size.into())), // Ret size
        Operation::Push((1_u8, ret_offset.into())), // Ret offset
        Operation::Push((1_u8, args_size.into())), // Args size
        Operation::Push((1_u8, args_offset.into())), // Args offset
        Operation::Push((20_u8, BigUint::from_bytes_be(callee_address.as_bytes()))), // Address
        Operation::Push((32_u8, gas.into())),     // Gas
        Operation::StaticCall,
        // Return
        Operation::Push((1_u8, ret_size.into())),
        Operation::Push((1_u8, ret_offset.into())),
        Operation::Return,
    ];

    let program = Program::from(caller_ops);
    let caller_bytecode = Bytecode::from(program.to_bytecode());
    let mut env = Env::default();
    let db = Db::new().with_contract(caller_address, caller_bytecode);
    env.tx.transact_to = TransactTo::Call(caller_address);

    run_program_assert_bytes_result(env, db, &expected_result);
}

#[test]
fn staticcall_on_precompile_ecmul_invalid_point() {
    let ret_size: u8 = 64;
    let ret_offset: u8 = 96;
    let args_size: u8 = 96;
    let args_offset: u8 = 0;
    let gas: u32 = 100_000_000;
    let callee_address = Address::from_low_u64_be(ECMUL_ADDRESS);
    let caller_address = Address::from_low_u64_be(4040);

    let x1: u8 = 1;
    let y1: u8 = 1;
    let s: u8 = 2;

    let mut jumpdest: u8 = (33 * 3) + (3 * 3); // parameters store
    jumpdest += (2 * 4) + 21 + 33 + 1; // call
    jumpdest += 1 + 2 + 1 + (2 * 2) + 1; // check and return

    let caller_ops = vec![
        // Store the parameters in memory
        Operation::Push((32_u8, x1.into())),
        Operation::Push((1_u8, 0_u8.into())),
        Operation::Mstore,
        Operation::Push((32_u8, y1.into())),
        Operation::Push((1_u8, 0x20_u8.into())),
        Operation::Mstore,
        Operation::Push((32_u8, s.into())),
        Operation::Push((1_u8, 0x40_u8.into())),
        Operation::Mstore,
        // Do the call
        Operation::Push((1_u8, ret_size.into())), // Ret size
        Operation::Push((1_u8, ret_offset.into())), // Ret offset
        Operation::Push((1_u8, args_size.into())), // Args size
        Operation::Push((1_u8, args_offset.into())), // Args offset
        Operation::Push((20_u8, BigUint::from_bytes_be(callee_address.as_bytes()))), // Address
        Operation::Push((32_u8, gas.into())),     // Gas
        Operation::StaticCall,
        // Check if STATICCALL returned 0 (failure)
        Operation::IsZero,
        Operation::Push((1_u8, jumpdest.into())), // Push the location of revert
        Operation::Jumpi,
        // Continue execution if STATICCALL returned 1 (shouldn't happen)
        Operation::Push((1_u8, ret_size.into())), // Ret size
        Operation::Push((1_u8, ret_offset.into())), // Ret offset
        Operation::Return,
        // Revert
        Operation::Jumpdest {
            pc: jumpdest as usize,
        },
        Operation::Push0, // Ret size
        Operation::Push0, // Ret offset
        Operation::Revert,
    ];

    let program = Program::from(caller_ops);
    let caller_bytecode = Bytecode::from(program.to_bytecode());
    let mut env = Env::default();
    let db = Db::new().with_contract(caller_address, caller_bytecode);
    env.tx.transact_to = TransactTo::Call(caller_address);

    run_program_assert_revert(env, db);
}

#[test]
fn staticcall_on_precompile_ecmul_with_missing_stack_parameter() {
    let ret_offset: u8 = 96;
    let args_size: u8 = 96;
    let args_offset: u8 = 0;
    let gas: u32 = 100_000_000;
    let callee_address = Address::from_low_u64_be(ECMUL_ADDRESS);
    let caller_address = Address::from_low_u64_be(4040);

    let x1: u8 = 1;
    let y1: u8 = 2;
    let s: u8 = 2;

    let caller_ops = vec![
        // Store the parameters in memory
        Operation::Push((32_u8, x1.into())),
        Operation::Push((1_u8, 0_u8.into())),
        Operation::Mstore,
        Operation::Push((32_u8, y1.into())),
        Operation::Push((1_u8, 0x20_u8.into())),
        Operation::Mstore,
        Operation::Push((32_u8, s.into())),
        Operation::Push((1_u8, 0x40_u8.into())),
        Operation::Mstore,
        // Do the call
        // Operation::Push((1_u8, ret_size.into())), // Ret size missing
        Operation::Push((1_u8, ret_offset.into())), // Ret offset
        Operation::Push((1_u8, args_size.into())),  // Args size
        Operation::Push((1_u8, args_offset.into())), // Args offset
        Operation::Push((20_u8, BigUint::from_bytes_be(callee_address.as_bytes()))), // Address
        Operation::Push((32_u8, gas.into())),       // Gas
        Operation::StaticCall,
    ];

    let program = Program::from(caller_ops);
    let caller_bytecode = Bytecode::from(program.to_bytecode());
    let mut env = Env::default();
    let db = Db::new().with_contract(caller_address, caller_bytecode);
    env.tx.transact_to = TransactTo::Call(caller_address);

    run_program_assert_halt(env, db);
}

#[test]
fn staticcall_on_precompile_ecmul_with_not_enough_gas() {
    let ret_size: u8 = 64;
    let ret_offset: u8 = 96;
    let args_size: u8 = 96;
    let args_offset: u8 = 0;
    let gas: u32 = 5999;
    let callee_address = Address::from_low_u64_be(ECMUL_ADDRESS);
    let caller_address = Address::from_low_u64_be(4040);

    let x1: u8 = 1;
    let y1: u8 = 2;
    let s: u8 = 2;

    let mut jumpdest: u8 = (33 * 3) + (3 * 3); // parameters store
    jumpdest += (2 * 4) + 21 + 33 + 1; // call
    jumpdest += 1 + 2 + 1 + (2 * 2) + 1; // check and return

    let caller_ops = vec![
        // Store the parameters in memory
        Operation::Push((32_u8, x1.into())),
        Operation::Push((1_u8, 0_u8.into())),
        Operation::Mstore,
        Operation::Push((32_u8, y1.into())),
        Operation::Push((1_u8, 0x20_u8.into())),
        Operation::Mstore,
        Operation::Push((32_u8, s.into())),
        Operation::Push((1_u8, 0x40_u8.into())),
        Operation::Mstore,
        // Do the call
        Operation::Push((1_u8, ret_size.into())), // Ret size
        Operation::Push((1_u8, ret_offset.into())), // Ret offset
        Operation::Push((1_u8, args_size.into())), // Args size
        Operation::Push((1_u8, args_offset.into())), // Args offset
        Operation::Push((20_u8, BigUint::from_bytes_be(callee_address.as_bytes()))), // Address
        Operation::Push((32_u8, gas.into())),     // Gas
        Operation::StaticCall,
        // Check if STATICCALL returned 0 (failure)
        Operation::IsZero,
        Operation::Push((1_u8, jumpdest.into())), // Push the location of revert
        Operation::Jumpi,
        // Continue execution if STATICCALL returned 1 (shouldn't happen)
        Operation::Push((1_u8, ret_size.into())), // Ret size
        Operation::Push((1_u8, ret_offset.into())), // Ret offset
        Operation::Return,
        // Revert
        Operation::Jumpdest {
            pc: jumpdest as usize,
        },
        Operation::Push0, // Ret size
        Operation::Push0, // Ret offset
        Operation::Revert,
    ];

    let program = Program::from(caller_ops);
    let caller_bytecode = Bytecode::from(program.to_bytecode());
    let mut env = Env::default();
    let db = Db::new().with_contract(caller_address, caller_bytecode);
    env.tx.transact_to = TransactTo::Call(caller_address);

    run_program_assert_revert(env, db);
}

#[test]
fn staticcall_on_precompile_ecpairing_happy_path() {
    let ret_size: u16 = 32;
    let ret_offset: u16 = 128;
    let args_size: u16 = 384;
    let args_offset: u16 = 0;
    let gas_limit: u32 = 100_000_000;
    let callee_address = Address::from_low_u64_be(ECPAIRING_ADDRESS);
    let caller_address = Address::from_low_u64_be(4040);

    let calldata = Bytes::from(
        hex::decode(
            "\
        2cf44499d5d27bb186308b7af7af02ac5bc9eeb6a3d147c186b21fb1b76e18da\
        2c0f001f52110ccfe69108924926e45f0b0c868df0e7bde1fe16d3242dc715f6\
        1fb19bb476f6b9e44e2a32234da8212f61cd63919354bc06aef31e3cfaff3ebc\
        22606845ff186793914e03e21df544c34ffe2f2f3504de8a79d9159eca2d98d9\
        2bd368e28381e8eccb5fa81fc26cf3f048eea9abfdd85d7ed3ab3698d63e4f90\
        2fe02e47887507adf0ff1743cbac6ba291e66f59be6bd763950bb16041a0a85e\
        0000000000000000000000000000000000000000000000000000000000000001\
        30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd45\
        1971ff0471b09fa93caaf13cbf443c1aede09cc4328f5a62aad45f40ec133eb4\
        091058a3141822985733cbdddfed0fd8d6c104e9e9eff40bf5abfef9ab163bc7\
        2a23af9a5ce2ba2796c1f4e453a370eb0af8c212d9dc9acd8fc02c2e907baea2\
        23a8eb0b0996252cb548a4487da97b02422ebc0e834613f954de6c7e0afdc1fc",
        )
        .unwrap(),
    );

    let expected_result =
        hex::decode("0000000000000000000000000000000000000000000000000000000000000001").unwrap();

    let mut caller_ops = vec![];

    // Store the entire calldata in memory (384 bytes, broken into 12 chunks of 32 bytes)
    for i in 0..12 {
        caller_ops.push(Operation::Push((
            32_u8,
            BigUint::from_bytes_be(&calldata[i * 32..(i + 1) * 32]),
        )));
        caller_ops.push(Operation::Push((2_u8, ((i * 32) as u16).into()))); // Adjusted for u16 offset
        caller_ops.push(Operation::Mstore);
    }

    caller_ops.extend(vec![
        // Do the call
        Operation::Push((2_u8, ret_size.into())), // Ret size
        Operation::Push((2_u8, ret_offset.into())), // Ret offset
        Operation::Push((2_u8, args_size.into())), // Args size
        Operation::Push((2_u8, args_offset.into())), // Args offset
        Operation::Push((20_u8, BigUint::from_bytes_be(callee_address.as_bytes()))), // Address
        Operation::Push((32_u8, gas_limit.into())), // Gas
        Operation::StaticCall,
        // Return
        Operation::Push((2_u8, ret_size.into())),
        Operation::Push((2_u8, ret_offset.into())),
        Operation::Return,
    ]);

    let program = Program::from(caller_ops);
    let caller_bytecode = Bytecode::from(program.to_bytecode());
    let mut env = Env::default();
    let db = Db::new().with_contract(caller_address, caller_bytecode);
    env.tx.transact_to = TransactTo::Call(caller_address);

    run_program_assert_bytes_result(env, db, &expected_result);
}

#[test]
fn staticcall_on_precompile_ecpairing_p1_is_infinity() {
    let ret_size: u8 = 32;
    let ret_offset: u8 = 128;
    let args_size: u8 = 192;
    let args_offset: u8 = 0;
    let gas_limit: u32 = 100_000_000;
    let callee_address = Address::from_low_u64_be(ECPAIRING_ADDRESS);
    let caller_address = Address::from_low_u64_be(4040);

    let calldata = Bytes::from(
        hex::decode(
            "\
            0000000000000000000000000000000000000000000000000000000000000000\
            0000000000000000000000000000000000000000000000000000000000000000\
            1fb19bb476f6b9e44e2a32234da8212f61cd63919354bc06aef31e3cfaff3ebc\
            22606845ff186793914e03e21df544c34ffe2f2f3504de8a79d9159eca2d98d9\
            2bd368e28381e8eccb5fa81fc26cf3f048eea9abfdd85d7ed3ab3698d63e4f90\
            2fe02e47887507adf0ff1743cbac6ba291e66f59be6bd763950bb16041a0a85e",
        )
        .unwrap(),
    );

    let expected_result =
        hex::decode("0000000000000000000000000000000000000000000000000000000000000001").unwrap();

    let mut caller_ops = vec![];

    // Store the entire calldata in memory (192 bytes, broken into 6 chunks of 32 bytes)
    for i in 0..6 {
        caller_ops.push(Operation::Push((
            32_u8,
            BigUint::from_bytes_be(&calldata[i * 32..(i + 1) * 32]),
        )));
        caller_ops.push(Operation::Push((1_u8, ((i * 32) as u8).into())));
        caller_ops.push(Operation::Mstore);
    }

    caller_ops.extend(vec![
        // Do the call
        Operation::Push((1_u8, ret_size.into())), // Ret size
        Operation::Push((1_u8, ret_offset.into())), // Ret offset
        Operation::Push((1_u8, args_size.into())), // Args size
        Operation::Push((1_u8, args_offset.into())), // Args offset
        Operation::Push((20_u8, BigUint::from_bytes_be(callee_address.as_bytes()))), // Address
        Operation::Push((32_u8, gas_limit.into())), // Gas
        Operation::StaticCall,
        // Return
        Operation::Push((1_u8, ret_size.into())),
        Operation::Push((1_u8, ret_offset.into())),
        Operation::Return,
    ]);

    let program = Program::from(caller_ops);
    let caller_bytecode = Bytecode::from(program.to_bytecode());
    let mut env = Env::default();
    let db = Db::new().with_contract(caller_address, caller_bytecode);
    env.tx.transact_to = TransactTo::Call(caller_address);

    run_program_assert_bytes_result(env, db, &expected_result);
}

#[test]
fn staticcall_on_precompile_ecpairing_p2_is_infinity() {
    let ret_size: u8 = 32;
    let ret_offset: u8 = 128;
    let args_size: u8 = 192;
    let args_offset: u8 = 0;
    let gas_limit: u32 = 100_000_000;
    let callee_address = Address::from_low_u64_be(ECPAIRING_ADDRESS);
    let caller_address = Address::from_low_u64_be(4040);

    let calldata = Bytes::from(
        hex::decode(
            "\
            2cf44499d5d27bb186308b7af7af02ac5bc9eeb6a3d147c186b21fb1b76e18da\
            2c0f001f52110ccfe69108924926e45f0b0c868df0e7bde1fe16d3242dc715f6\
            0000000000000000000000000000000000000000000000000000000000000000\
            0000000000000000000000000000000000000000000000000000000000000000\
            0000000000000000000000000000000000000000000000000000000000000000\
            0000000000000000000000000000000000000000000000000000000000000000",
        )
        .unwrap(),
    );

    let expected_result =
        hex::decode("0000000000000000000000000000000000000000000000000000000000000001").unwrap();

    let mut caller_ops = vec![];

    // Store the entire calldata in memory (192 bytes, broken into 6 chunks of 32 bytes)
    for i in 0..6 {
        caller_ops.push(Operation::Push((
            32_u8,
            BigUint::from_bytes_be(&calldata[i * 32..(i + 1) * 32]),
        )));
        caller_ops.push(Operation::Push((1_u8, ((i * 32) as u8).into())));
        caller_ops.push(Operation::Mstore);
    }

    caller_ops.extend(vec![
        // Do the call
        Operation::Push((1_u8, ret_size.into())), // Ret size
        Operation::Push((1_u8, ret_offset.into())), // Ret offset
        Operation::Push((1_u8, args_size.into())), // Args size
        Operation::Push((1_u8, args_offset.into())), // Args offset
        Operation::Push((20_u8, BigUint::from_bytes_be(callee_address.as_bytes()))), // Address
        Operation::Push((32_u8, gas_limit.into())), // Gas
        Operation::StaticCall,
        // Return
        Operation::Push((1_u8, ret_size.into())),
        Operation::Push((1_u8, ret_offset.into())),
        Operation::Return,
    ]);

    let program = Program::from(caller_ops);
    let caller_bytecode = Bytecode::from(program.to_bytecode());
    let mut env = Env::default();
    let db = Db::new().with_contract(caller_address, caller_bytecode);
    env.tx.transact_to = TransactTo::Call(caller_address);

    run_program_assert_bytes_result(env, db, &expected_result);
}

#[test]
fn staticcall_on_precompile_ecpairing_empty_calldata() {
    let ret_size: u8 = 32;
    let ret_offset: u8 = 128;
    let args_size: u8 = 0;
    let args_offset: u8 = 0;
    let gas_limit: u32 = 100_000_000;
    let callee_address = Address::from_low_u64_be(ECPAIRING_ADDRESS);
    let caller_address = Address::from_low_u64_be(4040);

    let expected_result =
        hex::decode("0000000000000000000000000000000000000000000000000000000000000001").unwrap();

    let mut caller_ops = vec![];

    caller_ops.extend(vec![
        // Do the call
        Operation::Push((1_u8, ret_size.into())), // Ret size
        Operation::Push((1_u8, ret_offset.into())), // Ret offset
        Operation::Push((1_u8, args_size.into())), // Args size
        Operation::Push((1_u8, args_offset.into())), // Args offset
        Operation::Push((20_u8, BigUint::from_bytes_be(callee_address.as_bytes()))), // Address
        Operation::Push((32_u8, gas_limit.into())), // Gas
        Operation::StaticCall,
        // Return
        Operation::Push((1_u8, ret_size.into())),
        Operation::Push((1_u8, ret_offset.into())),
        Operation::Return,
    ]);

    let program = Program::from(caller_ops);
    let caller_bytecode = Bytecode::from(program.to_bytecode());
    let mut env = Env::default();
    let db = Db::new().with_contract(caller_address, caller_bytecode);
    env.tx.transact_to = TransactTo::Call(caller_address);

    run_program_assert_bytes_result(env, db, &expected_result);
}

#[test]
fn staticcall_on_precompile_ecpairing_invalid_point() {
    let ret_size: u16 = 32;
    let ret_offset: u16 = 128;
    let args_size: u16 = 384;
    let args_offset: u16 = 0;
    let gas_limit: u32 = 100_000_000;
    let callee_address = Address::from_low_u64_be(ECPAIRING_ADDRESS);
    let caller_address = Address::from_low_u64_be(4040);

    // changed last byte from `fc` to `fd`
    let calldata = Bytes::from(
        hex::decode(
            "\
        2cf44499d5d27bb186308b7af7af02ac5bc9eeb6a3d147c186b21fb1b76e18da\
        2c0f001f52110ccfe69108924926e45f0b0c868df0e7bde1fe16d3242dc715f6\
        1fb19bb476f6b9e44e2a32234da8212f61cd63919354bc06aef31e3cfaff3ebc\
        22606845ff186793914e03e21df544c34ffe2f2f3504de8a79d9159eca2d98d9\
        2bd368e28381e8eccb5fa81fc26cf3f048eea9abfdd85d7ed3ab3698d63e4f90\
        2fe02e47887507adf0ff1743cbac6ba291e66f59be6bd763950bb16041a0a85e\
        0000000000000000000000000000000000000000000000000000000000000001\
        30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd45\
        1971ff0471b09fa93caaf13cbf443c1aede09cc4328f5a62aad45f40ec133eb4\
        091058a3141822985733cbdddfed0fd8d6c104e9e9eff40bf5abfef9ab163bc7\
        2a23af9a5ce2ba2796c1f4e453a370eb0af8c212d9dc9acd8fc02c2e907baea2\
        23a8eb0b0996252cb548a4487da97b02422ebc0e834613f954de6c7e0afdc1fd",
        )
        .unwrap(),
    );

    let mut jumpdest: u16 = (33 + 3 + 1) * 12; // operations inside for
    jumpdest += (3 * 4) + 21 + 33 + 1 + 1 + 3 + 1 + (2 * 2) + 1;

    let mut caller_ops = vec![];
    // Store the entire calldata in memory (384 bytes, broken into 12 chunks of 32 bytes)
    for i in 0..12 {
        caller_ops.push(Operation::Push((
            32_u8,
            BigUint::from_bytes_be(&calldata[i * 32..(i + 1) * 32]),
        )));
        caller_ops.push(Operation::Push((2_u8, ((i * 32) as u16).into()))); // Adjusted for u16 offset
        caller_ops.push(Operation::Mstore);
    }

    caller_ops.extend(vec![
        // Do the call
        Operation::Push((2_u8, ret_size.into())), // Ret size
        Operation::Push((2_u8, ret_offset.into())), // Ret offset
        Operation::Push((2_u8, args_size.into())), // Args size
        Operation::Push((2_u8, args_offset.into())), // Args offset
        Operation::Push((20_u8, BigUint::from_bytes_be(callee_address.as_bytes()))), // Address
        Operation::Push((32_u8, gas_limit.into())), // Gas
        Operation::StaticCall,
        // Check if STATICCALL returned 0 (failure)
        Operation::IsZero,
        Operation::Push((2_u8, jumpdest.into())), // Push the location of revert
        Operation::Jumpi,
        // Continue execution if STATICCALL returned 1 (shouldn't happen)
        Operation::Push((1_u8, ret_size.into())), // Ret size
        Operation::Push((1_u8, ret_offset.into())), // Ret offset
        Operation::Return,
        // Revert
        Operation::Jumpdest {
            pc: jumpdest as usize,
        },
        Operation::Push0, // Ret size
        Operation::Push0, // Ret offset
        Operation::Revert,
    ]);

    let program = Program::from(caller_ops);
    let caller_bytecode = Bytecode::from(program.to_bytecode());
    let mut env = Env::default();
    let db = Db::new().with_contract(caller_address, caller_bytecode);
    env.tx.transact_to = TransactTo::Call(caller_address);

    run_program_assert_revert(env, db);
}

#[test]
fn staticcall_on_precompile_ecpairing_invalid_calldata() {
    let ret_size: u16 = 32;
    let ret_offset: u16 = 128;
    let args_size: u16 = 384;
    let args_offset: u16 = 0;
    let gas_limit: u32 = 100_000_000;
    let callee_address = Address::from_low_u64_be(ECPAIRING_ADDRESS);
    let caller_address = Address::from_low_u64_be(4040);

    // deleted last value
    let calldata = Bytes::from(
        hex::decode(
            "\
        2cf44499d5d27bb186308b7af7af02ac5bc9eeb6a3d147c186b21fb1b76e18da\
        2c0f001f52110ccfe69108924926e45f0b0c868df0e7bde1fe16d3242dc715f6\
        1fb19bb476f6b9e44e2a32234da8212f61cd63919354bc06aef31e3cfaff3ebc\
        22606845ff186793914e03e21df544c34ffe2f2f3504de8a79d9159eca2d98d9\
        2bd368e28381e8eccb5fa81fc26cf3f048eea9abfdd85d7ed3ab3698d63e4f90\
        2fe02e47887507adf0ff1743cbac6ba291e66f59be6bd763950bb16041a0a85e\
        0000000000000000000000000000000000000000000000000000000000000001\
        30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd45\
        1971ff0471b09fa93caaf13cbf443c1aede09cc4328f5a62aad45f40ec133eb4\
        091058a3141822985733cbdddfed0fd8d6c104e9e9eff40bf5abfef9ab163bc7\
        2a23af9a5ce2ba2796c1f4e453a370eb0af8c212d9dc9acd8fc02c2e907baea2",
        )
        .unwrap(),
    );

    let mut jumpdest: u16 = (33 + 3 + 1) * 11; // operations inside for
    jumpdest += (3 * 4) + 21 + 33 + 1 + 1 + 3 + 1 + (2 * 2) + 1;

    let mut caller_ops = vec![];
    // Store the incomplete calldata in memory (352 bytes, broken into 11 chunks of 32 bytes)
    for i in 0..11 {
        caller_ops.push(Operation::Push((
            32_u8,
            BigUint::from_bytes_be(&calldata[i * 32..(i + 1) * 32]),
        )));
        caller_ops.push(Operation::Push((2_u8, ((i * 32) as u16).into()))); // Adjusted for u16 offset
        caller_ops.push(Operation::Mstore);
    }

    caller_ops.extend(vec![
        // Do the call
        Operation::Push((2_u8, ret_size.into())), // Ret size
        Operation::Push((2_u8, ret_offset.into())), // Ret offset
        Operation::Push((2_u8, args_size.into())), // Args size
        Operation::Push((2_u8, args_offset.into())), // Args offset
        Operation::Push((20_u8, BigUint::from_bytes_be(callee_address.as_bytes()))), // Address
        Operation::Push((32_u8, gas_limit.into())), // Gas
        Operation::StaticCall,
        // Check if STATICCALL returned 0 (failure)
        Operation::IsZero,
        Operation::Push((2_u8, jumpdest.into())), // Push the location of revert
        Operation::Jumpi,
        // Continue execution if STATICCALL returned 1 (shouldn't happen)
        Operation::Push((1_u8, ret_size.into())), // Ret size
        Operation::Push((1_u8, ret_offset.into())), // Ret offset
        Operation::Return,
        // Revert
        Operation::Jumpdest {
            pc: jumpdest as usize,
        },
        Operation::Push0, // Ret size
        Operation::Push0, // Ret offset
        Operation::Revert,
    ]);

    let program = Program::from(caller_ops);
    let caller_bytecode = Bytecode::from(program.to_bytecode());
    let mut env = Env::default();
    let db = Db::new().with_contract(caller_address, caller_bytecode);
    env.tx.transact_to = TransactTo::Call(caller_address);

    run_program_assert_revert(env, db);
}

#[test]
fn staticcall_on_precompile_ecpairing_with_not_enough_gas() {
    let ret_size: u16 = 32;
    let ret_offset: u16 = 128;
    let args_size: u16 = 384;
    let args_offset: u16 = 0;
    // needs 113_000
    let gas_limit: u32 = 100_000;
    let callee_address = Address::from_low_u64_be(ECPAIRING_ADDRESS);
    let caller_address = Address::from_low_u64_be(4040);

    let calldata = Bytes::from(
        hex::decode(
            "\
        2cf44499d5d27bb186308b7af7af02ac5bc9eeb6a3d147c186b21fb1b76e18da\
        2c0f001f52110ccfe69108924926e45f0b0c868df0e7bde1fe16d3242dc715f6\
        1fb19bb476f6b9e44e2a32234da8212f61cd63919354bc06aef31e3cfaff3ebc\
        22606845ff186793914e03e21df544c34ffe2f2f3504de8a79d9159eca2d98d9\
        2bd368e28381e8eccb5fa81fc26cf3f048eea9abfdd85d7ed3ab3698d63e4f90\
        2fe02e47887507adf0ff1743cbac6ba291e66f59be6bd763950bb16041a0a85e\
        0000000000000000000000000000000000000000000000000000000000000001\
        30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd45\
        1971ff0471b09fa93caaf13cbf443c1aede09cc4328f5a62aad45f40ec133eb4\
        091058a3141822985733cbdddfed0fd8d6c104e9e9eff40bf5abfef9ab163bc7\
        2a23af9a5ce2ba2796c1f4e453a370eb0af8c212d9dc9acd8fc02c2e907baea2\
        23a8eb0b0996252cb548a4487da97b02422ebc0e834613f954de6c7e0afdc1fc",
        )
        .unwrap(),
    );

    let mut jumpdest: u16 = (33 + 3 + 1) * 12; // operations inside for
    jumpdest += (3 * 4) + 21 + 33 + 1 + 1 + 3 + 1 + (2 * 2) + 1;

    let mut caller_ops = vec![];
    // Store the entire calldata in memory (384 bytes, broken into 12 chunks of 32 bytes)
    for i in 0..12 {
        caller_ops.push(Operation::Push((
            32_u8,
            BigUint::from_bytes_be(&calldata[i * 32..(i + 1) * 32]),
        )));
        caller_ops.push(Operation::Push((2_u8, ((i * 32) as u16).into()))); // Adjusted for u16 offset
        caller_ops.push(Operation::Mstore);
    }

    caller_ops.extend(vec![
        // Do the call
        Operation::Push((2_u8, ret_size.into())), // Ret size
        Operation::Push((2_u8, ret_offset.into())), // Ret offset
        Operation::Push((2_u8, args_size.into())), // Args size
        Operation::Push((2_u8, args_offset.into())), // Args offset
        Operation::Push((20_u8, BigUint::from_bytes_be(callee_address.as_bytes()))), // Address
        Operation::Push((32_u8, gas_limit.into())), // Gas
        Operation::StaticCall,
        // Check if STATICCALL returned 0 (failure)
        Operation::IsZero,
        Operation::Push((2_u8, jumpdest.into())), // Push the location of revert
        Operation::Jumpi,
        // Continue execution if STATICCALL returned 1 (shouldn't happen)
        Operation::Push((1_u8, ret_size.into())), // Ret size
        Operation::Push((1_u8, ret_offset.into())), // Ret offset
        Operation::Return,
        // Revert
        Operation::Jumpdest {
            pc: jumpdest as usize,
        },
        Operation::Push0, // Ret size
        Operation::Push0, // Ret offset
        Operation::Revert,
    ]);

    let program = Program::from(caller_ops);
    let caller_bytecode = Bytecode::from(program.to_bytecode());
    let mut env = Env::default();
    let db = Db::new().with_contract(caller_address, caller_bytecode);
    env.tx.transact_to = TransactTo::Call(caller_address);

    run_program_assert_revert(env, db);
}

#[test]
fn staticcall_on_precompile_blake2f_happy_path() {
    let gas = 100_000_000_u32;
    let args_offset = 0_u8;
    let args_size = 213_u8;
    let ret_offset = 0_u8;
    let ret_size = 64_u8;

    // 4 bytes
    let rounds = hex::decode("0000000c").unwrap();
    // 64 bytes
    let h = hex::decode("48c9bdf267e6096a3ba7ca8485ae67bb2bf894fe72f36e3cf1361d5f3af54fa5d182e6ad7f520e511f6c3e2b8c68059b6bbd41fbabd9831f79217e1319cde05b").unwrap();
    // 128 bytes
    let m = hex::decode("6162630000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000").unwrap();
    // 16 bytes
    let t = hex::decode("03000000000000000000000000000000").unwrap();
    // 1 bytes
    let f = hex::decode("01").unwrap();

    // Reach 32 bytes multiple
    let padding = vec![0_u8; 11];

    let calldata = [rounds, h, m, t, f, padding].concat();

    let callee_address = Address::from_low_u64_be(BLAKE2F_ADDRESS);
    let caller_address = Address::from_low_u64_be(4040);

    let expected_result = hex::decode(
        "ba80a53f981c4d0d6a2797b69f12f6e94c212f14685ac4b74b12bb6fdbffa2d17d87c5392aab792dc252d5de4533cc9518d38aa8dbf1925ab92386edd4009923"
    ).unwrap();

    let caller_ops = vec![
        // Place the parameters in memory
        // rounds - 4 bytes
        Operation::Push((32_u8, BigUint::from_bytes_be(&calldata[..32]))),
        Operation::Push((32_u8, 0_u8.into())),
        Operation::Mstore,
        Operation::Push((32_u8, BigUint::from_bytes_be(&calldata[32..64]))),
        Operation::Push((32_u8, 32_u8.into())),
        Operation::Mstore,
        Operation::Push((32_u8, BigUint::from_bytes_be(&calldata[64..96]))),
        Operation::Push((32_u8, 64_u8.into())),
        Operation::Mstore,
        Operation::Push((32_u8, BigUint::from_bytes_be(&calldata[96..128]))),
        Operation::Push((32_u8, 96_u8.into())),
        Operation::Mstore,
        Operation::Push((32_u8, BigUint::from_bytes_be(&calldata[128..160]))),
        Operation::Push((32_u8, 128_u8.into())),
        Operation::Mstore,
        Operation::Push((32_u8, BigUint::from_bytes_be(&calldata[160..192]))),
        Operation::Push((32_u8, 160_u8.into())),
        Operation::Mstore,
        Operation::Push((32_u8, BigUint::from_bytes_be(&calldata[192..224]))),
        Operation::Push((32_u8, 192_u8.into())),
        Operation::Mstore,
        // Do the call
        Operation::Push((1_u8, BigUint::from(ret_size))), //Ret size
        Operation::Push((1_u8, BigUint::from(ret_offset))), //Ret offset
        Operation::Push((1_u8, BigUint::from(args_size))), //Args size
        Operation::Push((1_u8, BigUint::from(args_offset))), //Args offset
        Operation::Push((20_u8, BigUint::from_bytes_be(callee_address.as_bytes()))), //Address
        Operation::Push((32_u8, BigUint::from(gas))),     //Gas
        Operation::StaticCall,
        // Return
        Operation::Push((1_u8, ret_size.into())),
        Operation::Push((1_u8, ret_offset.into())),
        Operation::Return,
    ];

    let program = Program::from(caller_ops);
    let caller_bytecode = Bytecode::from(program.to_bytecode());
    let mut env = Env::default();
    let db = Db::new().with_contract(caller_address, caller_bytecode);
    env.tx.transact_to = TransactTo::Call(caller_address);

    run_program_assert_bytes_result(env, db, &expected_result);
}
