use melior::{
    dialect::{
        arith::{self, CmpiPredicate},
        cf,
        llvm::{self, r#type::pointer, AllocaOptions, LoadStoreOptions},
    },
    ir::{
        attribute::{IntegerAttribute, TypeAttribute},
        r#type::IntegerType,
        Block, Location, Region, Value,
    },
};

use crate::{
    codegen::context::OperationCtx,
    constants::{MEMORY_PTR_GLOBAL, MEMORY_SIZE_GLOBAL},
    errors::CodegenError,
    utils::{
        compare_values,
        gas::{consume_gas, consume_gas_as_value},
        round_up_32,
    },
};

use super::llvm_mlir;

pub(crate) fn compute_memory_cost<'c>(
    op_ctx: &'c OperationCtx,
    block: &'c Block,
    memory_byte_size: Value<'c, 'c>,
) -> Result<Value<'c, 'c>, CodegenError> {
    // this function computes memory cost, which is given by the following equations
    // memory_size_word = (memory_byte_size + 31) / 32
    // memory_cost = (memory_size_word ** 2) / 512 + (3 * memory_size_word)
    //
    //
    let context = op_ctx.mlir_context;
    let location = Location::unknown(context);
    let uint64 = IntegerType::new(context, 64).into();

    let memory_size_extended = block
        .append_operation(arith::extui(memory_byte_size, uint64, location))
        .result(0)?
        .into();

    let constant_31 = block
        .append_operation(arith::constant(
            context,
            IntegerAttribute::new(uint64, 31).into(),
            location,
        ))
        .result(0)?
        .into();

    let constant_512 = block
        .append_operation(arith::constant(
            context,
            IntegerAttribute::new(uint64, 512).into(),
            location,
        ))
        .result(0)?
        .into();

    let constant_32 = block
        .append_operation(arith::constant(
            context,
            IntegerAttribute::new(uint64, 32).into(),
            location,
        ))
        .result(0)?
        .into();

    let constant_3 = block
        .append_operation(arith::constant(
            context,
            IntegerAttribute::new(uint64, 3).into(),
            location,
        ))
        .result(0)?
        .into();

    let memory_byte_size_plus_31 = block
        .append_operation(arith::addi(memory_size_extended, constant_31, location))
        .result(0)?
        .into();

    let memory_size_word = block
        .append_operation(arith::divui(
            memory_byte_size_plus_31,
            constant_32,
            location,
        ))
        .result(0)?
        .into();

    let memory_size_word_squared = block
        .append_operation(arith::muli(memory_size_word, memory_size_word, location))
        .result(0)?
        .into();

    let memory_size_word_squared_divided_by_512 = block
        .append_operation(arith::divui(
            memory_size_word_squared,
            constant_512,
            location,
        ))
        .result(0)?
        .into();

    let memory_size_word_times_3 = block
        .append_operation(arith::muli(memory_size_word, constant_3, location))
        .result(0)?
        .into();

    let memory_cost = block
        .append_operation(arith::addi(
            memory_size_word_squared_divided_by_512,
            memory_size_word_times_3,
            location,
        ))
        .result(0)?
        .into();

    Ok(memory_cost)
}

pub(crate) fn compute_copy_cost<'c>(
    op_ctx: &'c OperationCtx,
    block: &'c Block,
    memory_byte_size: Value<'c, 'c>,
) -> Result<Value<'c, 'c>, CodegenError> {
    // this function computes memory copying cost (excluding expansion), which is given by the following equations
    // memory_size_word = (memory_byte_size + 31) / 32
    // memory_cost = 3 * memory_size_word
    //
    //
    let context = op_ctx.mlir_context;
    let location = Location::unknown(context);
    let uint64 = IntegerType::new(context, 64).into();

    let memory_size_extended = block
        .append_operation(arith::extui(memory_byte_size, uint64, location))
        .result(0)?
        .into();

    let constant_3 = block
        .append_operation(arith::constant(
            context,
            IntegerAttribute::new(uint64, 3).into(),
            location,
        ))
        .result(0)?
        .into();

    let constant_31 = block
        .append_operation(arith::constant(
            context,
            IntegerAttribute::new(uint64, 31).into(),
            location,
        ))
        .result(0)?
        .into();

    let constant_32 = block
        .append_operation(arith::constant(
            context,
            IntegerAttribute::new(uint64, 32).into(),
            location,
        ))
        .result(0)?
        .into();

    let memory_byte_size_plus_31 = block
        .append_operation(arith::addi(memory_size_extended, constant_31, location))
        .result(0)?
        .into();

    let memory_size_word = block
        .append_operation(arith::divui(
            memory_byte_size_plus_31,
            constant_32,
            location,
        ))
        .result(0)?
        .into();

    let memory_cost = block
        .append_operation(arith::muli(memory_size_word, constant_3, location))
        .result(0)?
        .into();

    Ok(memory_cost)
}

/// Wrapper for calling the [`extend_memory`](crate::syscall::SyscallContext::extend_memory) syscall.
/// Extends memory only if the current memory size is less than the required size, consuming the corresponding gas.
pub(crate) fn extend_memory<'c>(
    op_ctx: &'c OperationCtx,
    block: &'c Block,
    finish_block: &'c Block,
    region: &Region<'c>,
    required_size: Value<'c, 'c>,
    fixed_gas: i64,
) -> Result<(), CodegenError> {
    let context = op_ctx.mlir_context;
    let location = Location::unknown(context);
    let ptr_type = pointer(context, 0);
    let uint32 = IntegerType::new(context, 32);
    let uint64 = IntegerType::new(context, 64);

    // Load memory size
    let memory_size_ptr = block
        .append_operation(llvm_mlir::addressof(
            context,
            MEMORY_SIZE_GLOBAL,
            ptr_type,
            location,
        ))
        .result(0)?
        .into();
    let memory_size = block
        .append_operation(llvm::load(
            context,
            memory_size_ptr,
            uint32.into(),
            location,
            LoadStoreOptions::default(),
        ))
        .result(0)?
        .into();

    let rounded_required_size = round_up_32(op_ctx, block, required_size)?;

    // Compare current memory size and required size
    let extension_flag = compare_values(
        context,
        block,
        CmpiPredicate::Ult,
        memory_size,
        rounded_required_size,
    )?;
    let extension_block = region.append_block(Block::new(&[]));
    let no_extension_block = region.append_block(Block::new(&[]));

    block.append_operation(cf::cond_br(
        context,
        extension_flag,
        &extension_block,
        &no_extension_block,
        &[],
        &[],
        location,
    ));

    // Consume gas for memory extension case
    let memory_cost_before = compute_memory_cost(op_ctx, &extension_block, memory_size)?;
    let memory_cost_after = compute_memory_cost(op_ctx, &extension_block, rounded_required_size)?;

    let dynamic_gas_value = extension_block
        .append_operation(arith::subi(memory_cost_after, memory_cost_before, location))
        .result(0)?
        .into();
    let fixed_gas_value = extension_block
        .append_operation(arith::constant(
            context,
            IntegerAttribute::new(uint64.into(), fixed_gas).into(),
            location,
        ))
        .result(0)?
        .into();
    let total_gas = extension_block
        .append_operation(arith::addi(dynamic_gas_value, fixed_gas_value, location))
        .result(0)?
        .into();
    let extension_gas_flag = consume_gas_as_value(context, &extension_block, total_gas)?;

    // Consume gas for no memory extension case
    let no_extension_gas_flag = consume_gas(context, &no_extension_block, fixed_gas)?;

    let memory_ptr =
        op_ctx.extend_memory_syscall(&extension_block, rounded_required_size, location)?;

    // Store new memory size and pointer
    let res = extension_block.append_operation(llvm::store(
        context,
        rounded_required_size,
        memory_size_ptr,
        location,
        LoadStoreOptions::default(),
    ));
    assert!(res.verify());
    let memory_ptr_ptr = extension_block
        .append_operation(llvm_mlir::addressof(
            context,
            MEMORY_PTR_GLOBAL,
            ptr_type,
            location,
        ))
        .result(0)?;
    let res = extension_block.append_operation(llvm::store(
        context,
        memory_ptr,
        memory_ptr_ptr.into(),
        location,
        LoadStoreOptions::default(),
    ));
    assert!(res.verify());

    // Jump to finish block
    extension_block.append_operation(cf::cond_br(
        context,
        extension_gas_flag,
        finish_block,
        &op_ctx.revert_block,
        &[],
        &[],
        location,
    ));

    no_extension_block.append_operation(cf::cond_br(
        context,
        no_extension_gas_flag,
        finish_block,
        &op_ctx.revert_block,
        &[],
        &[],
        location,
    ));

    Ok(())
}

pub(crate) fn get_memory_pointer<'a>(
    op_ctx: &'a OperationCtx<'a>,
    block: &'a Block<'a>,
    location: Location<'a>,
) -> Result<Value<'a, 'a>, CodegenError> {
    let context = op_ctx.mlir_context;
    let ptr_type = pointer(context, 0);

    let memory_ptr_ptr = block
        .append_operation(llvm_mlir::addressof(
            context,
            MEMORY_PTR_GLOBAL,
            ptr_type,
            location,
        ))
        .result(0)?;

    let memory_ptr = block
        .append_operation(llvm::load(
            context,
            memory_ptr_ptr.into(),
            ptr_type,
            location,
            LoadStoreOptions::default(),
        ))
        .result(0)?;

    Ok(memory_ptr.into())
}

/// Allocates memory for a 32-byte value, stores the value in the memory
/// and returns a pointer to the value
pub(crate) fn allocate_and_store_value<'a>(
    op_ctx: &'a OperationCtx<'a>,
    block: &'a Block<'a>,
    value: Value<'a, 'a>,
    location: Location<'a>,
) -> Result<Value<'a, 'a>, CodegenError> {
    let context = op_ctx.mlir_context;
    let ptr_type = pointer(context, 0);
    let uint32 = IntegerType::new(context, 32);
    let uint256 = IntegerType::new(context, 256);

    let number_of_elements = block
        .append_operation(arith::constant(
            context,
            IntegerAttribute::new(uint32.into(), 1).into(),
            location,
        ))
        .result(0)?
        .into();

    let value_ptr = block
        .append_operation(llvm::alloca(
            context,
            number_of_elements,
            ptr_type,
            location,
            AllocaOptions::new().elem_type(TypeAttribute::new(uint256.into()).into()),
        ))
        .result(0)?
        .into();

    block.append_operation(llvm::store(
        context,
        value,
        value_ptr,
        location,
        LoadStoreOptions::default()
            .align(IntegerAttribute::new(IntegerType::new(context, 64).into(), 1).into()),
    ));

    Ok(value_ptr)
}
