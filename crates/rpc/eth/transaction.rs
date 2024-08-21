use super::block::BlockIdentifier;
use crate::utils::RpcErr;
use ethereum_rust_core::{types::GenericTransaction, Bytes};
use ethereum_rust_evm::{evm_state, ExecutionResult, SpecId};
use ethereum_rust_storage::Store;
use serde_json::Value;
use tracing::info;

pub struct CallRequest {
    transaction: GenericTransaction,
    block: Option<BlockIdentifier>,
}

impl CallRequest {
    pub fn parse(params: &Option<Vec<Value>>) -> Option<CallRequest> {
        let params = params.as_ref()?;
        if params.len() != 2 {
            return None;
        };
        Some(CallRequest {
            transaction: serde_json::from_value(params[0].clone()).ok()?,
            block: serde_json::from_value(params[1].clone()).ok()?,
        })
    }
}

pub fn call(request: &CallRequest, storage: Store) -> Result<Value, RpcErr> {
    let block = request.block.clone().unwrap_or_default();
    info!("Requested call on block: {}", block);
    let block_number = match block.resolve_block_number(&storage)? {
        Some(block_number) => block_number,
        _ => return Ok(Value::Null),
    };
    let header = match storage.get_block_header(block_number)? {
        Some(header) => header,
        // Block not found
        _ => return Ok(Value::Null),
    };
    // Run transaction
    let data = match ethereum_rust_evm::simulate_tx_from_generic(
        &request.transaction,
        &header,
        &mut evm_state(storage),
        SpecId::CANCUN,
    )? {
        ExecutionResult::Success {
            reason: _,
            gas_used: _,
            gas_refunded: _,
            output,
        } => match output {
            ethereum_rust_evm::Output::Call(bytes) => bytes,
            ethereum_rust_evm::Output::Create(bytes, _) => bytes,
        },
        ExecutionResult::Revert {
            gas_used: _,
            output,
        } => {
            return Err(RpcErr::Revert {
                data: format!("0x{:#x}", output),
            });
        }
        ExecutionResult::Halt {
            reason: _,
            gas_used: _,
        } => Bytes::new(),
    };
    serde_json::to_value(format!("0x{:#x}", data)).map_err(|_| RpcErr::Internal)
}
