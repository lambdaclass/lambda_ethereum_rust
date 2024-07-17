use crate::rlp::{
    decode::RLPDecode,
    encode::RLPEncode,
    error::RLPDecodeError,
    structs::{Decoder, Encoder},
};
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

impl RLPDecode for Receipt {
    fn decode_unfinished(rlp: &[u8]) -> Result<(Self, &[u8]), RLPDecodeError> {
        // Decode tx type
        let (tx_type, rlp) = match rlp.first() {
            Some(tx_type) if *tx_type < 0x7f => match tx_type {
                0x0 => (TxType::Legacy, &rlp[1..]),
                0x1 => (TxType::EIP2930, &rlp[1..]),
                0x2 => (TxType::EIP1559, &rlp[1..]),
                0x3 => (TxType::EIP4844, &rlp[1..]),
                ty => {
                    return Err(RLPDecodeError::Custom(format!(
                        "Invalid transaction type: {ty}"
                    )))
                }
            },
            // Legacy Tx
            _ => (TxType::Legacy, rlp),
        };
        // Decode the remaining fields
        let decoder = Decoder::new(rlp)?;
        let (succeeded, decoder) = decoder.decode_field("succeeded")?;
        let (cumulative_gas_used, decoder) = decoder.decode_field("cumulative_gas_used")?;
        let (bloom, decoder) = decoder.decode_field("bloom")?;
        let (logs, decoder) = decoder.decode_field("logs")?;
        let receipt = Receipt {
            tx_type,
            succeeded,
            cumulative_gas_used,
            bloom,
            logs,
        };
        Ok((receipt, decoder.finish()?))
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

impl RLPDecode for Log {
    fn decode_unfinished(rlp: &[u8]) -> Result<(Self, &[u8]), RLPDecodeError> {
        let decoder = Decoder::new(rlp)?;
        let (address, decoder) = decoder.decode_field("address")?;
        let (topics, decoder) = decoder.decode_field("topics")?;
        let (data, decoder) = decoder.decode_field("data")?;
        let log = Log {
            address,
            topics,
            data,
        };
        Ok((log, decoder.finish()?))
    }
}

// Struct used by RPC
#[derive(Debug, Serialize)]
pub struct ReceiptWithTxAndBlockInfo {
    #[serde(flatten)]
    pub receipt: Receipt,
    #[serde(flatten)]
    pub tx_info: ReceiptTxInfo,
    #[serde(flatten)]
    pub block_info: ReceiptBlockInfo,
}

#[derive(Debug, Serialize, Clone)]
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
