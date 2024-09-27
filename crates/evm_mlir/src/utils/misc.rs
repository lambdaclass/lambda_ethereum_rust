use bytes::{BufMut, Bytes};
use melior::{
    dialect::{
        arith::{self, CmpiPredicate},
        func,
        llvm::{self, r#type::pointer, AllocaOptions, LoadStoreOptions},
        ods,
    },
    ir::{
        attribute::{IntegerAttribute, TypeAttribute},
        r#type::IntegerType,
        Block, BlockRef, Location, Region, Value,
    },
    Context as MeliorContext,
};
use sha3::{Digest, Keccak256};

use crate::{
    codegen::context::OperationCtx,
    constants::{
        gas_cost::{self, TX_ACCESS_LIST_ADDRESS_COST, TX_ACCESS_LIST_STORAGE_KEY_COST},
        precompiles::{
            BLAKE2F_ADDRESS, ECADD_ADDRESS, ECMUL_ADDRESS, ECPAIRING_ADDRESS, ECRECOVER_ADDRESS,
            IDENTITY_ADDRESS, MODEXP_ADDRESS, RIPEMD_160_ADDRESS, SHA2_256_ADDRESS,
        },
        CALLDATA_PTR_GLOBAL, CALLDATA_SIZE_GLOBAL, GAS_COUNTER_GLOBAL,
    },
    env::AccessList,
    errors::CodegenError,
    primitives::{Address, H160, U256},
    syscall::{symbols::CONTEXT_IS_STATIC, ExitStatusCode},
};

use super::{extend_memory, gas::get_remaining_gas, llvm_mlir, stack::stack_pop};

pub(crate) fn check_context_is_not_static<'c>(
    op_ctx: &'c OperationCtx,
    block: &'c Block,
) -> Result<Value<'c, 'c>, CodegenError> {
    let context = &op_ctx.mlir_context;
    let location = Location::unknown(context);
    let uint1 = IntegerType::new(context, 1);

    let is_static = context_is_static(op_ctx, block)?;
    let true_value = block
        .append_operation(arith::constant(
            context,
            IntegerAttribute::new(uint1.into(), 1).into(),
            location,
        ))
        .result(0)?
        .into();

    let is_not_static = block
        .append_operation(arith::xori(is_static, true_value, location))
        .result(0)?
        .into();

    Ok(is_not_static)
}

pub(crate) fn context_is_static<'c>(
    op_ctx: &'c OperationCtx,
    block: &'c Block,
) -> Result<Value<'c, 'c>, CodegenError> {
    let context = &op_ctx.mlir_context;
    let location = Location::unknown(context);
    let uint1 = IntegerType::new(context, 1);
    let ptr_type = pointer(context, 0);
    let static_flag_ptr = block
        .append_operation(llvm_mlir::addressof(
            context,
            CONTEXT_IS_STATIC,
            ptr_type,
            location,
        ))
        .result(0)?
        .into();
    let is_static = block
        .append_operation(llvm::load(
            context,
            static_flag_ptr,
            uint1.into(),
            location,
            LoadStoreOptions::default(),
        ))
        .result(0)?
        .into();

    Ok(is_static)
}

pub fn constant_value_from_i64<'ctx>(
    context: &'ctx MeliorContext,
    block: &'ctx Block,
    value: i64,
) -> Result<Value<'ctx, 'ctx>, CodegenError> {
    let location = Location::unknown(context);

    Ok(block
        .append_operation(arith::constant(
            context,
            integer_constant_from_i64(context, value).into(),
            location,
        ))
        .result(0)?
        .into())
}

pub fn compare_values<'ctx>(
    context: &'ctx MeliorContext,
    block: &'ctx Block,
    predicate: CmpiPredicate,
    lhs: Value<'ctx, 'ctx>,
    rhs: Value<'ctx, 'ctx>,
) -> Result<Value<'ctx, 'ctx>, CodegenError> {
    let location = Location::unknown(context);

    let flag = block
        .append_operation(arith::cmpi(context, predicate, lhs, rhs, location))
        .result(0)?;

    Ok(flag.into())
}

pub fn check_if_zero<'ctx>(
    context: &'ctx MeliorContext,
    block: &'ctx Block,
    value: &'ctx Value,
) -> Result<Value<'ctx, 'ctx>, CodegenError> {
    let location = Location::unknown(context);

    //Load zero value constant
    let zero_constant_value = block
        .append_operation(arith::constant(
            context,
            integer_constant_from_i64(context, 0i64).into(),
            location,
        ))
        .result(0)?
        .into();

    //Perform the comparisson -> value == 0
    let flag = block
        .append_operation(
            ods::llvm::icmp(
                context,
                IntegerType::new(context, 1).into(),
                zero_constant_value,
                *value,
                IntegerAttribute::new(
                    IntegerType::new(context, 64).into(),
                    /* "eq" predicate enum value */ 0,
                )
                .into(),
                location,
            )
            .into(),
        )
        .result(0)?;

    Ok(flag.into())
}

pub(crate) fn round_up_32<'c>(
    op_ctx: &'c OperationCtx,
    block: &'c Block,
    size: Value<'c, 'c>,
) -> Result<Value<'c, 'c>, CodegenError> {
    let context = op_ctx.mlir_context;
    let location = Location::unknown(context);
    let uint32 = IntegerType::new(context, 32).into();

    let constant_31 = block
        .append_operation(arith::constant(
            context,
            IntegerAttribute::new(uint32, 31).into(),
            location,
        ))
        .result(0)?
        .into();

    let constant_32 = block
        .append_operation(arith::constant(
            context,
            IntegerAttribute::new(uint32, 32).into(),
            location,
        ))
        .result(0)?
        .into();

    let size_plus_31 = block
        .append_operation(arith::addi(size, constant_31, location))
        .result(0)?
        .into();

    let memory_size_word = block
        .append_operation(arith::divui(size_plus_31, constant_32, location))
        .result(0)?
        .into();

    let memory_size_bytes = block
        .append_operation(arith::muli(memory_size_word, constant_32, location))
        .result(0)?
        .into();

    Ok(memory_size_bytes)
}

pub(crate) fn get_calldata_ptr<'c>(
    op_ctx: &'c OperationCtx,
    block: &'c Block,
    location: Location<'c>,
) -> Result<Value<'c, 'c>, CodegenError> {
    let context = op_ctx.mlir_context;
    let ptr_type = pointer(context, 0);

    let calldata_ptr_ptr = block
        .append_operation(llvm_mlir::addressof(
            context,
            CALLDATA_PTR_GLOBAL,
            ptr_type,
            location,
        ))
        .result(0)?;

    let calldata_ptr = block
        .append_operation(llvm::load(
            context,
            calldata_ptr_ptr.into(),
            ptr_type,
            location,
            LoadStoreOptions::default(),
        ))
        .result(0)?
        .into();

    Ok(calldata_ptr)
}

pub(crate) fn get_calldata_size<'c>(
    op_ctx: &'c OperationCtx,
    block: &'c Block,
    location: Location<'c>,
) -> Result<Value<'c, 'c>, CodegenError> {
    let context = op_ctx.mlir_context;
    let ptr_type = pointer(context, 0);

    let calldata_size_ptr = block
        .append_operation(llvm_mlir::addressof(
            context,
            CALLDATA_SIZE_GLOBAL,
            ptr_type,
            location,
        ))
        .result(0)?;

    let calldata_size = block
        .append_operation(llvm::load(
            context,
            calldata_size_ptr.into(),
            IntegerType::new(context, 32).into(),
            location,
            LoadStoreOptions::default(),
        ))
        .result(0)?
        .into();

    Ok(calldata_size)
}

pub(crate) fn return_empty_result(
    op_ctx: &OperationCtx,
    block: &Block,
    reason_code: ExitStatusCode,
    location: Location,
) -> Result<(), CodegenError> {
    let context = op_ctx.mlir_context;
    let uint32 = IntegerType::new(context, 32).into();

    let zero_constant = block
        .append_operation(arith::constant(
            context,
            IntegerAttribute::new(uint32, 0).into(),
            location,
        ))
        .result(0)?
        .into();

    return_result_with_offset_and_size(
        op_ctx,
        block,
        zero_constant,
        zero_constant,
        reason_code,
        location,
    )?;

    Ok(())
}

pub(crate) fn return_result_from_stack(
    op_ctx: &OperationCtx,
    region: &Region<'_>,
    block: &Block,
    reason_code: ExitStatusCode,
    location: Location,
) -> Result<(), CodegenError> {
    let context = op_ctx.mlir_context;
    let uint32 = IntegerType::new(context, 32);

    let offset_u256 = stack_pop(context, block)?;
    let size_u256 = stack_pop(context, block)?;

    let offset = block
        .append_operation(arith::trunci(offset_u256, uint32.into(), location))
        .result(0)
        .unwrap()
        .into();

    let size = block
        .append_operation(arith::trunci(size_u256, uint32.into(), location))
        .result(0)
        .unwrap()
        .into();

    let required_size = block
        .append_operation(arith::addi(offset, size, location))
        .result(0)?
        .into();

    let return_block = region.append_block(Block::new(&[]));

    extend_memory(op_ctx, block, &return_block, region, required_size, 0)?;

    return_result_with_offset_and_size(op_ctx, &return_block, offset, size, reason_code, location)?;

    Ok(())
}

pub(crate) fn return_result_with_offset_and_size(
    op_ctx: &OperationCtx,
    block: &Block,
    offset: Value,
    size: Value,
    reason_code: ExitStatusCode,
    location: Location,
) -> Result<(), CodegenError> {
    let context = op_ctx.mlir_context;
    let remaining_gas = get_remaining_gas(context, block)?;

    let reason = block
        .append_operation(arith::constant(
            context,
            integer_constant_from_u8(context, reason_code.to_u8()).into(),
            location,
        ))
        .result(0)?
        .into();

    op_ctx.write_result_syscall(block, offset, size, remaining_gas, reason, location);

    block.append_operation(func::r#return(&[reason], location));
    Ok(())
}

pub(crate) fn get_block_number<'a>(
    op_ctx: &'a OperationCtx<'a>,
    block: &'a Block<'a>,
) -> Result<Value<'a, 'a>, CodegenError> {
    let context = op_ctx.mlir_context;
    let location = Location::unknown(context);
    let ptr_type = pointer(context, 0);
    let pointer_size = constant_value_from_i64(context, block, 1_i64)?;
    let uint256 = IntegerType::new(context, 256);

    let block_number_ptr = block
        .append_operation(llvm::alloca(
            context,
            pointer_size,
            ptr_type,
            location,
            AllocaOptions::new().elem_type(Some(TypeAttribute::new(uint256.into()))),
        ))
        .result(0)?
        .into();

    op_ctx.get_block_number_syscall(block, block_number_ptr, location);

    // get the value from the pointer
    let block_number = block
        .append_operation(llvm::load(
            context,
            block_number_ptr,
            IntegerType::new(context, 256).into(),
            location,
            LoadStoreOptions::default(),
        ))
        .result(0)?
        .into();

    Ok(block_number)
}

pub(crate) fn get_prevrandao<'a>(
    op_ctx: &'a OperationCtx<'a>,
    block: &'a Block<'a>,
) -> Result<Value<'a, 'a>, CodegenError> {
    let context = op_ctx.mlir_context;
    let location = Location::unknown(context);
    let ptr_type = pointer(context, 0);
    let pointer_size = constant_value_from_i64(context, block, 1_i64)?;
    let uint256 = IntegerType::new(context, 256);

    let prevrandao_ptr = block
        .append_operation(llvm::alloca(
            context,
            pointer_size,
            ptr_type,
            location,
            AllocaOptions::new().elem_type(Some(TypeAttribute::new(uint256.into()))),
        ))
        .result(0)?
        .into();

    op_ctx.get_prevrandao_syscall(block, prevrandao_ptr, location);

    // get the value from the pointer
    let prevrandao = block
        .append_operation(llvm::load(
            context,
            prevrandao_ptr,
            IntegerType::new(context, 256).into(),
            location,
            LoadStoreOptions::default(),
        ))
        .result(0)?
        .into();

    Ok(prevrandao)
}

pub(crate) fn get_blob_hash_at_index<'a>(
    op_ctx: &'a OperationCtx<'a>,
    block: &'a Block<'a>,
    index_ptr: Value<'a, 'a>,
) -> Result<Value<'a, 'a>, CodegenError> {
    let context = op_ctx.mlir_context;
    let location = Location::unknown(context);
    let ptr_type = pointer(context, 0);
    let pointer_size = constant_value_from_i64(context, block, 1_i64)?;
    let uint256 = IntegerType::new(context, 256);

    let blobhash_ptr = block
        .append_operation(llvm::alloca(
            context,
            pointer_size,
            ptr_type,
            location,
            AllocaOptions::new().elem_type(Some(TypeAttribute::new(uint256.into()))),
        ))
        .result(0)?
        .into();

    op_ctx.get_blob_hash_at_index_syscall(block, index_ptr, blobhash_ptr, location);

    // get the value from the pointer
    let blobhash = block
        .append_operation(llvm::load(
            context,
            blobhash_ptr,
            IntegerType::new(context, 256).into(),
            location,
            LoadStoreOptions::default(),
        ))
        .result(0)?
        .into();

    Ok(blobhash)
}

pub fn integer_constant_from_i64(context: &MeliorContext, value: i64) -> IntegerAttribute {
    let uint256 = IntegerType::new(context, 256);
    IntegerAttribute::new(uint256.into(), value)
}

pub fn integer_constant_from_u8(context: &MeliorContext, value: u8) -> IntegerAttribute {
    let uint8 = IntegerType::new(context, 8);
    IntegerAttribute::new(uint8.into(), value.into())
}

/// Returns the basefee
pub(crate) fn get_basefee<'a>(
    op_ctx: &'a OperationCtx<'a>,
    block: &'a Block<'a>,
) -> Result<Value<'a, 'a>, CodegenError> {
    let context = op_ctx.mlir_context;
    let location = Location::unknown(context);
    let ptr_type = pointer(context, 0);
    let pointer_size = constant_value_from_i64(context, block, 1_i64)?;
    let uint256 = IntegerType::new(context, 256);

    let basefee_ptr = block
        .append_operation(llvm::alloca(
            context,
            pointer_size,
            ptr_type,
            location,
            AllocaOptions::new().elem_type(Some(TypeAttribute::new(uint256.into()))),
        ))
        .result(0)?
        .into();

    op_ctx.store_in_basefee_ptr_syscall(basefee_ptr, block, location);

    // get the value from the pointer
    let basefee = block
        .append_operation(llvm::load(
            context,
            basefee_ptr,
            IntegerType::new(context, 256).into(),
            location,
            LoadStoreOptions::default(),
        ))
        .result(0)?
        .into();

    Ok(basefee)
}

/// Calculates the blob gas price from the header's excess blob gas field.
///
/// See also [the EIP-4844 helpers](https://eips.ethereum.org/EIPS/eip-4844#helpers)
/// (`get_blob_gasprice`).
pub fn calc_blob_gasprice(excess_blob_gas: u64) -> u128 {
    fake_exponential(
        gas_cost::MIN_BLOB_GASPRICE,
        excess_blob_gas,
        gas_cost::BLOB_GASPRICE_UPDATE_FRACTION,
    )
}

/// Approximates `factor * e ** (numerator / denominator)` using Taylor expansion.
///
/// This is used to calculate the blob price.
///
/// See also [the EIP-4844 helpers](https://eips.ethereum.org/EIPS/eip-4844#helpers)
/// (`fake_exponential`).
///
/// # Panics
///
/// This function panics if `denominator` is zero.
pub fn fake_exponential(factor: u64, numerator: u64, denominator: u64) -> u128 {
    assert_ne!(denominator, 0, "attempt to divide by zero");
    let factor = factor as u128;
    let numerator = numerator as u128;
    let denominator = denominator as u128;

    let mut i = 1;
    let mut output = 0;
    let mut numerator_accum = factor * denominator;
    while numerator_accum > 0 {
        output += numerator_accum;

        // Denominator is asserted as not zero at the start of the function.
        numerator_accum = (numerator_accum * numerator) / (denominator * i);
        i += 1;
    }
    output / denominator
}

pub fn encode_rlp_u64(number: u64) -> Bytes {
    let mut buf: Vec<u8> = vec![];
    match number {
        // 0, also known as null or the empty string is 0x80
        0 => buf.put_u8(0x80),
        // for a single byte whose value is in the [0x00, 0x7f] range, that byte is its own RLP encoding.
        n @ 1..=0x7f => buf.put_u8(n as u8),
        // Otherwise, if a string is 0-55 bytes long, the RLP encoding consists of a
        // single byte with value RLP_NULL (0x80) plus the length of the string followed by the string.
        n => {
            let mut bytes: Vec<u8> = vec![];
            bytes.extend_from_slice(&n.to_be_bytes());
            let start = bytes.iter().position(|&x| x != 0).unwrap();
            let len = bytes.len() - start;
            buf.put_u8(0x80 + len as u8);
            buf.put_slice(&bytes[start..]);
        }
    }
    buf.into()
}

pub fn compute_contract_address(address: H160, nonce: u64) -> Address {
    // Compute the destination address as keccak256(rlp([sender_address,sender_nonce]))[12:]
    // TODO: replace manual encoding once rlp is added
    let encoded_nonce = encode_rlp_u64(nonce);
    let mut buf = Vec::<u8>::new();
    buf.push(0xd5);
    buf.extend_from_slice(&encoded_nonce.len().to_be_bytes());
    buf.push(0x94);
    buf.extend_from_slice(address.as_bytes());
    buf.extend_from_slice(&encoded_nonce);
    let mut hasher = Keccak256::new();
    hasher.update(&buf);
    Address::from_slice(&hasher.finalize()[12..])
}

pub fn compute_contract_address2(address: H160, salt: U256, initialization_code: &[u8]) -> Address {
    // Compute the destination address as keccak256(0xff + sender_address + salt + keccak256(initialisation_code))[12:]
    let mut hasher = Keccak256::new();
    hasher.update(initialization_code);
    let initialization_code_hash = hasher.finalize();

    let mut hasher = Keccak256::new();
    let mut salt_bytes = [0; 32];
    salt.to_big_endian(&mut salt_bytes);
    hasher.update([0xff]);
    hasher.update(address.as_bytes());
    hasher.update(salt_bytes);
    hasher.update(initialization_code_hash);
    Address::from_slice(&hasher.finalize()[12..])
}

pub fn access_list_cost(access_list: &AccessList) -> u64 {
    access_list.iter().fold(0, |acc, (_, keys)| {
        acc + TX_ACCESS_LIST_ADDRESS_COST + keys.len() as u64 * TX_ACCESS_LIST_STORAGE_KEY_COST
    })
}

pub fn precompiled_addresses() -> AccessList {
    let access_list = vec![
        (H160::from_low_u64_be(ECRECOVER_ADDRESS), Vec::new()),
        (H160::from_low_u64_be(SHA2_256_ADDRESS), Vec::new()),
        (H160::from_low_u64_be(RIPEMD_160_ADDRESS), Vec::new()),
        (H160::from_low_u64_be(IDENTITY_ADDRESS), Vec::new()),
        (H160::from_low_u64_be(MODEXP_ADDRESS), Vec::new()),
        (H160::from_low_u64_be(ECADD_ADDRESS), Vec::new()),
        (H160::from_low_u64_be(ECMUL_ADDRESS), Vec::new()),
        (H160::from_low_u64_be(ECPAIRING_ADDRESS), Vec::new()),
        (H160::from_low_u64_be(BLAKE2F_ADDRESS), Vec::new()),
    ];

    access_list
}

pub fn allocate_gas_counter_ptr<'c>(
    context: &&'c melior::Context,
    block: &'c BlockRef<'c, 'c>,
    location: Location<'c>,
) -> Result<melior::ir::Value<'c, 'c>, CodegenError> {
    let uint64 = IntegerType::new(context, 64);
    let uint32 = IntegerType::new(context, 32);

    let ptr_type = pointer(context, 0);

    let gas_counter_ptr = block
        .append_operation(llvm_mlir::addressof(
            context,
            GAS_COUNTER_GLOBAL,
            ptr_type,
            location,
        ))
        .result(0)?
        .into();
    let gas_counter = block
        .append_operation(llvm::load(
            context,
            gas_counter_ptr,
            uint64.into(),
            location,
            LoadStoreOptions::default(),
        ))
        .result(0)?
        .into();
    let number_of_elements = block
        .append_operation(arith::constant(
            context,
            IntegerAttribute::new(uint32.into(), 1).into(),
            location,
        ))
        .result(0)?
        .into();
    let gas_ptr = block
        .append_operation(llvm::alloca(
            context,
            number_of_elements,
            ptr_type,
            location,
            AllocaOptions::new().elem_type(TypeAttribute::new(uint64.into()).into()),
        ))
        .result(0)?
        .into();
    block.append_operation(llvm::store(
        context,
        gas_counter,
        gas_ptr,
        location,
        LoadStoreOptions::default().align(IntegerAttribute::new(uint64.into(), 1).into()),
    ));

    Ok(gas_ptr)
}

// Left pads calldata with zeros until specified length
pub fn left_pad(calldata: &Bytes, target_len: usize) -> Bytes {
    let mut padded_calldata = vec![0u8; target_len];
    if calldata.len() < target_len {
        padded_calldata[target_len - calldata.len()..].copy_from_slice(calldata);
    } else {
        return calldata.clone();
    }
    padded_calldata.into()
}

// Right pads calldata with zeros until specified length
pub fn right_pad(calldata: &Bytes, target_len: usize) -> Bytes {
    let mut padded_calldata = calldata.to_vec();
    if padded_calldata.len() < target_len {
        padded_calldata.extend(vec![0u8; target_len - padded_calldata.len()]);
    }
    padded_calldata.into()
}
