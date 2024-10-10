use std::collections::BTreeMap;

use melior::{
    dialect::{
        arith, cf, func,
        llvm::{self, attributes::Linkage, r#type::pointer, AllocaOptions, LoadStoreOptions},
    },
    ir::{
        attribute::{IntegerAttribute, TypeAttribute},
        r#type::IntegerType,
        Block, BlockRef, Location, Module, Region, Value,
    },
    Context as MeliorContext,
};

use crate::{
    constants::{
        CallType, CALLDATA_PTR_GLOBAL, CALLDATA_SIZE_GLOBAL, GAS_COUNTER_GLOBAL, MAX_STACK_SIZE,
        MEMORY_PTR_GLOBAL, MEMORY_SIZE_GLOBAL, STACK_BASEPTR_GLOBAL, STACK_PTR_GLOBAL,
    },
    errors::CodegenError,
    program::{Operation, Program},
    syscall::{self, ExitStatusCode},
    utils::{
        allocate_and_store_value, constant_value_from_i64, consume_gas_as_value, get_remaining_gas,
        integer_constant_from_u8, llvm_mlir,
    },
};

#[derive(Debug, Clone)]
pub(crate) struct OperationCtx<'c> {
    /// The MLIR context.
    pub mlir_context: &'c MeliorContext,
    /// The program IR.
    pub program: &'c Program,
    /// The syscall context to be passed to syscalls.
    pub syscall_ctx: Value<'c, 'c>,
    /// Reference to the revert block.
    /// This block takes care of reverts.
    pub revert_block: BlockRef<'c, 'c>,
    /// Reference to the jump table block.
    /// This block receives the PC as an argument and jumps to the block corresponding to that PC,
    /// or reverts in case the destination is not a JUMPDEST.
    pub jumptable_block: BlockRef<'c, 'c>,
    /// Blocks to jump to. These are registered dynamically as JUMPDESTs are processed.
    pub jumpdest_blocks: BTreeMap<usize, BlockRef<'c, 'c>>,
}

impl<'c> OperationCtx<'c> {
    pub(crate) fn new(
        context: &'c MeliorContext,
        module: &'c Module,
        region: &'c Region,
        setup_block: &'c Block<'c>,
        program: &'c Program,
    ) -> Result<Self, CodegenError> {
        let location = Location::unknown(context);
        let ptr_type = pointer(context, 0);
        let uint64 = IntegerType::new(context, 64).into();
        // PERF: avoid generating unneeded setup blocks
        let syscall_ctx = setup_block.add_argument(ptr_type, location);
        let initial_gas = setup_block.add_argument(uint64, location);

        // Append setup code to be run at the start
        generate_stack_setup_code(context, module, setup_block)?;
        generate_memory_setup_code(context, module, setup_block)?;
        generate_calldata_setup_code(context, syscall_ctx, module, setup_block)?;
        generate_gas_counter_setup_code(context, module, setup_block, initial_gas)?;

        syscall::mlir::declare_symbols(context, module);

        // Generate helper blocks
        let revert_block = region.append_block(generate_revert_block(context, syscall_ctx)?);
        let jumptable_block = region.append_block(create_jumptable_landing_block(context));

        let op_ctx = OperationCtx {
            mlir_context: context,
            program,
            syscall_ctx,
            revert_block,
            jumptable_block,
            jumpdest_blocks: Default::default(),
        };
        Ok(op_ctx)
    }

    /// Populate the jumptable block with a dynamic dispatch according to the
    /// received PC.
    pub(crate) fn populate_jumptable(&self) -> Result<(), CodegenError> {
        let context = self.mlir_context;
        let program = self.program;
        let start_block = self.jumptable_block;

        let location = Location::unknown(context);
        let uint256 = IntegerType::new(context, 256);

        // The block receives a single argument: the value to switch on
        // TODO: move to program module
        let jumpdest_pcs: Vec<i64> = program
            .operations
            .iter()
            .filter_map(|op| match op {
                Operation::Jumpdest { pc } => Some(*pc as i64),
                _ => None,
            })
            .collect();

        let arg = start_block.argument(0)?;

        let case_destinations: Vec<_> = self
            .jumpdest_blocks
            .values()
            .map(|b| {
                let x: (&Block, &[Value]) = (b, &[]);
                x
            })
            .collect();

        let op = start_block.append_operation(cf::switch(
            context,
            &jumpdest_pcs,
            arg.into(),
            uint256.into(),
            (&self.revert_block, &[]),
            &case_destinations,
            location,
        )?);

        assert!(op.verify());

        Ok(())
    }

    /// Registers a block as a valid jump destination.
    // TODO: move into jumptable module
    pub(crate) fn register_jump_destination(&mut self, pc: usize, block: BlockRef<'c, 'c>) {
        self.jumpdest_blocks.insert(pc, block);
    }

    /// Registers a block as a valid jump destination.
    // TODO: move into jumptable module
    #[allow(dead_code)]
    pub(crate) fn add_jump_op(
        &mut self,
        block: BlockRef<'c, 'c>,
        pc_to_jump_to: Value,
        location: Location,
    ) {
        let op = block.append_operation(cf::br(&self.jumptable_block, &[pc_to_jump_to], location));
        assert!(op.verify());
    }
}

fn generate_gas_counter_setup_code<'c>(
    context: &'c MeliorContext,
    module: &'c Module,
    block: &'c Block<'c>,
    initial_gas: Value,
) -> Result<(), CodegenError> {
    let location = Location::unknown(context);
    let ptr_type = pointer(context, 0);
    let uint64 = IntegerType::new(context, 64).into();

    let body = module.body();
    let res = body.append_operation(llvm_mlir::global(
        context,
        GAS_COUNTER_GLOBAL,
        uint64,
        Linkage::Internal,
        location,
    ));

    assert!(res.verify());

    let gas_addr = block
        .append_operation(llvm_mlir::addressof(
            context,
            GAS_COUNTER_GLOBAL,
            ptr_type,
            location,
        ))
        .result(0)?;

    let res = block.append_operation(llvm::store(
        context,
        initial_gas,
        gas_addr.into(),
        location,
        LoadStoreOptions::default(),
    ));

    assert!(res.verify());

    Ok(())
}

fn generate_stack_setup_code<'c>(
    context: &'c MeliorContext,
    module: &'c Module,
    block: &'c Block<'c>,
) -> Result<(), CodegenError> {
    let location = Location::unknown(context);
    let ptr_type = pointer(context, 0);

    // Declare the stack pointer and base pointer globals
    let body = module.body();
    let res = body.append_operation(llvm_mlir::global(
        context,
        STACK_BASEPTR_GLOBAL,
        ptr_type,
        Linkage::Internal,
        location,
    ));
    assert!(res.verify());
    let res = body.append_operation(llvm_mlir::global(
        context,
        STACK_PTR_GLOBAL,
        ptr_type,
        Linkage::Internal,
        location,
    ));
    assert!(res.verify());

    let uint256 = IntegerType::new(context, 256);

    // Allocate stack memory
    let stack_size = block
        .append_operation(arith::constant(
            context,
            IntegerAttribute::new(uint256.into(), MAX_STACK_SIZE as i64).into(),
            location,
        ))
        .result(0)?
        .into();

    let stack_baseptr = block
        .append_operation(llvm::alloca(
            context,
            stack_size,
            ptr_type,
            location,
            AllocaOptions::new().elem_type(Some(TypeAttribute::new(uint256.into()))),
        ))
        .result(0)?;

    // Populate the globals with the allocated stack memory
    let stack_baseptr_ptr = block
        .append_operation(llvm_mlir::addressof(
            context,
            STACK_BASEPTR_GLOBAL,
            ptr_type,
            location,
        ))
        .result(0)?;

    let res = block.append_operation(llvm::store(
        context,
        stack_baseptr.into(),
        stack_baseptr_ptr.into(),
        location,
        LoadStoreOptions::default(),
    ));
    assert!(res.verify());

    let stackptr_ptr = block
        .append_operation(llvm_mlir::addressof(
            context,
            STACK_PTR_GLOBAL,
            ptr_type,
            location,
        ))
        .result(0)?;

    let res = block.append_operation(llvm::store(
        context,
        stack_baseptr.into(),
        stackptr_ptr.into(),
        location,
        LoadStoreOptions::default(),
    ));
    assert!(res.verify());

    Ok(())
}

fn generate_memory_setup_code<'c>(
    context: &'c MeliorContext,
    module: &'c Module,
    block: &'c Block<'c>,
) -> Result<(), CodegenError> {
    let location = Location::unknown(context);
    let ptr_type = pointer(context, 0);
    let uint32 = IntegerType::new(context, 32).into();

    // Declare the stack pointer and base pointer globals
    let body = module.body();
    let res = body.append_operation(llvm_mlir::global(
        context,
        MEMORY_PTR_GLOBAL,
        ptr_type,
        Linkage::Internal,
        location,
    ));
    assert!(res.verify());
    let res = body.append_operation(llvm_mlir::global(
        context,
        MEMORY_SIZE_GLOBAL,
        uint32,
        Linkage::Internal,
        location,
    ));
    assert!(res.verify());

    let zero = block
        .append_operation(arith::constant(
            context,
            IntegerAttribute::new(uint32, 0).into(),
            location,
        ))
        .result(0)?
        .into();

    let memory_size_ptr = block
        .append_operation(llvm_mlir::addressof(
            context,
            MEMORY_SIZE_GLOBAL,
            ptr_type,
            location,
        ))
        .result(0)?;

    let res = block.append_operation(llvm::store(
        context,
        zero,
        memory_size_ptr.into(),
        location,
        LoadStoreOptions::default(),
    ));
    assert!(res.verify());

    Ok(())
}

fn generate_calldata_setup_code<'c>(
    context: &'c MeliorContext,
    syscall_ctx: Value<'c, 'c>,
    module: &'c Module,
    block: &'c Block<'c>,
) -> Result<(), CodegenError> {
    let location = Location::unknown(context);
    let ptr_type = pointer(context, 0);
    let uint32 = IntegerType::new(context, 32).into();

    // Declare globals
    let body = module.body();
    let res = body.append_operation(llvm_mlir::global(
        context,
        CALLDATA_PTR_GLOBAL,
        ptr_type,
        Linkage::Internal,
        location,
    ));
    assert!(res.verify());
    let res = body.append_operation(llvm_mlir::global(
        context,
        CALLDATA_SIZE_GLOBAL,
        uint32,
        Linkage::Internal,
        location,
    ));
    assert!(res.verify());

    // Setup CALLDATA_PTR_GLOBAL
    let calldata_ptr_value =
        syscall::mlir::get_calldata_ptr_syscall(context, syscall_ctx, block, location)?;
    let calldata_ptr_ptr = block
        .append_operation(llvm_mlir::addressof(
            context,
            CALLDATA_PTR_GLOBAL,
            ptr_type,
            location,
        ))
        .result(0)?;

    block.append_operation(llvm::store(
        context,
        calldata_ptr_value,
        calldata_ptr_ptr.into(),
        location,
        LoadStoreOptions::default(),
    ));

    // Setup CALLDATA_SIZE_GLOBAL
    let calldata_size_value =
        syscall::mlir::get_calldata_size_syscall(context, syscall_ctx, block, location)?;
    let calldata_size_ptr = block
        .append_operation(llvm_mlir::addressof(
            context,
            CALLDATA_SIZE_GLOBAL,
            ptr_type,
            location,
        ))
        .result(0)?;

    block.append_operation(llvm::store(
        context,
        calldata_size_value,
        calldata_size_ptr.into(),
        location,
        LoadStoreOptions::default(),
    ));

    Ok(())
}

/// Create the jumptable landing block. This is the main entrypoint
/// for JUMP and JUMPI operations.
fn create_jumptable_landing_block(context: &MeliorContext) -> Block {
    let location = Location::unknown(context);
    let uint256 = IntegerType::new(context, 256);
    Block::new(&[(uint256.into(), location)])
}

pub fn generate_revert_block<'c>(
    context: &'c MeliorContext,
    syscall_ctx: Value<'c, 'c>,
) -> Result<Block<'c>, CodegenError> {
    let location = Location::unknown(context);
    let uint32 = IntegerType::new(context, 32).into();
    let uint64 = IntegerType::new(context, 64).into();

    let revert_block = Block::new(&[]);
    let remaining_gas = get_remaining_gas(context, &revert_block)?;

    let zero_u32 = revert_block
        .append_operation(arith::constant(
            context,
            IntegerAttribute::new(uint32, 0).into(),
            location,
        ))
        .result(0)?
        .into();

    let zero_u64 = revert_block
        .append_operation(arith::constant(
            context,
            IntegerAttribute::new(uint64, 0).into(),
            location,
        ))
        .result(0)?
        .into();

    let reason = revert_block
        .append_operation(arith::constant(
            context,
            integer_constant_from_u8(context, ExitStatusCode::Error.to_u8()).into(),
            location,
        ))
        .result(0)?
        .into();

    consume_gas_as_value(context, &revert_block, remaining_gas)?;

    syscall::mlir::write_result_syscall(
        context,
        syscall_ctx,
        &revert_block,
        zero_u32,
        zero_u32,
        zero_u64,
        reason,
        location,
    );

    revert_block.append_operation(func::r#return(&[reason], location));

    Ok(revert_block)
}

// Syscall MLIR wrappers
impl<'c> OperationCtx<'c> {
    pub(crate) fn write_result_syscall(
        &self,
        block: &Block,
        offset: Value,
        size: Value,
        gas: Value,
        reason: Value,
        location: Location,
    ) {
        syscall::mlir::write_result_syscall(
            self.mlir_context,
            self.syscall_ctx,
            block,
            offset,
            size,
            gas,
            reason,
            location,
        )
    }

    pub(crate) fn keccak256_syscall(
        &'c self,
        block: &'c Block,
        offset: Value<'c, 'c>,
        size: Value<'c, 'c>,
        hash_ptr: Value<'c, 'c>,
        location: Location<'c>,
    ) {
        syscall::mlir::keccak256_syscall(
            self.mlir_context,
            self.syscall_ctx,
            block,
            offset,
            size,
            hash_ptr,
            location,
        )
    }

    pub(crate) fn get_calldata_size_syscall(
        &'c self,
        block: &'c Block,
        location: Location<'c>,
    ) -> Result<Value, CodegenError> {
        syscall::mlir::get_calldata_size_syscall(
            self.mlir_context,
            self.syscall_ctx,
            block,
            location,
        )
    }

    pub(crate) fn get_calldata_ptr_syscall(
        &'c self,
        block: &'c Block,
        location: Location<'c>,
    ) -> Result<Value, CodegenError> {
        syscall::mlir::get_calldata_ptr_syscall(
            self.mlir_context,
            self.syscall_ctx,
            block,
            location,
        )
    }

    pub(crate) fn get_origin_syscall(
        &'c self,
        block: &'c Block,
        address_ptr: Value<'c, 'c>,
        location: Location<'c>,
    ) {
        syscall::mlir::get_origin_syscall(
            self.mlir_context,
            self.syscall_ctx,
            block,
            address_ptr,
            location,
        )
    }

    pub(crate) fn get_chainid_syscall(
        &'c self,
        block: &'c Block,
        location: Location<'c>,
    ) -> Result<Value, CodegenError> {
        syscall::mlir::get_chainid_syscall(self.mlir_context, self.syscall_ctx, block, location)
    }

    pub(crate) fn store_in_callvalue_ptr(
        &'c self,
        block: &'c Block,
        location: Location<'c>,
        callvalue_ptr: Value<'c, 'c>,
    ) {
        syscall::mlir::store_in_callvalue_ptr(
            self.mlir_context,
            self.syscall_ctx,
            block,
            location,
            callvalue_ptr,
        )
    }

    pub(crate) fn store_in_caller_ptr(
        &'c self,
        block: &'c Block,
        location: Location<'c>,
        caller_ptr: Value<'c, 'c>,
    ) {
        syscall::mlir::store_in_caller_ptr(
            self.mlir_context,
            self.syscall_ctx,
            block,
            location,
            caller_ptr,
        )
    }

    pub(crate) fn store_in_blobbasefee_ptr(
        &'c self,
        block: &'c Block,
        location: Location<'c>,
        blob_base_fee_ptr: Value<'c, 'c>,
    ) {
        syscall::mlir::store_in_blobbasefee_ptr(
            self.mlir_context,
            self.syscall_ctx,
            block,
            location,
            blob_base_fee_ptr,
        )
    }

    pub(crate) fn store_in_gasprice_ptr(
        &'c self,
        block: &'c Block,
        location: Location<'c>,
        gasprice_ptr: Value<'c, 'c>,
    ) {
        syscall::mlir::store_in_gasprice_ptr(
            self.mlir_context,
            self.syscall_ctx,
            block,
            location,
            gasprice_ptr,
        )
    }

    pub(crate) fn get_gaslimit(
        &'c self,
        block: &'c Block,
        location: Location<'c>,
    ) -> Result<Value, CodegenError> {
        syscall::mlir::get_gaslimit(self.mlir_context, self.syscall_ctx, block, location)
    }

    pub(crate) fn store_in_selfbalance_ptr(
        &'c self,
        block: &'c Block,
        location: Location<'c>,
        selfbalance_ptr: Value<'c, 'c>,
    ) {
        syscall::mlir::store_in_selfbalance_ptr(
            self.mlir_context,
            self.syscall_ctx,
            block,
            location,
            selfbalance_ptr,
        )
    }

    pub(crate) fn extend_memory_syscall(
        &'c self,
        block: &'c Block,
        new_size: Value<'c, 'c>,
        location: Location<'c>,
    ) -> Result<Value, CodegenError> {
        syscall::mlir::extend_memory_syscall(
            self.mlir_context,
            self.syscall_ctx,
            block,
            new_size,
            location,
        )
    }

    pub(crate) fn storage_read_syscall(
        &'c self,
        block: &'c Block,
        key: Value<'c, 'c>,
        value: Value<'c, 'c>,
        location: Location<'c>,
    ) -> Result<Value, CodegenError> {
        syscall::mlir::storage_read_syscall(
            self.mlir_context,
            self.syscall_ctx,
            block,
            key,
            value,
            location,
        )
    }

    pub(crate) fn storage_write_syscall(
        &'c self,
        block: &'c Block,
        key: Value<'c, 'c>,
        value: Value<'c, 'c>,
        location: Location<'c>,
    ) -> Result<Value, CodegenError> {
        syscall::mlir::storage_write_syscall(
            self.mlir_context,
            self.syscall_ctx,
            block,
            key,
            value,
            location,
        )
    }

    pub(crate) fn transient_storage_read_syscall(
        &'c self,
        block: &'c Block,
        key: Value<'c, 'c>,
        value: Value<'c, 'c>,
        location: Location<'c>,
    ) {
        syscall::mlir::transient_storage_read_syscall(
            self.mlir_context,
            self.syscall_ctx,
            block,
            key,
            value,
            location,
        )
    }

    pub(crate) fn transient_storage_write_syscall(
        &'c self,
        block: &'c Block,
        key: Value<'c, 'c>,
        value: Value<'c, 'c>,
        location: Location<'c>,
    ) {
        syscall::mlir::transient_storage_write_syscall(
            self.mlir_context,
            self.syscall_ctx,
            block,
            key,
            value,
            location,
        )
    }

    pub(crate) fn append_log_syscall(
        &'c self,
        block: &'c Block,
        data: Value<'c, 'c>,
        size: Value<'c, 'c>,
        location: Location<'c>,
    ) {
        syscall::mlir::append_log_syscall(
            self.mlir_context,
            self.syscall_ctx,
            block,
            data,
            size,
            location,
        );
    }

    pub(crate) fn append_log_with_one_topic_syscall(
        &'c self,
        block: &'c Block,
        data: Value<'c, 'c>,
        size: Value<'c, 'c>,
        topic: Value<'c, 'c>,
        location: Location<'c>,
    ) {
        syscall::mlir::append_log_with_one_topic_syscall(
            self.mlir_context,
            self.syscall_ctx,
            block,
            data,
            size,
            topic,
            location,
        );
    }

    pub(crate) fn append_log_with_two_topics_syscall(
        &'c self,
        block: &'c Block,
        data: Value<'c, 'c>,
        size: Value<'c, 'c>,
        topic1_ptr: Value<'c, 'c>,
        topic2_ptr: Value<'c, 'c>,
        location: Location<'c>,
    ) {
        syscall::mlir::append_log_with_two_topics_syscall(
            self.mlir_context,
            self.syscall_ctx,
            block,
            data,
            size,
            topic1_ptr,
            topic2_ptr,
            location,
        );
    }
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn append_log_with_three_topics_syscall(
        &'c self,
        block: &'c Block,
        data: Value<'c, 'c>,
        size: Value<'c, 'c>,
        topic1_ptr: Value<'c, 'c>,
        topic2_ptr: Value<'c, 'c>,
        topic3_ptr: Value<'c, 'c>,
        location: Location<'c>,
    ) {
        syscall::mlir::append_log_with_three_topics_syscall(
            self.mlir_context,
            self.syscall_ctx,
            block,
            data,
            size,
            topic1_ptr,
            topic2_ptr,
            topic3_ptr,
            location,
        );
    }
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn append_log_with_four_topics_syscall(
        &'c self,
        block: &'c Block,
        data: Value<'c, 'c>,
        size: Value<'c, 'c>,
        topic1_ptr: Value<'c, 'c>,
        topic2_ptr: Value<'c, 'c>,
        topic3_ptr: Value<'c, 'c>,
        topic4_ptr: Value<'c, 'c>,
        location: Location<'c>,
    ) {
        syscall::mlir::append_log_with_four_topics_syscall(
            self.mlir_context,
            self.syscall_ctx,
            block,
            data,
            size,
            topic1_ptr,
            topic2_ptr,
            topic3_ptr,
            topic4_ptr,
            location,
        );
    }

    pub(crate) fn get_coinbase_ptr_syscall(
        &'c self,
        block: &'c Block,
        location: Location<'c>,
    ) -> Result<Value, CodegenError> {
        syscall::mlir::get_coinbase_ptr_syscall(
            self.mlir_context,
            self.syscall_ctx,
            block,
            location,
        )
    }

    #[allow(unused)]
    pub(crate) fn get_block_number_syscall(
        &'c self,
        block: &'c Block,
        number: Value<'c, 'c>,
        location: Location<'c>,
    ) {
        syscall::mlir::get_block_number_syscall(
            self.mlir_context,
            self.syscall_ctx,
            block,
            number,
            location,
        )
    }

    pub(crate) fn store_in_timestamp_ptr(
        &'c self,
        block: &'c Block,
        location: Location<'c>,
        timestamp_ptr: Value<'c, 'c>,
    ) {
        syscall::mlir::store_in_timestamp_ptr(
            self.mlir_context,
            self.syscall_ctx,
            block,
            location,
            timestamp_ptr,
        )
    }

    pub(crate) fn copy_code_to_memory_syscall(
        &'c self,
        block: &'c Block,
        offset: Value,
        size: Value,
        dest_offset: Value,
        location: Location<'c>,
    ) {
        syscall::mlir::copy_code_to_memory_syscall(
            self.mlir_context,
            self.syscall_ctx,
            block,
            offset,
            size,
            dest_offset,
            location,
        )
    }

    pub(crate) fn store_in_basefee_ptr_syscall(
        &'c self,
        basefee_ptr: Value<'c, 'c>,
        block: &'c Block,
        location: Location<'c>,
    ) {
        syscall::mlir::store_in_basefee_ptr_syscall(
            self.mlir_context,
            self.syscall_ctx,
            basefee_ptr,
            block,
            location,
        )
    }

    pub(crate) fn get_prevrandao_syscall(
        &'c self,
        block: &'c Block,
        prevrandao_ptr: Value<'c, 'c>,
        location: Location<'c>,
    ) {
        syscall::mlir::get_prevrandao_syscall(
            self.mlir_context,
            self.syscall_ctx,
            block,
            prevrandao_ptr,
            location,
        )
    }

    #[allow(unused)]
    pub(crate) fn get_address_ptr_syscall(
        &'c self,
        block: &'c Block,
        location: Location<'c>,
    ) -> Result<Value, CodegenError> {
        syscall::mlir::get_address_ptr_syscall(self.mlir_context, self.syscall_ctx, block, location)
    }

    pub(crate) fn store_in_balance_syscall(
        &'c self,
        block: &'c Block,
        address: Value<'c, 'c>,
        balance: Value<'c, 'c>,
        location: Location<'c>,
    ) -> Result<Value, CodegenError> {
        syscall::mlir::store_in_balance_syscall(
            self.mlir_context,
            self.syscall_ctx,
            block,
            address,
            balance,
            location,
        )
    }

    pub(crate) fn copy_ext_code_to_memory_syscall(
        &'c self,
        block: &'c Block,
        address_ptr: Value<'c, 'c>,
        offset: Value<'c, 'c>,
        size: Value<'c, 'c>,
        dest_offset: Value<'c, 'c>,
        location: Location<'c>,
    ) -> Result<Value, CodegenError> {
        syscall::mlir::copy_ext_code_to_memory_syscall(
            self.mlir_context,
            self.syscall_ctx,
            block,
            address_ptr,
            offset,
            size,
            dest_offset,
            location,
        )
    }

    pub(crate) fn get_codesize_from_address_syscall(
        &'c self,
        block: &'c Block,
        address: Value<'c, 'c>,
        gas: Value<'c, 'c>,
        location: Location<'c>,
    ) -> Result<Value, CodegenError> {
        syscall::mlir::get_codesize_from_address_syscall(
            self.mlir_context,
            self.syscall_ctx,
            block,
            address,
            gas,
            location,
        )
    }

    pub(crate) fn get_blob_hash_at_index_syscall(
        &'c self,
        block: &'c Block,
        index: Value<'c, 'c>,
        blobhash: Value<'c, 'c>,
        location: Location<'c>,
    ) {
        syscall::mlir::get_blob_hash_at_index_syscall(
            self.mlir_context,
            self.syscall_ctx,
            block,
            index,
            blobhash,
            location,
        )
    }

    pub(crate) fn get_block_hash_syscall(
        &'c self,
        block: &'c Block,
        block_number: Value<'c, 'c>,
        location: Location<'c>,
    ) {
        syscall::mlir::get_block_hash_syscall(
            self.mlir_context,
            self.syscall_ctx,
            block,
            block_number,
            location,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn call_syscall(
        &'c self,
        start_block: &'c Block,
        finish_block: &'c Block,
        location: Location<'c>,
        gas: Value<'c, 'c>,
        address: Value<'c, 'c>,
        value: Value<'c, 'c>,
        args_offset: Value<'c, 'c>,
        args_size: Value<'c, 'c>,
        ret_offset: Value<'c, 'c>,
        ret_size: Value<'c, 'c>,
        call_type: CallType,
    ) -> Result<Value, CodegenError> {
        let context = self.mlir_context;
        let uint64 = IntegerType::new(context, 64);
        let uint8 = IntegerType::new(context, 8);
        let ptr_type = pointer(context, 0);

        let available_gas = get_remaining_gas(context, start_block)?;
        // Alloc and store value argument
        // NOTE: We have to alloc memory for value on STATICCALL and DELEGATECALL
        // because we are using the same syscall. We could create a new syscall to not alloc memory
        // and optimize this
        let value_ptr = allocate_and_store_value(self, start_block, value, location)?;
        let address_ptr = allocate_and_store_value(self, start_block, address, location)?;

        // Alloc pointer to return gas value
        let gas_pointer_size = constant_value_from_i64(context, start_block, 1_i64)?;
        let gas_return_ptr = start_block
            .append_operation(llvm::alloca(
                context,
                gas_pointer_size,
                ptr_type,
                location,
                AllocaOptions::new().elem_type(Some(TypeAttribute::new(uint64.into()))),
            ))
            .result(0)?
            .into();

        let call_type_value = start_block
            .append_operation(arith::constant(
                context,
                IntegerAttribute::new(uint8.into(), call_type as u8 as i64).into(),
                location,
            ))
            .result(0)?
            .into();

        let return_value = syscall::mlir::call_syscall(
            context,
            self.syscall_ctx,
            start_block,
            location,
            gas,
            address_ptr,
            value_ptr,
            args_offset,
            args_size,
            ret_offset,
            ret_size,
            available_gas,
            gas_return_ptr,
            call_type_value,
        )?;

        // Update the available gas with the remaining gas after the call
        let consumed_gas = start_block
            .append_operation(llvm::load(
                context,
                gas_return_ptr,
                uint64.into(),
                location,
                LoadStoreOptions::default(),
            ))
            .result(0)?
            .into();
        let gas_flag = consume_gas_as_value(context, start_block, consumed_gas)?;

        start_block.append_operation(cf::cond_br(
            context,
            gas_flag,
            finish_block,
            &self.revert_block,
            &[],
            &[],
            location,
        ));

        // Extend the 8 bits result to 256 bits
        let uint256 = IntegerType::new(context, 256);

        let result = finish_block
            .append_operation(arith::extui(return_value, uint256.into(), location))
            .result(0)?
            .into();

        Ok(result)
    }

    pub(crate) fn get_code_hash_syscall(
        &'c self,
        block: &'c Block,
        address: Value<'c, 'c>,
        location: Location<'c>,
    ) -> Result<Value, CodegenError> {
        syscall::mlir::get_code_hash_syscall(
            self.mlir_context,
            self.syscall_ctx,
            block,
            address,
            location,
        )
    }

    pub(crate) fn create_syscall(
        &'c self,
        block: &'c Block,
        size: Value<'c, 'c>,
        offset: Value<'c, 'c>,
        value: Value<'c, 'c>,
        remaining_gas: Value<'c, 'c>,
        location: Location<'c>,
    ) -> Result<Value, CodegenError> {
        syscall::mlir::create_syscall(
            self.mlir_context,
            self.syscall_ctx,
            block,
            size,
            offset,
            value,
            remaining_gas,
            location,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn create2_syscall(
        &'c self,
        block: &'c Block,
        size: Value<'c, 'c>,
        offset: Value<'c, 'c>,
        value: Value<'c, 'c>,
        remaining_gas: Value<'c, 'c>,
        salt: Value<'c, 'c>,
        location: Location<'c>,
    ) -> Result<Value, CodegenError> {
        syscall::mlir::create2_syscall(
            self.mlir_context,
            self.syscall_ctx,
            block,
            size,
            offset,
            value,
            remaining_gas,
            salt,
            location,
        )
    }

    pub(crate) fn selfdestruct_syscall(
        &'c self,
        block: &'c Block,
        address: Value<'c, 'c>,
        location: Location<'c>,
    ) -> Result<Value, CodegenError> {
        syscall::mlir::selfdestruct_syscall(
            self.mlir_context,
            self.syscall_ctx,
            block,
            address,
            location,
        )
    }

    pub(crate) fn get_return_data_size(
        &'c self,
        block: &'c Block,
        location: Location<'c>,
    ) -> Result<Value, CodegenError> {
        let context = self.mlir_context;
        syscall::mlir::get_return_data_size(context, self.syscall_ctx, block, location)
    }

    pub(crate) fn copy_return_data_into_memory(
        &'c self,
        block: &'c Block,
        dest_offset: Value<'c, 'c>,
        offset: Value<'c, 'c>,
        size: Value<'c, 'c>,
        location: Location<'c>,
    ) {
        let context = self.mlir_context;
        syscall::mlir::copy_return_data_into_memory(
            context,
            self.syscall_ctx,
            block,
            dest_offset,
            offset,
            size,
            location,
        );
    }
}
