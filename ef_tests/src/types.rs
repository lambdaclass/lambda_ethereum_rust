use bytes::Bytes;
use ethrex_core::types::{
    code_hash, Account as EthrexAccount, AccountInfo, EIP1559Transaction, LegacyTransaction,
    Transaction as EthrexTransacion,
};
use ethrex_core::{types::BlockHeader, Address, Bloom, H256, U256, U64};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TestUnit {
    #[serde(default, rename = "_info")]
    pub info: Option<serde_json::Value>,
    pub blocks: Vec<Block>,
    pub genesis_block_header: Header,
    #[serde(rename = "genesisRLP")]
    pub genesis_rlp: serde_json::Value,
    pub lastblockhash: serde_json::Value,
    pub network: serde_json::Value,
    pub post_state: serde_json::Value,
    pub pre: HashMap<Address, Account>,
    pub seal_engine: serde_json::Value,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub struct Account {
    pub balance: U256,
    pub code: Bytes,
    pub nonce: U256,
    pub storage: HashMap<U256, U256>,
}

#[derive(Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Env {
    pub current_coinbase: Address,
    pub current_difficulty: U256,
    pub current_gas_limit: U256,
    pub current_number: U256,
    pub current_timestamp: U256,
    pub current_base_fee: Option<U256>,
    pub previous_hash: Option<H256>,
    pub current_random: Option<H256>,
    pub current_beacon_root: Option<H256>,
    pub current_withdrawals_root: Option<H256>,
    pub parent_blob_gas_used: Option<U256>,
    pub parent_excess_blob_gas: Option<U256>,
    pub current_excess_blob_gas: Option<U256>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AccessListItem {
    pub address: Address,
    pub storage_keys: Vec<H256>,
}

pub type AccessList = Vec<AccessListItem>;

#[derive(Debug, PartialEq, Eq, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Header {
    pub bloom: Bloom,
    pub coinbase: Address,
    pub difficulty: U256,
    pub extra_data: Bytes,
    pub gas_limit: U256,
    pub gas_used: U256,
    pub hash: H256,
    pub mix_hash: H256,
    pub nonce: U64,
    pub number: U256,
    pub parent_hash: H256,
    pub receipt_trie: H256,
    pub state_root: H256,
    pub timestamp: U256,
    pub transactions_trie: H256,
    pub uncle_hash: H256,
    pub base_fee_per_gas: Option<U256>,
    pub withdrawals_root: Option<H256>,
    pub blob_gas_used: Option<U256>,
    pub excess_blob_gas: Option<U256>,
    pub parent_beacon_block_root: Option<H256>,
    pub requests_root: Option<H256>,
}

#[derive(Debug, PartialEq, Eq, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Block {
    pub block_header: Option<Header>,
    pub rlp: Bytes,
    pub transactions: Option<Vec<Transaction>>,
    pub uncle_headers: Option<Vec<Header>>,
}

#[derive(Debug, PartialEq, Eq, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Transaction {
    #[serde(rename = "type")]
    pub transaction_type: Option<U256>,
    #[serde(deserialize_with = "deser_prefixed_bytes")]
    pub data: Bytes,
    pub gas_limit: U256,
    pub gas_price: Option<U256>,
    pub nonce: U256,
    pub r: U256,
    pub s: U256,
    pub v: U256,
    pub value: U256,
    pub chain_id: Option<U256>,
    pub access_list: Option<AccessList>,
    pub max_fee_per_gas: Option<U256>,
    pub max_priority_fee_per_gas: Option<U256>,
    pub hash: Option<H256>,
    pub sender: Address,
    pub to: Address,
}

// Conversions between EFtests & Ethrex types

impl Into<BlockHeader> for Header {
    fn into(self) -> BlockHeader {
        BlockHeader {
            parent_hash: self.parent_hash,
            ommers_hash: self.uncle_hash,
            coinbase: self.coinbase,
            state_root: self.state_root,
            transactions_root: self.transactions_trie,
            receipt_root: self.receipt_trie,
            logs_bloom: self.bloom.into(),
            difficulty: self.difficulty,
            number: self.number.as_u64(),
            gas_limit: self.gas_limit.as_u64(),
            gas_used: self.gas_used.as_u64(),
            timestamp: self.timestamp.as_u64(),
            extra_data: self.extra_data,
            prev_randao: self.mix_hash,
            nonce: self.nonce.as_u64(),
            base_fee_per_gas: self.base_fee_per_gas.unwrap().as_u64(),
            withdrawals_root: self.withdrawals_root.unwrap(),
            blob_gas_used: self.blob_gas_used.unwrap().as_u64(),
            excess_blob_gas: self.excess_blob_gas.unwrap().as_u64(),
            parent_beacon_block_root: self.parent_beacon_block_root.unwrap(),
        }
    }
}

impl Into<EthrexTransacion> for Transaction {
    fn into(self) -> EthrexTransacion {
        dbg!(&self);
        match self.transaction_type {
            Some(tx_type) => match tx_type.as_u64() {
                2 => EthrexTransacion::EIP1559Transaction(self.into()),
                _ => unimplemented!(),
            },
            None => EthrexTransacion::LegacyTransaction(self.into()),
        }
    }
}

impl Into<EIP1559Transaction> for Transaction {
    fn into(self) -> EIP1559Transaction {
        EIP1559Transaction {
            // Note: gas_price is not used in this conversion as it is not part of EIP1559Transaction, this could be a problem
            chain_id: self.chain_id.map(|id| id.as_u64()).unwrap_or(1 /*mainnet*/), // TODO: Consider converting this into Option
            signer_nonce: self.nonce.as_u64(),
            max_priority_fee_per_gas: self.max_priority_fee_per_gas.unwrap_or_default().as_u64(), // TODO: Consider converting this into Option
            max_fee_per_gas: self
                .max_fee_per_gas
                .unwrap_or(self.gas_price.unwrap_or_default())
                .as_u64(), // TODO: Consider converting this into Option
            gas_limit: self.gas_limit.as_u64(),
            destination: self.to,
            amount: self.value,
            payload: self.data,
            access_list: self
                .access_list
                .unwrap_or_default()
                .into_iter()
                .map(|item| (item.address, item.storage_keys))
                .collect(),
            signature_y_parity: self.v.as_u64().saturating_sub(27) != 0,
            signature_r: self.r,
            signature_s: self.s,
        }
    }
}

impl Into<LegacyTransaction> for Transaction {
    fn into(self) -> LegacyTransaction {
        LegacyTransaction {
            nonce: self.nonce.as_u64(),
            gas_price: self.gas_price.unwrap_or_default().as_u64(), // TODO: Consider converting this into Option
            gas: self.gas_limit.as_u64(),
            to: self.to,
            value: self.value,
            data: self.data,
            v: self.v,
            r: self.r,
            s: self.s,
        }
    }
}

impl Into<EthrexAccount> for Account {
    fn into(self) -> EthrexAccount {
        EthrexAccount {
            info: AccountInfo {
                code_hash: code_hash(&self.code),
                balance: self.balance,
                nonce: self.nonce.as_u64(),
            },
            code: self.code,
            storage: self
                .storage
                .into_iter()
                .map(|(k, v)| {
                    let mut k_bytes = [0; 32];
                    let mut v_bytes = [0; 32];
                    k.to_big_endian(&mut k_bytes);
                    v.to_big_endian(&mut v_bytes);
                    (H256(k_bytes), H256(v_bytes))
                })
                .collect(),
        }
    }
}

// Serde utils

pub fn deser_prefixed_bytes<'de, D>(d: D) -> Result<Bytes, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(d)?;
        Ok(Bytes::from(value.trim_start_matches("0x").to_string()))
    }
