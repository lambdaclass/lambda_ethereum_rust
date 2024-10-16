use std::str::FromStr;

use revm::{
    primitives::{EVMError, Spec},
    Context, Database, FrameResult,
};
use revm_primitives::{Address, TxKind};
use tracing::info;

pub fn deduct_caller<SPEC: Spec, EXT, DB: Database>(
    context: &mut revm::Context<EXT, DB>,
) -> Result<(), EVMError<DB::Error>> {
    // load caller's account.
    let (caller_account, _) = context
        .evm
        .inner
        .journaled_state
        .load_account(context.evm.inner.env.tx.caller, &mut context.evm.inner.db)?;
    // If the transaction is a deposit with a `mint` value, add the mint value
    // in wei to the caller's balance. This should be persisted to the database
    // prior to the rest of execution.
    if context.evm.inner.env.tx.caller
        == Address::from_str("0x0007a881CD95B1484fca47615B64803dad620C8d").unwrap()
        && context.evm.inner.env.tx.data == *b"mint".as_slice()
    {
        info!("TX from privileged account with `mint` data");
        caller_account.info.balance = caller_account
            .info
            .balance
            .saturating_add(context.evm.inner.env.tx.value);
    }
    // deduct gas cost from caller's account.
    revm::handler::mainnet::deduct_caller_inner::<SPEC>(caller_account, &context.evm.inner.env);
    Ok(())
}

pub fn validate_tx_against_state<SPEC: Spec, EXT, DB: Database>(
    context: &mut Context<EXT, DB>,
) -> Result<(), EVMError<DB::Error>> {
    if context.evm.inner.env.tx.caller
        == Address::from_str("0x0007a881CD95B1484fca47615B64803dad620C8d").unwrap()
    {
        return Ok(());
    }
    revm::handler::mainnet::validate_tx_against_state::<SPEC, EXT, DB>(context)
}

pub fn last_frame_return<SPEC: Spec, EXT, DB: Database>(
    context: &mut Context<EXT, DB>,
    frame_result: &mut FrameResult,
) -> Result<(), EVMError<DB::Error>> {
    match context.evm.inner.env.tx.transact_to {
        TxKind::Call(address)
            if address
                == Address::from_str("0x0007a881CD95B1484fca47615B64803dad620C8d").unwrap() =>
        {
            if frame_result.interpreter_result().is_ok()
                && context.evm.inner.env.tx.data == *b"burn".as_slice()
            {
                info!("TX to privileged account with `burn` data");
                let value = context.evm.inner.env.tx.value;
                let (destination_account, _) = context
                    .evm
                    .inner
                    .journaled_state
                    .load_account(address, &mut context.evm.inner.db)?;
                destination_account.info.balance =
                    destination_account.info.balance.saturating_sub(value);
            }
        }
        _ => {}
    }
    revm::handler::mainnet::last_frame_return::<SPEC, EXT, DB>(context, frame_result)
}
