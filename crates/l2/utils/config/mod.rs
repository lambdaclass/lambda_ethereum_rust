use keccak_hash::H256;
use libsecp256k1::SecretKey;
use serde::{Deserialize, Deserializer};

pub mod block_producer;
pub mod engine_api;
pub mod eth;
pub mod l1_tx_sender;
pub mod l1_watcher;
pub mod proof_data_provider;
pub mod prover;

fn secret_key_deserializer<'de, D>(deserializer: D) -> Result<SecretKey, D::Error>
where
    D: Deserializer<'de>,
{
    let hex = H256::deserialize(deserializer)?;
    SecretKey::parse(hex.as_fixed_bytes()).map_err(serde::de::Error::custom)
}
