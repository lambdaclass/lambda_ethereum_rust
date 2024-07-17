use crate::rlp::{encode::RLPEncode, structs::Encoder};
use bytes::Bytes;
use ethereum_types::{Address, Bloom, H256};
use serde::Serialize;

use super::{BlockHash, BlockNumber, TxKind, TxType};
pub type Index = u64;

/// Result of a transaction
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Receipt {
    pub tx_type: TxType,
    #[serde(with = "crate::serde_utils::bool")]
    pub succeeded: bool,
    #[serde(with = "crate::serde_utils::u64::hex_str")]
    pub cumulative_gas_used: u64,
    pub bloom: Bloom,
    pub logs: Vec<Log>,
}

impl Receipt {
    pub fn new(
        tx_type: TxType,
        succeeded: bool,
        cumulative_gas_used: u64,
        bloom: Bloom,
        logs: Vec<Log>,
    ) -> Self {
        Self {
            tx_type,
            succeeded,
            cumulative_gas_used,
            bloom,
            logs,
        }
    }

    pub fn encode_with_type(&self, buf: &mut dyn bytes::BufMut) {
        // tx_type || RLP(receipt)  if tx_type != 0
        //            RLP(receipt)  else
        match self.tx_type {
            TxType::Legacy => {}
            _ => buf.put_u8(self.tx_type as u8),
        }
        self.encode(buf);
    }
}

impl RLPEncode for Receipt {
    fn encode(&self, buf: &mut dyn bytes::BufMut) {
        Encoder::new(buf)
            .encode_field(&self.succeeded)
            .encode_field(&self.cumulative_gas_used)
            .encode_field(&self.bloom)
            .encode_field(&self.logs)
            .finish();
    }
}

/// Data record produced during the execution of a transaction.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Log {
    address: Address,
    topics: Vec<H256>,
    #[serde(with = "crate::serde_utils::bytes")]
    data: Bytes,
}

impl RLPEncode for Log {
    fn encode(&self, buf: &mut dyn bytes::BufMut) {
        Encoder::new(buf)
            .encode_field(&self.address)
            .encode_field(&self.topics)
            .encode_field(&self.data)
            .finish();
    }
}

// Struct used by RPC
#[derive(Debug, Serialize)]
pub struct ReceiptWithTxAndBlockInfo {
    #[serde(flatten)]
    receipt: Receipt,
    #[serde(flatten)]
    tx_info: ReceiptTxInfo,
    #[serde(flatten)]
    block_info: ReceiptBlockInfo,
}

#[derive(Debug, Serialize)]
pub struct ReceiptBlockInfo {
    pub block_hash: BlockHash,
    #[serde(with = "crate::serde_utils::u64::hex_str")]
    pub block_number: BlockNumber,
    #[serde(with = "crate::serde_utils::u64::hex_str")]
    pub gas_used: u64,
    #[serde(with = "crate::serde_utils::u64::hex_str")]
    pub blob_gas_used: u64,
    pub root: H256, // state root
}

#[derive(Debug, Serialize)]
pub struct ReceiptTxInfo {
    pub transaction_hash: H256,
    #[serde(with = "crate::serde_utils::u64::hex_str")]
    pub transaction_index: u64,
    pub from: Address,
    pub to: TxKind,
    #[serde(with = "crate::serde_utils::u64::hex_str")]
    pub effective_gas_price: u64,
    #[serde(with = "crate::serde_utils::u64::hex_str_opt")]
    pub blob_gas_price: Option<u64>,
}
