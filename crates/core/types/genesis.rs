use bytes::Bytes;
use ethereum_types::{Address, Bloom, H256, U256};
use serde::Deserialize;
use std::collections::HashMap;

use super::{Block, BlockBody, BlockHeader};

#[allow(unused)]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Genesis {
    /// Chain configuration
    pub config: ChainConfig,
    /// The initial state of the accounts in the genesis block.
    pub alloc: HashMap<Address, GenesisAccount>,
    /// Genesis header values
    pub coinbase: Address,
    pub difficulty: U256,
    pub extra_data: Bytes,
    #[serde(with = "crate::serde_utils::u64::hex_str")]
    pub gas_limit: u64,
    #[serde(with = "crate::serde_utils::u64::hex_str")]
    pub nonce: u64,
    pub mixhash: H256,
    #[serde(deserialize_with = "crate::serde_utils::u64::deser_dec_str")]
    pub timestamp: u64,
}

/// Blockchain settings defined per block
#[allow(unused)]
#[derive(Debug, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ChainConfig {
    /// Current chain identifier
    #[serde(deserialize_with = "crate::serde_utils::u256::deser_number")]
    pub chain_id: U256,

    /// Block numbers for the block where each fork was activated
    /// (None = no fork, 0 = fork is already active)
    pub homestead_block: Option<u64>,

    pub dao_fork_block: Option<u64>,
    /// Whether the node supports or opposes the DAO hard-fork
    #[serde(default)]
    pub dao_fork_support: bool,

    pub eip150_block: Option<u64>,
    pub eip155_block: Option<u64>,
    pub eip158_block: Option<u64>,

    pub byzantium_block: Option<u64>,
    pub constantinople_block: Option<u64>,
    pub petersburg_block: Option<u64>,
    pub istanbul_block: Option<u64>,
    pub muir_glacier_block: Option<u64>,
    pub berlin_block: Option<u64>,
    pub london_block: Option<u64>,
    pub arrow_glacier_block: Option<u64>,
    pub gray_glacier_block: Option<u64>,
    pub merge_netsplit_block: Option<u64>,

    /// Timestamp at which each fork was activated
    /// (None = no fork, 0 = fork is already active)
    pub shanghai_time: Option<u64>,
    pub cancun_time: Option<u64>,
    pub prague_time: Option<u64>,
    pub verkle_time: Option<u64>,

    /// Amount of total difficulty reached by the network that triggers the consensus upgrade.
    #[serde(
        default,
        deserialize_with = "crate::serde_utils::u256::deser_number_opt"
    )]
    pub terminal_total_difficulty: Option<U256>,
    /// Network has already passed the terminal total difficult
    #[serde(default)]
    pub terminal_total_difficulty_passed: bool,
}

#[allow(unused)]
#[derive(Debug, Deserialize, PartialEq)]
pub struct GenesisAccount {
    #[serde(default)]
    pub code: Bytes,
    #[serde(default)]
    pub storage: HashMap<H256, H256>,
    #[serde(deserialize_with = "crate::serde_utils::u256::deser_dec_str")]
    pub balance: U256,
    #[serde(default, deserialize_with = "crate::serde_utils::u64::deser_dec_str")]
    pub nonce: u64,
}

impl Genesis {
    pub fn get_block(&self) -> Block {
        let header = self.get_block_header();
        let body = self.get_block_body();
        Block { header, body }
    }

    fn get_block_header(&self) -> BlockHeader {
        BlockHeader {
            parent_hash: H256::zero(),
            ommers_hash: H256::zero(), // TODO
            coinbase: self.coinbase,
            state_root: H256::zero(),        // TODO
            transactions_root: H256::zero(), // TODO
            receipt_root: H256::zero(),      // TODO
            logs_bloom: Bloom::default(),    // TODO
            difficulty: self.difficulty,
            number: 0,
            gas_limit: self.gas_limit,
            gas_used: 0, // TODO
            timestamp: self.timestamp,
            extra_data: Bytes::default(), // TODO
            prev_randao: self.mixhash,
            nonce: self.nonce,
            base_fee_per_gas: 0,                    // TODO
            withdrawals_root: H256::zero(),         // TODO
            blob_gas_used: 0,                       // TODO
            excess_blob_gas: 0,                     // TODO
            parent_beacon_block_root: H256::zero(), // TODO
        }
    }

    fn get_block_body(&self) -> BlockBody {
        BlockBody {
            transactions: vec![], // TODO
            ommers: vec![],       // TODO
            withdrawals: vec![],  // TODO
        }
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;
    use std::{fs::File, io::BufReader};

    use super::*;

    #[test]
    fn deserialize_genesis_file() {
        // Deserialize genesis file
        let file = File::open("../../test_data/genesis.json").expect("Failed to open genesis file");
        let reader = BufReader::new(file);
        let genesis: Genesis =
            serde_json::from_reader(reader).expect("Failed to deserialize genesis file");
        // Check Genesis fields
        // Chain config
        let expected_chain_config = ChainConfig {
            chain_id: U256::from(3151908),
            homestead_block: Some(0),
            eip150_block: Some(0),
            eip155_block: Some(0),
            eip158_block: Some(0),
            byzantium_block: Some(0),
            constantinople_block: Some(0),
            petersburg_block: Some(0),
            istanbul_block: Some(0),
            berlin_block: Some(0),
            london_block: Some(0),
            merge_netsplit_block: Some(0),
            shanghai_time: Some(0),
            cancun_time: Some(0),
            prague_time: Some(1718232101),
            terminal_total_difficulty: Some(U256::from(0)),
            terminal_total_difficulty_passed: true,
            ..Default::default()
        };
        assert_eq!(&genesis.config, &expected_chain_config);
        // Genesis header fields
        assert_eq!(genesis.coinbase, Address::from([0; 20]));
        assert_eq!(genesis.difficulty, U256::from(1));
        assert!(genesis.extra_data.is_empty());
        assert_eq!(genesis.gas_limit, 0x17d7840);
        assert_eq!(genesis.nonce, 0x1234);
        assert_eq!(genesis.mixhash, H256::from([0; 32]));
        assert_eq!(genesis.timestamp, 1718040081);
        // Check alloc field
        // We will only check a couple of the hashmap's values as it is quite large
        let addr_a = Address::from_str("0x000F3df6D732807Ef1319fB7B8bB8522d0Beac02").unwrap();
        assert!(genesis.alloc.contains_key(&addr_a));
        let expected_account_a = GenesisAccount {
        code: Bytes::from(String::from("0x3373fffffffffffffffffffffffffffffffffffffffe14604d57602036146024575f5ffd5b5f35801560495762001fff810690815414603c575f5ffd5b62001fff01545f5260205ff35b5f5ffd5b62001fff42064281555f359062001fff015500")),
        balance: 0.into(),
        nonce: 1,
        storage: Default::default(),
    };
        assert_eq!(genesis.alloc[&addr_a], expected_account_a);
        // Check some storage values from another account
        let addr_b = Address::from_str("0x4242424242424242424242424242424242424242").unwrap();
        assert!(genesis.alloc.contains_key(&addr_b));
        let addr_b_storage = &genesis.alloc[&addr_b].storage;
        assert_eq!(
            addr_b_storage.get(
                &H256::from_str(
                    "0x0000000000000000000000000000000000000000000000000000000000000022"
                )
                .unwrap()
            ),
            Some(
                &H256::from_str(
                    "0xf5a5fd42d16a20302798ef6ed309979b43003d2320d9f0e8ea9831a92759fb4b"
                )
                .unwrap()
            )
        );
        assert_eq!(
            addr_b_storage.get(
                &H256::from_str(
                    "0x0000000000000000000000000000000000000000000000000000000000000038"
                )
                .unwrap()
            ),
            Some(
                &H256::from_str(
                    "0xe71f0aa83cc32edfbefa9f4d3e0174ca85182eec9f3a09f6a6c0df6377a510d7"
                )
                .unwrap()
            )
        );
    }
}
