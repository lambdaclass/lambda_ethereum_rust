use ethereum_rust_core::types::{BlockHeader, LegacyTransaction, Transaction};
use ethereum_rust_levm::{
    block::BlockEnv,
    env::Env,
    transaction::TxEnv,
    vm::{Db, VM},
    vm_result::ResultAndState,
};

extern crate ethereum_rust_levm;

fn main() {
    let tx = Transaction::LegacyTransaction(LegacyTransaction::default());
    let header = BlockHeader::default();

    let db = Db::default();

    let env = Env::new_with_env(&tx, &header);

    let mut vm = VM::new(env, db);

    let ResultAndState { result, state } = vm.transact().unwrap();
}
