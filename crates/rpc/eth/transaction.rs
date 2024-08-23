use super::block::BlockIdentifier;
use crate::{types::transaction::RpcTransaction, utils::RpcErr};
use ethereum_rust_core::{
    types::{AccessListEntry, BlockHash, GenericTransaction, ReceiptWithTxAndBlockInfo},
    Bytes, H256,
};

use ethereum_rust_storage::Store;

use ethereum_rust_evm::{evm_state, ExecutionResult, SpecId};
use serde::Serialize;

use serde_json::Value;
use tracing::info;

pub struct CallRequest {
    transaction: GenericTransaction,
    block: Option<BlockIdentifier>,
}

pub struct GetTransactionByBlockNumberAndIndexRequest {
    pub block: BlockIdentifier,
    pub transaction_index: usize,
}

pub struct GetTransactionByBlockHashAndIndexRequest {
    pub block: BlockHash,
    pub transaction_index: usize,
}

pub struct GetTransactionByHashRequest {
    pub transaction_hash: H256,
}

pub struct GetTransactionReceiptRequest {
    pub transaction_hash: H256,
}

pub struct CreateAccessListRequest {
    pub transaction: GenericTransaction,
    pub block: Option<BlockIdentifier>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccessListResult {
    access_list: Vec<AccessListEntry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(with = "ethereum_rust_core::serde_utils::u64::hex_str")]
    gas_used: u64,
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

impl GetTransactionByBlockNumberAndIndexRequest {
    pub fn parse(
        params: &Option<Vec<Value>>,
    ) -> Option<GetTransactionByBlockNumberAndIndexRequest> {
        let params = params.as_ref()?;
        if params.len() != 2 {
            return None;
        };
        let index_as_string: String = serde_json::from_value(params[1].clone()).ok()?;
        Some(GetTransactionByBlockNumberAndIndexRequest {
            block: serde_json::from_value(params[0].clone()).ok()?,
            transaction_index: usize::from_str_radix(index_as_string.trim_start_matches("0x"), 16)
                .ok()?,
        })
    }
}

impl GetTransactionByBlockHashAndIndexRequest {
    pub fn parse(params: &Option<Vec<Value>>) -> Option<GetTransactionByBlockHashAndIndexRequest> {
        let params = params.as_ref()?;
        if params.len() != 2 {
            return None;
        };
        let index_as_string: String = serde_json::from_value(params[1].clone()).ok()?;
        Some(GetTransactionByBlockHashAndIndexRequest {
            block: serde_json::from_value(params[0].clone()).ok()?,
            transaction_index: usize::from_str_radix(index_as_string.trim_start_matches("0x"), 16)
                .ok()?,
        })
    }
}

impl GetTransactionByHashRequest {
    pub fn parse(params: &Option<Vec<Value>>) -> Option<GetTransactionByHashRequest> {
        let params = params.as_ref()?;
        if params.len() != 1 {
            return None;
        };
        Some(GetTransactionByHashRequest {
            transaction_hash: serde_json::from_value(params[0].clone()).ok()?,
        })
    }
}

impl GetTransactionReceiptRequest {
    pub fn parse(params: &Option<Vec<Value>>) -> Option<GetTransactionReceiptRequest> {
        let params = params.as_ref()?;
        if params.len() != 1 {
            return None;
        };
        Some(GetTransactionReceiptRequest {
            transaction_hash: serde_json::from_value(params[0].clone()).ok()?,
        })
    }
}

impl CreateAccessListRequest {
    pub fn parse(params: &Option<Vec<Value>>) -> Option<CreateAccessListRequest> {
        let params = params.as_ref()?;
        if params.len() > 2 {
            return None;
        };
        let block = match params.get(1) {
            // Differentiate between missing and bad block param
            Some(value) => Some(serde_json::from_value(value.clone()).ok()?),
            None => None,
        };
        Some(CreateAccessListRequest {
            transaction: serde_json::from_value(params.first()?.clone()).ok()?,
            block,
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

pub fn get_transaction_by_block_number_and_index(
    request: &GetTransactionByBlockNumberAndIndexRequest,
    storage: Store,
) -> Result<Value, RpcErr> {
    info!(
        "Requested transaction at index: {} of block with number: {}",
        request.transaction_index, request.block,
    );
    let block_number = match request.block.resolve_block_number(&storage)? {
        Some(block_number) => block_number,
        _ => return Ok(Value::Null),
    };
    let block_body = match storage.get_block_body(block_number)? {
        Some(block_body) => block_body,
        _ => return Ok(Value::Null),
    };
    let block_header = match storage.get_block_header(block_number)? {
        Some(block_body) => block_body,
        _ => return Ok(Value::Null),
    };
    let tx = match block_body.transactions.get(request.transaction_index) {
        Some(tx) => tx,
        None => return Ok(Value::Null),
    };
    let tx = RpcTransaction::build(
        tx.clone(),
        block_number,
        block_header.compute_block_hash(),
        request.transaction_index,
    );
    serde_json::to_value(tx).map_err(|_| RpcErr::Internal)
}

pub fn get_transaction_by_block_hash_and_index(
    request: &GetTransactionByBlockHashAndIndexRequest,
    storage: Store,
) -> Result<Value, RpcErr> {
    info!(
        "Requested transaction at index: {} of block with hash: {}",
        request.transaction_index, request.block,
    );
    let block_number = match storage.get_block_number(request.block)? {
        Some(number) => number,
        _ => return Ok(Value::Null),
    };
    let block_body = match storage.get_block_body(block_number)? {
        Some(block_body) => block_body,
        _ => return Ok(Value::Null),
    };
    let tx = match block_body.transactions.get(request.transaction_index) {
        Some(tx) => tx,
        None => return Ok(Value::Null),
    };
    let tx = RpcTransaction::build(
        tx.clone(),
        block_number,
        request.block,
        request.transaction_index,
    );
    serde_json::to_value(tx).map_err(|_| RpcErr::Internal)
}

pub fn get_transaction_by_hash(
    request: &GetTransactionByHashRequest,
    storage: Store,
) -> Result<Value, RpcErr> {
    info!(
        "Requested transaction with hash: {}",
        request.transaction_hash,
    );
    let transaction: ethereum_rust_core::types::Transaction =
        match storage.get_transaction_by_hash(request.transaction_hash)? {
            Some(transaction) => transaction,
            _ => return Ok(Value::Null),
        };
    let (block_number, index) = match storage.get_transaction_location(request.transaction_hash)? {
        Some(location) => location,
        _ => return Ok(Value::Null),
    };
    let block_header = match storage.get_block_header(block_number)? {
        Some(header) => header,
        _ => return Ok(Value::Null),
    };
    let block_hash = block_header.compute_block_hash();
    let transaction = RpcTransaction::build(transaction, block_number, block_hash, index as usize);
    serde_json::to_value(transaction).map_err(|_| RpcErr::Internal)
}

pub fn get_transaction_receipt(
    request: &GetTransactionReceiptRequest,
    storage: Store,
) -> Result<Value, RpcErr> {
    info!(
        "Requested receipt for transaction {}",
        request.transaction_hash,
    );
    let (block_number, index) = match storage.get_transaction_location(request.transaction_hash)? {
        Some(location) => location,
        _ => return Ok(Value::Null),
    };
    let block_header = match storage.get_block_header(block_number)? {
        Some(block_header) => block_header,
        _ => return Ok(Value::Null),
    };
    let block_body = match storage.get_block_body(block_number)? {
        Some(block_body) => block_body,
        _ => return Ok(Value::Null),
    };
    let receipt = match storage.get_receipt(block_number, index)? {
        Some(receipt) => receipt,
        _ => return Ok(Value::Null),
    };
    let tx = match index
        .try_into()
        .ok()
        .and_then(|index: usize| block_body.transactions.get(index))
    {
        Some(tx) => tx,
        _ => return Ok(Value::Null),
    };
    let block_info = block_header.receipt_info();
    let tx_info = tx.receipt_info(index);
    let receipt = ReceiptWithTxAndBlockInfo {
        receipt,
        tx_info,
        block_info,
    };
    serde_json::to_value(&receipt).map_err(|_| RpcErr::Internal)
}

pub fn create_access_list(
    request: &CreateAccessListRequest,
    storage: Store,
) -> Result<Value, RpcErr> {
    let block = request.block.clone().unwrap_or_default();
    info!("Requested access list creation for tx on block: {}", block);
    let block_number = match block.resolve_block_number(&storage)? {
        Some(block_number) => block_number,
        _ => return Ok(Value::Null),
    };
    let header = match storage.get_block_header(block_number)? {
        Some(header) => header,
        // Block not found
        _ => return Ok(Value::Null),
    };
    // Run transaction and obtain access list
    let (gas_used, access_list, error) = match ethereum_rust_evm::create_access_list(
        &request.transaction,
        &header,
        &mut evm_state(storage),
        SpecId::CANCUN,
    )? {
        (
            ExecutionResult::Success {
                reason: _,
                gas_used,
                gas_refunded: _,
                output: _,
            },
            access_list,
        ) => (gas_used, access_list, None),
        (
            ExecutionResult::Revert {
                gas_used,
                output: _,
            },
            access_list,
        ) => (
            gas_used,
            access_list,
            Some("Transaction Reverted".to_string()),
        ),
        (ExecutionResult::Halt { reason, gas_used }, access_list) => {
            (gas_used, access_list, Some(reason))
        }
    };
    let result = AccessListResult {
        access_list: access_list
            .into_iter()
            .map(|(address, storage_keys)| AccessListEntry {
                address,
                storage_keys,
            })
            .collect(),
        error,
        gas_used,
    };

    serde_json::to_value(result).map_err(|_| RpcErr::Internal)
}
