use melior::{
    dialect::{
        llvm::{self, r#type::pointer, LoadStoreOptions},
        ods,
    },
    ir::{
        attribute::{DenseI32ArrayAttribute, IntegerAttribute},
        operation::OperationResult,
        r#type::IntegerType,
        Block, Location, Value, ValueLike,
    },
    Context as MeliorContext,
};

use crate::{
    constants::{MAX_STACK_SIZE, STACK_BASEPTR_GLOBAL, STACK_PTR_GLOBAL},
    errors::CodegenError,
};

use super::llvm_mlir;

pub fn get_stack_pointer<'ctx>(
    context: &'ctx MeliorContext,
    block: &'ctx Block,
) -> Result<Value<'ctx, 'ctx>, CodegenError> {
    let location = Location::unknown(context);
    let ptr_type = pointer(context, 0);

    // Get address of stack pointer global
    let stack_ptr_ptr = block
        .append_operation(llvm_mlir::addressof(
            context,
            STACK_PTR_GLOBAL,
            ptr_type,
            location,
        ))
        .result(0)?;

    // Load stack pointer
    let stack_ptr = block
        .append_operation(llvm::load(
            context,
            stack_ptr_ptr.into(),
            ptr_type,
            location,
            LoadStoreOptions::default(),
        ))
        .result(0)?
        .into();

    Ok(stack_ptr)
}

pub fn inc_stack_pointer<'ctx>(
    context: &'ctx MeliorContext,
    block: &'ctx Block,
) -> Result<(), CodegenError> {
    let location = Location::unknown(context);
    let ptr_type = pointer(context, 0);

    // Get address of stack pointer global
    let stack_ptr_ptr = block
        .append_operation(llvm_mlir::addressof(
            context,
            STACK_PTR_GLOBAL,
            ptr_type,
            location,
        ))
        .result(0)?;

    // Load stack pointer
    let stack_ptr = block
        .append_operation(llvm::load(
            context,
            stack_ptr_ptr.into(),
            ptr_type,
            location,
            LoadStoreOptions::default(),
        ))
        .result(0)?;

    let uint256 = IntegerType::new(context, 256);
    // Increment stack pointer
    let new_stack_ptr = block
        .append_operation(llvm::get_element_ptr(
            context,
            stack_ptr.into(),
            DenseI32ArrayAttribute::new(context, &[1]),
            uint256.into(),
            ptr_type,
            location,
        ))
        .result(0)?;

    // Store incremented stack pointer
    let res = block.append_operation(llvm::store(
        context,
        new_stack_ptr.into(),
        stack_ptr_ptr.into(),
        location,
        LoadStoreOptions::default(),
    ));
    assert!(res.verify());

    Ok(())
}

pub fn stack_pop<'ctx>(
    context: &'ctx MeliorContext,
    block: &'ctx Block,
) -> Result<Value<'ctx, 'ctx>, CodegenError> {
    let uint256 = IntegerType::new(context, 256);
    let location = Location::unknown(context);
    let ptr_type = pointer(context, 0);

    // Get address of stack pointer global
    let stack_ptr_ptr = block
        .append_operation(llvm_mlir::addressof(
            context,
            STACK_PTR_GLOBAL,
            ptr_type,
            location,
        ))
        .result(0)?;

    // Load stack pointer
    let stack_ptr = block
        .append_operation(llvm::load(
            context,
            stack_ptr_ptr.into(),
            ptr_type,
            location,
            LoadStoreOptions::default(),
        ))
        .result(0)?;

    // Decrement stack pointer
    let old_stack_ptr = block
        .append_operation(llvm::get_element_ptr(
            context,
            stack_ptr.into(),
            DenseI32ArrayAttribute::new(context, &[-1]),
            uint256.into(),
            ptr_type,
            location,
        ))
        .result(0)?;

    // Load value from top of stack
    let value = block
        .append_operation(llvm::load(
            context,
            old_stack_ptr.into(),
            uint256.into(),
            location,
            LoadStoreOptions::default(),
        ))
        .result(0)?
        .into();

    // Store decremented stack pointer
    let res = block.append_operation(llvm::store(
        context,
        old_stack_ptr.into(),
        stack_ptr_ptr.into(),
        location,
        LoadStoreOptions::default(),
    ));
    assert!(res.verify());

    Ok(value)
}

pub fn stack_push<'ctx>(
    context: &'ctx MeliorContext,
    block: &'ctx Block,
    value: Value,
) -> Result<(), CodegenError> {
    let location = Location::unknown(context);
    let ptr_type = pointer(context, 0);

    //Check that the value to push is 256 bits wide.
    let uint256 = IntegerType::new(context, 256);
    debug_assert!(value.r#type().eq(&uint256.into()));

    // Get address of stack pointer global
    let stack_ptr_ptr = block
        .append_operation(llvm_mlir::addressof(
            context,
            STACK_PTR_GLOBAL,
            ptr_type,
            location,
        ))
        .result(0)?;

    // Load stack pointer
    let stack_ptr = block
        .append_operation(llvm::load(
            context,
            stack_ptr_ptr.into(),
            ptr_type,
            location,
            LoadStoreOptions::default(),
        ))
        .result(0)?;

    // Store value at stack pointer
    let res = block.append_operation(llvm::store(
        context,
        value,
        stack_ptr.into(),
        location,
        LoadStoreOptions::default(),
    ));
    assert!(res.verify());

    // Increment stack pointer
    let new_stack_ptr = block
        .append_operation(llvm::get_element_ptr(
            context,
            stack_ptr.into(),
            DenseI32ArrayAttribute::new(context, &[1]),
            uint256.into(),
            ptr_type,
            location,
        ))
        .result(0)?;

    // Store incremented stack pointer
    let res = block.append_operation(llvm::store(
        context,
        new_stack_ptr.into(),
        stack_ptr_ptr.into(),
        location,
        LoadStoreOptions::default(),
    ));
    assert!(res.verify());

    Ok(())
}

// Returns a copy of the nth value of the stack along with its stack's address
pub fn get_nth_from_stack<'ctx>(
    context: &'ctx MeliorContext,
    block: &'ctx Block,
    nth: u8,
) -> Result<(Value<'ctx, 'ctx>, OperationResult<'ctx, 'ctx>), CodegenError> {
    debug_assert!((nth as u32) < MAX_STACK_SIZE as u32);
    let uint256 = IntegerType::new(context, 256);
    let location = Location::unknown(context);
    let ptr_type = pointer(context, 0);

    // Get address of stack pointer global
    let stack_ptr_ptr = block
        .append_operation(llvm_mlir::addressof(
            context,
            STACK_PTR_GLOBAL,
            ptr_type,
            location,
        ))
        .result(0)?;

    // Load stack pointer
    let stack_ptr = block
        .append_operation(llvm::load(
            context,
            stack_ptr_ptr.into(),
            ptr_type,
            location,
            LoadStoreOptions::default(),
        ))
        .result(0)?;

    // Decrement stack pointer
    let nth_stack_ptr = block
        .append_operation(llvm::get_element_ptr(
            context,
            stack_ptr.into(),
            DenseI32ArrayAttribute::new(context, &[-(nth as i32)]),
            uint256.into(),
            ptr_type,
            location,
        ))
        .result(0)?;

    // Load value from top of stack
    let value = block
        .append_operation(llvm::load(
            context,
            nth_stack_ptr.into(),
            uint256.into(),
            location,
            LoadStoreOptions::default(),
        ))
        .result(0)?
        .into();

    Ok((value, nth_stack_ptr))
}

pub fn swap_stack_elements<'ctx>(
    context: &'ctx MeliorContext,
    block: &'ctx Block,
    position_1: u8,
    position_2: u8,
) -> Result<(), CodegenError> {
    debug_assert!((position_1 as u32) < MAX_STACK_SIZE as u32);
    debug_assert!((position_2 as u32) < MAX_STACK_SIZE as u32);
    let location = Location::unknown(context);

    let (first_element, first_elem_address) = get_nth_from_stack(context, block, position_1)?;
    let (nth_element, nth_elem_address) = get_nth_from_stack(context, block, position_2)?;

    // Store element in position 1 into position 2
    let res = block.append_operation(llvm::store(
        context,
        first_element,
        nth_elem_address.into(),
        location,
        LoadStoreOptions::default(),
    ));
    assert!(res.verify());

    // Store element in position 2 into position 1
    let res = block.append_operation(llvm::store(
        context,
        nth_element,
        first_elem_address.into(),
        location,
        LoadStoreOptions::default(),
    ));
    assert!(res.verify());

    Ok(())
}

/// Generates code for checking if the stack has enough space for `element_count` more elements.
pub fn check_stack_has_space_for<'ctx>(
    context: &'ctx MeliorContext,
    block: &'ctx Block,
    element_count: u32,
) -> Result<Value<'ctx, 'ctx>, CodegenError> {
    debug_assert!(element_count < MAX_STACK_SIZE as u32);
    let location = Location::unknown(context);
    let ptr_type = pointer(context, 0);
    let uint256 = IntegerType::new(context, 256);

    // Get address of stack pointer global
    let stack_ptr_ptr = block
        .append_operation(llvm_mlir::addressof(
            context,
            STACK_PTR_GLOBAL,
            ptr_type,
            location,
        ))
        .result(0)?;

    // Load stack pointer
    let stack_ptr = block
        .append_operation(llvm::load(
            context,
            stack_ptr_ptr.into(),
            ptr_type,
            location,
            LoadStoreOptions::default(),
        ))
        .result(0)?;

    // Get address of stack base pointer global
    let stack_baseptr_ptr = block
        .append_operation(llvm_mlir::addressof(
            context,
            STACK_BASEPTR_GLOBAL,
            ptr_type,
            location,
        ))
        .result(0)?;

    // Load stack base pointer
    let stack_baseptr = block
        .append_operation(llvm::load(
            context,
            stack_baseptr_ptr.into(),
            ptr_type,
            location,
            LoadStoreOptions::default(),
        ))
        .result(0)?;

    // Compare `subtracted_stack_ptr = stack_ptr + element_count - MAX_STACK_SIZE`
    let subtracted_stack_ptr = block
        .append_operation(llvm::get_element_ptr(
            context,
            stack_ptr.into(),
            DenseI32ArrayAttribute::new(context, &[element_count as i32 - MAX_STACK_SIZE as i32]),
            uint256.into(),
            ptr_type,
            location,
        ))
        .result(0)?;

    // Compare `stack_ptr + element_count - MAX_STACK_SIZE <= stack_baseptr`
    let flag = block
        .append_operation(
            ods::llvm::icmp(
                context,
                IntegerType::new(context, 1).into(),
                subtracted_stack_ptr.into(),
                stack_baseptr.into(),
                // 7 should be the "ule" predicate enum value
                IntegerAttribute::new(
                    IntegerType::new(context, 64).into(),
                    /* "ule" predicate enum value */ 7,
                )
                .into(),
                location,
            )
            .into(),
        )
        .result(0)?;

    Ok(flag.into())
}

/// Generates code for checking if the stack has enough space for `element_count` more elements.
/// Returns true if there are at least `element_count` elements in the stack.
pub fn check_stack_has_at_least<'ctx>(
    context: &'ctx MeliorContext,
    block: &'ctx Block,
    element_count: u32,
) -> Result<Value<'ctx, 'ctx>, CodegenError> {
    debug_assert!(element_count < MAX_STACK_SIZE as u32);
    let location = Location::unknown(context);
    let ptr_type = pointer(context, 0);
    let uint256 = IntegerType::new(context, 256);

    // Get address of stack pointer global
    let stack_ptr_ptr = block
        .append_operation(llvm_mlir::addressof(
            context,
            STACK_PTR_GLOBAL,
            ptr_type,
            location,
        ))
        .result(0)?;

    // Load stack pointer
    let stack_ptr = block
        .append_operation(llvm::load(
            context,
            stack_ptr_ptr.into(),
            ptr_type,
            location,
            LoadStoreOptions::default(),
        ))
        .result(0)?;

    // Get address of stack base pointer global
    let stack_baseptr_ptr = block
        .append_operation(llvm_mlir::addressof(
            context,
            STACK_BASEPTR_GLOBAL,
            ptr_type,
            location,
        ))
        .result(0)?;

    // Load stack base pointer
    let stack_baseptr = block
        .append_operation(llvm::load(
            context,
            stack_baseptr_ptr.into(),
            ptr_type,
            location,
            LoadStoreOptions::default(),
        ))
        .result(0)?;

    // Compare `subtracted_stack_ptr = stack_ptr - element_count`
    let subtracted_stack_ptr = block
        .append_operation(llvm::get_element_ptr(
            context,
            stack_ptr.into(),
            DenseI32ArrayAttribute::new(context, &[-(element_count as i32)]),
            uint256.into(),
            ptr_type,
            location,
        ))
        .result(0)?;

    // Compare `stack_ptr - element_count >= stack_baseptr`
    let flag = block
        .append_operation(
            ods::llvm::icmp(
                context,
                IntegerType::new(context, 1).into(),
                subtracted_stack_ptr.into(),
                stack_baseptr.into(),
                IntegerAttribute::new(
                    IntegerType::new(context, 64).into(),
                    /* "uge" predicate enum value */ 9,
                )
                .into(),
                location,
            )
            .into(),
        )
        .result(0)?;

    Ok(flag.into())
}
