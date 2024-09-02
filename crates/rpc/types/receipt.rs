use ethereum_rust_core::{
    types::{BlockHash, BlockHeader, BlockNumber, Log, Receipt, Transaction, TxKind, TxType},
    Address, H256,
};
use ethereum_rust_evm::RevmAddress;

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct RpcReceipt {
    #[serde(flatten)]
    pub receipt: Receipt,
    pub logs: Vec<RpcLog>,
    #[serde(flatten)]
    pub tx_info: RpcReceiptTxInfo,
    #[serde(flatten)]
    pub block_info: RpcReceiptBlockInfo,
}

impl RpcReceipt {
    pub fn new(
        receipt: Receipt,
        tx_info: RpcReceiptTxInfo,
        block_info: RpcReceiptBlockInfo,
        init_log_index: u64,
    ) -> Self {
        let mut logs = vec![];
        let mut log_index = init_log_index;
        for log in receipt.logs.clone() {
            logs.push(RpcLog {
                log,
                log_index,
                removed: false,
                transaction_hash: tx_info.transaction_hash,
                transaction_index: tx_info.transaction_index,
                block_hash: block_info.block_hash,
                block_number: block_info.block_number,
            });
            log_index += 1;
        }
        Self {
            receipt,
            logs,
            tx_info,
            block_info,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcLog {
    #[serde(flatten)]
    pub log: Log,
    #[serde(with = "ethereum_rust_core::serde_utils::u64::hex_str")]
    pub log_index: u64,
    pub removed: bool,
    pub transaction_hash: H256,
    #[serde(with = "ethereum_rust_core::serde_utils::u64::hex_str")]
    pub transaction_index: u64,
    pub block_hash: BlockHash,
    #[serde(with = "ethereum_rust_core::serde_utils::u64::hex_str")]
    pub block_number: BlockNumber,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RpcReceiptBlockInfo {
    pub block_hash: BlockHash,
    #[serde(with = "ethereum_rust_core::serde_utils::u64::hex_str")]
    pub block_number: BlockNumber,
    #[serde(
        skip_serializing_if = "Option::is_none",
        with = "ethereum_rust_core::serde_utils::u64::hex_str_opt"
    )]
    pub blob_gas_used: Option<u64>,
}

impl RpcReceiptBlockInfo {
    pub fn from_block_header(block_header: BlockHeader) -> Self {
        RpcReceiptBlockInfo {
            block_hash: block_header.compute_block_hash(),
            block_number: block_header.number,
            blob_gas_used: block_header.blob_gas_used.and_then(|val| {
                if val == 0 {
                    None
                } else {
                    Some(val)
                }
            }),
        }
    }
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcReceiptTxInfo {
    pub transaction_hash: H256,
    #[serde(with = "ethereum_rust_core::serde_utils::u64::hex_str")]
    pub transaction_index: u64,
    pub from: Address,
    pub to: Option<Address>,
    pub contract_address: Option<Address>,
    #[serde(with = "ethereum_rust_core::serde_utils::u64::hex_str")]
    pub gas_used: u64,
    #[serde(with = "ethereum_rust_core::serde_utils::u64::hex_str")]
    pub effective_gas_price: u64,
    #[serde(
        skip_serializing_if = "Option::is_none",
        with = "ethereum_rust_core::serde_utils::u64::hex_str_opt"
    )]
    pub blob_gas_price: Option<u64>,
}

impl RpcReceiptTxInfo {
    pub fn from_transaction(
        transaction: Transaction,
        index: u64,
        gas_used: u64,
        block_blob_gas_price: u64,
    ) -> Self {
        let nonce = transaction.nonce();
        let from = transaction.sender();
        let blob_gas_price = if transaction.tx_type() == TxType::EIP4844 {
            Some(block_blob_gas_price)
        } else {
            None
        };
        let (contract_address, to) = match transaction.to() {
            TxKind::Create => (
                Some(Address::from_slice(
                    RevmAddress(from.0.into()).create(nonce).0.as_ref(),
                )),
                None,
            ),
            TxKind::Call(addr) => (None, Some(addr)),
        };
        Self {
            transaction_hash: transaction.compute_hash(),
            transaction_index: index,
            from,
            to,
            contract_address,
            gas_used,
            effective_gas_price: transaction.gas_price(),
            blob_gas_price,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ethereum_rust_core::{
        types::{Log, TxType},
        Bloom, Bytes,
    };
    use hex_literal::hex;

    #[test]
    fn serialize_receipt() {
        let receipt = RpcReceipt::new(
            Receipt {
                tx_type: TxType::EIP4844,
                succeeded: true,
                cumulative_gas_used: 147,
                bloom: Bloom::zero(),
                logs: vec![Log {
                    address: Address::zero(),
                    topics: vec![],
                    data: Bytes::from_static(b"strawberry"),
                }],
            },
            RpcReceiptTxInfo {
                transaction_hash: H256::zero(),
                transaction_index: 1,
                from: Address::zero(),
                to: Some(Address::from(hex!(
                    "7435ed30a8b4aeb0877cef0c6e8cffe834eb865f"
                ))),
                contract_address: None,
                gas_used: 147,
                effective_gas_price: 157,
                blob_gas_price: None,
            },
            RpcReceiptBlockInfo {
                block_hash: BlockHash::zero(),
                block_number: 3,
                blob_gas_used: None,
            },
            0,
        );
        let expected = r#"{"type":"0x3","status":"0x1","cumulativeGasUsed":"0x93","logsBloom":"0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000","logs":[{"address":"0x0000000000000000000000000000000000000000","topics":[],"data":"0x73747261776265727279","logIndex":"0x0","removed":false,"transactionHash":"0x0000000000000000000000000000000000000000000000000000000000000000","transactionIndex":"0x1","blockHash":"0x0000000000000000000000000000000000000000000000000000000000000000","blockNumber":"0x3"}],"transactionHash":"0x0000000000000000000000000000000000000000000000000000000000000000","transactionIndex":"0x1","from":"0x0000000000000000000000000000000000000000","to":"0x7435ed30a8b4aeb0877cef0c6e8cffe834eb865f","contractAddress":null,"gasUsed":"0x93","effectiveGasPrice":"0x9d","blockHash":"0x0000000000000000000000000000000000000000000000000000000000000000","blockNumber":"0x3"}"#;
        assert_eq!(serde_json::to_string(&receipt).unwrap(), expected);
    }
}
