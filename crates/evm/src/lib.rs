use bytes::Bytes;
use revm::{primitives::{Address, ExecutionResult, Output, TxEnv, TxKind, U256}, Evm};

fn run_evm() {
    let caller = Address([0;20].into());
    let to = Address([0;20].into());
    let code = Bytes::new();
    let mut evm = Evm::builder().modify_tx_env(|tx| {
        tx.caller = caller;
        tx.transact_to = TxKind::Call(to);
        tx.data = code.into();
        tx.value = U256::from(0);
    }).build();
    let tx_result = evm.transact().unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
