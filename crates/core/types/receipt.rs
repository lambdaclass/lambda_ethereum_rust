use crate::rlp::{
    decode::RLPDecode,
    encode::RLPEncode,
    error::RLPDecodeError,
    structs::{Decoder, Encoder},
};
use bytes::Bytes;
use ethereum_types::{Address, Bloom, H256};
use revm::primitives::Address as EvmAddress;
use serde::Serialize;

use super::{BlockHash, BlockNumber, TxKind, TxType};
pub type Index = u64;

/// Result of a transaction
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]

pub struct Receipt {
    #[serde(rename = "type")]
    pub tx_type: TxType,
    #[serde(with = "crate::serde_utils::bool", rename = "status")]
    pub succeeded: bool,
    #[serde(with = "crate::serde_utils::u64::hex_str")]
    pub cumulative_gas_used: u64,
    #[serde(rename = "logsBloom")]
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
}

impl RLPEncode for Receipt {
    fn encode(&self, buf: &mut dyn bytes::BufMut) {
        // tx_type || RLP(receipt)  if tx_type != 0
        //            RLP(receipt)  else
        match self.tx_type {
            TxType::Legacy => {}
            _ => buf.put_u8(self.tx_type as u8),
        }
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
#[serde(rename_all = "camelCase")]
pub struct ReceiptBlockInfo {
    pub block_hash: BlockHash,
    #[serde(with = "crate::serde_utils::u64::hex_str")]
    pub block_number: BlockNumber,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReceiptTxInfo {
    pub transaction_hash: H256,
    #[serde(with = "crate::serde_utils::u64::hex_str")]
    pub transaction_index: u64,
    pub from: Address,
    pub to: Option<Address>,
    pub contract_address: Option<Address>,
    #[serde(with = "crate::serde_utils::u64::hex_str")]
    pub gas_used: u64,
    #[serde(with = "crate::serde_utils::u64::hex_str")]
    pub effective_gas_price: u64,
}

impl ReceiptTxInfo {
    pub fn new(
        transaction_hash: H256,
        transaction_index: u64,
        nonce: u64,
        from: Address,
        tx_kind: TxKind,
        gas_used: u64,
        effective_gas_price: u64,
    ) -> ReceiptTxInfo {
        let (contract_address, to) = match tx_kind {
            TxKind::Create => (
                Some(Address::from_slice(
                    EvmAddress(from.0.into()).create(nonce).0.as_ref(),
                )),
                None,
            ),
            TxKind::Call(addr) => (None, Some(addr)),
        };
        Self {
            transaction_hash,
            transaction_index,
            from,
            to,
            contract_address,
            gas_used,
            effective_gas_price,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex_literal::hex;

    #[test]
    fn serialize_receipt() {
        let receipt = ReceiptWithTxAndBlockInfo {
            receipt: Receipt {
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
            tx_info: ReceiptTxInfo {
                transaction_hash: H256::zero(),
                transaction_index: 1,
                from: Address::zero(),
                to: Some(Address::from(hex!(
                    "7435ed30a8b4aeb0877cef0c6e8cffe834eb865f"
                ))),
                contract_address: None,
                gas_used: 147,
                effective_gas_price: 157,
            },
            block_info: ReceiptBlockInfo {
                block_hash: BlockHash::zero(),
                block_number: 3,
            },
        };
        let expected = r#"{"type":"0x3","status":"0x1","cumulativeGasUsed":"0x93","logsBloom":"0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000","logs":[{"address":"0x0000000000000000000000000000000000000000","topics":[],"data":"0x73747261776265727279"}],"transactionHash":"0x0000000000000000000000000000000000000000000000000000000000000000","transactionIndex":"0x1","from":"0x0000000000000000000000000000000000000000","to":"0x7435ed30a8b4aeb0877cef0c6e8cffe834eb865f","contractAddress":null,"gasUsed":"0x93","effectiveGasPrice":"0x9d","blockHash":"0x0000000000000000000000000000000000000000000000000000000000000000","blockNumber":"0x3"}"#;
        assert_eq!(serde_json::to_string(&receipt).unwrap(), expected);
    }
}
