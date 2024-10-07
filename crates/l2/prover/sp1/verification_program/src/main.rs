#![no_main]
#![allow(unused_imports)]
#![allow(dead_code)]

use ethereum_rust_blockchain::{validate_gas_used, verify_blob_gas_usage};
use ethereum_rust_core::types::{
    validate_block_header, validate_cancun_header_fields, Receipt, Transaction,
};
use ethereum_rust_evm::{block_env, tx_env};
use prover_lib::{db_memorydb::MemoryDB, inputs::ProverInput};

use revm::{
    db::CacheDB, inspectors::TracerEip3155, primitives::ResultAndState as RevmResultAndState,
    Evm as Revm,
};

sp1_zkvm::entrypoint!(main);

pub fn main() {
    let head_block_bytes = sp1_zkvm::io::read::<Vec<u8>>();
    let parent_block_header_bytes = sp1_zkvm::io::read::<Vec<u8>>();

    // Make Inputs public.
    sp1_zkvm::io::commit(&head_block_bytes);
    sp1_zkvm::io::commit(&parent_block_header_bytes);

    let block = <ethereum_rust_core::types::Block as ethereum_rust_rlp::decode::RLPDecode>::decode(
        &head_block_bytes,
    )
    .unwrap();
    let parent_block_header =
        <ethereum_rust_core::types::BlockHeader as ethereum_rust_rlp::decode::RLPDecode>::decode(
            &parent_block_header_bytes,
        )
        .unwrap();

    // TODO: For execution.
    // let db = input.db;
    // let mut cache_db = CacheDB::new(db);

    let block_header_is_valid = validate_block(&block, &parent_block_header);

    // TODO: For execution.
    // let block_receipts = execute_block(&block, &mut cache_db).unwrap();
    // TODO
    // Handle the case in which the gas used differs and throws an error.
    // Should the zkVM panic? Should it generate a dummy proof?
    // let _ = validate_gas_used(&block_receipts, &block.header);

    // Make Output public.
    // TODO: For execution.
    // sp1_zkvm::io::commit(&block_receipts);
    sp1_zkvm::io::commit(&block_header_is_valid);
}

fn validate_block(
    block: &ethereum_rust_core::types::Block,
    parent_block_header: &ethereum_rust_core::types::BlockHeader,
) -> bool {
    validate_block_header(&block.header, parent_block_header).unwrap();
    validate_cancun_header_fields(&block.header, parent_block_header).unwrap();
    verify_blob_gas_usage(block).is_ok()
}

fn execute_block(
    block: &ethereum_rust_core::types::Block,
    db: &mut CacheDB<MemoryDB>,
) -> Result<Vec<Receipt>, ethereum_rust_evm::EvmError> {
    let spec_id = revm::primitives::SpecId::CANCUN;
    let mut receipts = Vec::new();
    let mut cumulative_gas_used = 0;

    for transaction in block.body.transactions.iter() {
        let result = execute_tx(transaction, &block.header, db, spec_id)?;
        cumulative_gas_used += result.gas_used();
        let receipt = Receipt::new(
            transaction.tx_type(),
            result.is_success(),
            cumulative_gas_used,
            result.logs(),
        );
        receipts.push(receipt);
    }

    Ok(receipts)
}

fn execute_tx(
    transaction: &Transaction,
    block_header: &ethereum_rust_core::types::BlockHeader,
    db: &mut CacheDB<MemoryDB>,
    spec_id: revm::primitives::SpecId,
) -> Result<ethereum_rust_evm::ExecutionResult, ethereum_rust_evm::EvmError> {
    let block_env = block_env(block_header);
    let tx_env = tx_env(transaction);
    run_evm(tx_env, block_env, db, spec_id)
        .map(Into::into)
        .map_err(ethereum_rust_evm::EvmError::from)
}

fn run_evm(
    tx_env: revm::primitives::TxEnv,
    block_env: revm::primitives::BlockEnv,
    db: &mut CacheDB<MemoryDB>,
    spec_id: revm::primitives::SpecId,
) -> Result<revm::primitives::ExecutionResult, ethereum_rust_evm::EvmError> {
    // let chain_spec = db.get_chain_config()?;
    let mut evm = Revm::builder()
        .with_db(db)
        .with_block_env(block_env)
        .with_tx_env(tx_env)
        // .modify_cfg_env(|cfg| cfg.chain_id = chain_spec.chain_id)
        .with_spec_id(spec_id)
        .with_external_context(TracerEip3155::new(Box::new(std::io::stderr())).without_summary())
        .build();
    let RevmResultAndState { result, state: _ } = evm.transact().unwrap();
    Ok(result)
}
