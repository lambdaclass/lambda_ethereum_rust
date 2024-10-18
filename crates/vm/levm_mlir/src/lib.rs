use builder::EvmBuilder;
use db::{Database, Db};
use executor::{Executor, OptLevel};
use journal::Journal;
use program::Program;
use report::{InvalidTransaction, TransactionReport};
use syscall::{CallFrame, SyscallContext};

use crate::context::Context;

pub mod builder;
pub mod codegen;
pub mod constants;
pub mod context;
pub mod db;
pub mod env;
pub mod errors;
pub mod executor;
pub mod module;
pub mod primitives;
pub mod program;
pub mod syscall;
pub mod utils;
pub use env::Environment;
pub mod journal;
pub mod precompiles;
pub mod report;
pub mod state;

#[derive(Debug)]
pub struct Evm<DB: Database> {
    pub env: Environment,
    pub db: DB,
}

impl<DB: Database + Default> Evm<DB> {
    /// Returns evm builder with empty database.
    pub fn builder() -> EvmBuilder<DB> {
        EvmBuilder::default()
    }

    /// Creates a new EVM instance with the given environment and database.
    pub fn new(env: Environment, db: DB) -> Self {
        Self { env, db }
    }
}

impl Evm<Db> {
    fn validate_transaction(&mut self) -> Result<u64, InvalidTransaction> {
        self.env.validate_transaction()?;
        let initial_gas_consumed = self.env.consume_intrinsic_cost()?;

        Ok(initial_gas_consumed)
    }

    fn create_syscall_context(&mut self, initial_gas: u64) -> SyscallContext {
        let call_frame = CallFrame::new(self.env.tx_caller);
        let journal = Journal::new(&mut self.db).with_prefetch(&self.env.tx_access_list);
        SyscallContext::new(self.env.clone(), journal, call_frame, initial_gas)
    }

    fn run_program(&mut self, program: Program, initial_gas_consumed: u64) -> TransactionReport {
        let context = Context::new();
        let module = context
            .compile(&program, Default::default())
            .expect("failed to compile program");

        let gas_limit = self.env.tx_gas_limit;
        let mut context = self.create_syscall_context(gas_limit + initial_gas_consumed);
        let executor = Executor::new(&module, &context, OptLevel::Aggressive);

        // TODO: improve this once we stabilize the API a bit
        context.inner_context.program = program.to_bytecode();
        executor.execute(&mut context, gas_limit);

        context.get_report()
    }

    fn call(&mut self, initial_gas_consumed: u64) -> Result<TransactionReport, InvalidTransaction> {
        let code_address = self.env.get_tx_code_address();

        let bytecode = match self.db.code_by_address(code_address) {
            Ok(bytecode) => bytecode,
            Err(_) => return Err(InvalidTransaction::TargetContractDoesNotExist),
        };

        let program = Program::from_bytecode(&bytecode);
        Ok(self.run_program(program, initial_gas_consumed))
    }

    fn get_env_value(&self) -> syscall::U256 {
        let mut ethereum_value = self.env.tx_value.0.to_vec();
        ethereum_value.reverse(); // we have to reverse the bytes, it's in little endian and we use big endian with syscall
        let mut value = [0u8; 32];

        for (i, num) in ethereum_value.iter().enumerate() {
            value[i * 8..(i + 1) * 8].copy_from_slice(&num.to_be_bytes());
        }

        syscall::U256::from_fixed_be_bytes(value)
    }

    fn create(&mut self, initial_gas_consumed: u64) -> TransactionReport {
        let mut value = self.get_env_value();
        let mut remaining_gas = self.env.tx_gas_limit;
        let gas_limit = self.env.tx_gas_limit;
        let program = self.env.tx_calldata.to_vec();
        let program_size = program.len() as u32;
        let mut context = self.create_syscall_context(gas_limit + initial_gas_consumed);
        context.inner_context.memory = program;

        context.create(program_size, 0, &mut value, &mut remaining_gas);
        context.inner_context.gas_remaining = Some(gas_limit.saturating_sub(remaining_gas));
        context.get_report()
    }

    pub fn transact(&mut self) -> Result<TransactionReport, InvalidTransaction> {
        let initial_gas_consumed = self.validate_transaction()?;
        match self.env.tx_to {
            Some(_) => self.call(initial_gas_consumed),
            None => Ok(self.create(initial_gas_consumed)),
        }
    }

    pub fn transact_commit(&mut self) -> Result<TransactionReport, InvalidTransaction> {
        let report = self.transact()?;
        if let Some(new_state) = report.new_state.clone() {
            self.db.commit(new_state);
        }

        Ok(report)
    }
}
