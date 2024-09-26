use crate::{codegen::context::OperationCtx, constants::GAS_COUNTER_GLOBAL, errors::CodegenError};
use melior::{
    dialect::{
        arith::{self},
        llvm::{self, r#type::pointer, LoadStoreOptions},
    },
    ir::{attribute::IntegerAttribute, r#type::IntegerType, Block, Location, Value},
    Context as MeliorContext,
};

use super::{llvm_mlir, misc::integer_constant_from_i64};

// NOTE: the value is of type i64
pub fn get_remaining_gas<'ctx>(
    context: &'ctx MeliorContext,
    block: &'ctx Block,
) -> Result<Value<'ctx, 'ctx>, CodegenError> {
    let location = Location::unknown(context);
    let ptr_type = pointer(context, 0);

    // Get address of gas counter global
    let gas_counter_ptr = block
        .append_operation(llvm_mlir::addressof(
            context,
            GAS_COUNTER_GLOBAL,
            ptr_type,
            location,
        ))
        .result(0)?;

    // Load gas counter
    let gas_counter = block
        .append_operation(llvm::load(
            context,
            gas_counter_ptr.into(),
            IntegerType::new(context, 64).into(),
            location,
            LoadStoreOptions::default(),
        ))
        .result(0)?
        .into();

    Ok(gas_counter)
}

/// Returns true if there is enough Gas
pub fn consume_gas<'ctx>(
    context: &'ctx MeliorContext,
    block: &'ctx Block,
    amount: i64,
) -> Result<Value<'ctx, 'ctx>, CodegenError> {
    let location = Location::unknown(context);
    let ptr_type = pointer(context, 0);
    let uint64 = IntegerType::new(context, 64).into();

    // Get address of gas counter global
    let gas_counter_ptr = block
        .append_operation(llvm_mlir::addressof(
            context,
            GAS_COUNTER_GLOBAL,
            ptr_type,
            location,
        ))
        .result(0)?;

    // Load gas counter
    let gas_counter = block
        .append_operation(llvm::load(
            context,
            gas_counter_ptr.into(),
            uint64,
            location,
            LoadStoreOptions::default(),
        ))
        .result(0)?
        .into();

    let gas_value = block
        .append_operation(arith::constant(
            context,
            IntegerAttribute::new(uint64, amount).into(),
            location,
        ))
        .result(0)?
        .into();

    // Check that gas_counter >= gas_value
    let flag = block
        .append_operation(arith::cmpi(
            context,
            arith::CmpiPredicate::Sge,
            gas_counter,
            gas_value,
            location,
        ))
        .result(0)?;

    // Subtract gas from gas counter
    let new_gas_counter = block
        .append_operation(arith::subi(gas_counter, gas_value, location))
        .result(0)?;

    // Store new gas counter
    let _res = block.append_operation(llvm::store(
        context,
        new_gas_counter.into(),
        gas_counter_ptr.into(),
        location,
        LoadStoreOptions::default(),
    ));

    Ok(flag.into())
}

/// Returns true if there is enough Gas
pub fn consume_gas_as_value<'ctx>(
    context: &'ctx MeliorContext,
    block: &'ctx Block,
    gas_value: Value<'ctx, 'ctx>,
) -> Result<Value<'ctx, 'ctx>, CodegenError> {
    let location = Location::unknown(context);
    let ptr_type = pointer(context, 0);
    let uint64 = IntegerType::new(context, 64).into();

    // Get address of gas counter global
    let gas_counter_ptr = block
        .append_operation(llvm_mlir::addressof(
            context,
            GAS_COUNTER_GLOBAL,
            ptr_type,
            location,
        ))
        .result(0)?;

    // Load gas counter
    let gas_counter = block
        .append_operation(llvm::load(
            context,
            gas_counter_ptr.into(),
            uint64,
            location,
            LoadStoreOptions::default(),
        ))
        .result(0)?
        .into();

    // Check that gas_counter >= gas_value
    let flag = block
        .append_operation(arith::cmpi(
            context,
            arith::CmpiPredicate::Sge,
            gas_counter,
            gas_value,
            location,
        ))
        .result(0)?;

    // Subtract gas from gas counter
    let new_gas_counter = block
        .append_operation(arith::subi(gas_counter, gas_value, location))
        .result(0)?;

    // Store new gas counter
    let _res = block.append_operation(llvm::store(
        context,
        new_gas_counter.into(),
        gas_counter_ptr.into(),
        location,
        LoadStoreOptions::default(),
    ));

    Ok(flag.into())
}

// computes dynamic_gas = 375 * topic_count + 8 * size
pub(crate) fn compute_log_dynamic_gas<'a>(
    op_ctx: &'a OperationCtx<'a>,
    block: &'a Block<'a>,
    nth: u8,
    size: Value<'a, 'a>,
    location: Location<'a>,
) -> Result<Value<'a, 'a>, CodegenError> {
    let context = op_ctx.mlir_context;
    let uint64 = IntegerType::new(context, 64);

    let constant_375 = block
        .append_operation(arith::constant(
            context,
            integer_constant_from_i64(context, 375).into(),
            location,
        ))
        .result(0)?
        .into();

    let constant_8 = block
        .append_operation(arith::constant(
            context,
            integer_constant_from_i64(context, 8).into(),
            location,
        ))
        .result(0)?
        .into();

    let topic_count = block
        .append_operation(arith::constant(
            context,
            integer_constant_from_i64(context, nth as i64).into(),
            location,
        ))
        .result(0)?
        .into();

    let topic_count_x_375 = block
        .append_operation(arith::muli(topic_count, constant_375, location))
        .result(0)?
        .into();
    let size_x_8 = block
        .append_operation(arith::muli(size, constant_8, location))
        .result(0)?
        .into();
    let dynamic_gas = block
        .append_operation(arith::addi(topic_count_x_375, size_x_8, location))
        .result(0)?
        .into();
    let dynamic_gas = block
        .append_operation(arith::trunci(dynamic_gas, uint64.into(), location))
        .result(0)?
        .into();
    Ok(dynamic_gas)
}
