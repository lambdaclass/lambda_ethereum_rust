use crate::{EvmError, EvmState};
use bytes::Bytes;
use ethereum_rust_core::{
    types::{Block, Receipt},
    Address,
};
use ethereum_rust_levm::vm::{Db, VM};

pub fn execute_block(block: &Block, _state: &mut EvmState) -> Result<Vec<Receipt>, EvmError> {
    for transaction in block.body.transactions.iter() {
        // TODO: check all the parameters
        let mut vm = VM::new(
            Address::random(),
            transaction.sender(),
            transaction.value(),
            Bytes::new(),
            block.header.gas_limit,
            transaction.gas_limit().into(),
            block.header.number.into(),
            block.header.coinbase,
            block.header.timestamp.into(),
            Some(block.header.prev_randao),
            transaction.chain_id().unwrap().into(),
            block.header.base_fee_per_gas.unwrap().into(),
            Some(transaction.gas_price().into()),
            Db::default(),
        );
        let _result = vm.transact();
    }
    // TODO: map the result to the expected Result<Vec<Receipt>, EvmError>
    Ok(Vec::new())
}
