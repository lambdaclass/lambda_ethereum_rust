use melior::ExecutionEngine;

use crate::{
    constants::MAIN_ENTRYPOINT,
    module::MLIRModule,
    syscall::{MainFunc, SyscallContext},
};

#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum OptLevel {
    None = 0,
    Less,
    #[default]
    Default,
    Aggressive,
}

pub struct Executor {
    engine: ExecutionEngine,
}

impl Executor {
    pub fn new(module: &MLIRModule, syscall_ctx: &SyscallContext, opt_level: OptLevel) -> Self {
        let engine = ExecutionEngine::new(module.module(), opt_level as usize, &[], false);
        syscall_ctx.register_symbols(&engine);
        Self { engine }
    }

    pub fn execute(&self, context: &mut SyscallContext, initial_gas: u64) -> u8 {
        let main_fn: MainFunc = self.get_main_entrypoint();

        main_fn(context, initial_gas)
    }

    fn get_main_entrypoint(&self) -> MainFunc {
        let function_name = format!("_mlir_ciface_{MAIN_ENTRYPOINT}");
        let fptr = self.engine.lookup(&function_name);
        unsafe { std::mem::transmute(fptr) }
    }
}
