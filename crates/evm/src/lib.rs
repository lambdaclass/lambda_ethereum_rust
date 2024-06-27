use bytes::Bytes;
use core::types::{BlockHeader, Transaction};
use revm::{
    primitives::{ruint::Uint, Address, BlockEnv, ExecutionResult, Output, TxEnv, TxKind, U256},
    Evm,
};

fn run_evm(tx: &Transaction, header: &BlockHeader) {
    let caller = Address([0; 20].into());
    let to = Address([0; 20].into());
    let code = Bytes::new();
    let block_env = block_env(header);
    let tx_env = tx_env(tx);
    let mut evm = Evm::builder()
        .with_block_env(block_env)
        .with_tx_env(tx_env)
        .build();
    let _tx_result = evm.transact().unwrap();
}

fn block_env(header: &BlockHeader) -> BlockEnv {
    BlockEnv {
        number: Uint::<256, 4>::from(header.number),
        coinbase: Address(header.coinbase.0.into()),
        timestamp: Uint::<256, 4>::from(header.timestamp),
        gas_limit: Uint::<256, 4>::from(header.gas_limit),
        basefee: Uint::<256, 4>::from(header.base_fee_per_gas),
        difficulty: Uint::<256, 4>::from_limbs(header.difficulty.0),
        prevrandao: Some(header.prev_randao.as_fixed_bytes().into()),
        ..Default::default()
    }
}

fn tx_env(tx: &Transaction) -> TxEnv {
    TxEnv {
        caller: todo!(),
        gas_limit: todo!(),
        gas_price: todo!(),
        transact_to: todo!(),
        value: todo!(),
        data: todo!(),
        nonce: todo!(),
        chain_id: todo!(),
        access_list: todo!(),
        gas_priority_fee: todo!(),
        blob_hashes: todo!(),
        max_fee_per_blob_gas: todo!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
}
