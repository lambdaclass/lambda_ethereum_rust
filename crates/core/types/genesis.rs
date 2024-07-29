use bytes::Bytes;
use ethereum_types::{Address, Bloom, H256, U256};
use patricia_merkle_tree::PatriciaMerkleTree;
use serde::Deserialize;
use sha3::{Digest, Keccak256};
use std::collections::{BTreeMap, HashMap};

use crate::rlp::encode::RLPEncode as _;

use super::{
    code_hash, AccountInfo, AccountState, Block, BlockBody, BlockHeader, DEFAULT_OMMERS_HASH,
};

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
    #[serde(default, with = "crate::serde_utils::bytes")]
    pub code: Bytes,
    #[serde(default)]
    pub storage: BTreeMap<H256, H256>,
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
            ommers_hash: *DEFAULT_OMMERS_HASH,
            coinbase: self.coinbase,
            state_root: self.compute_state_root(),
            transactions_root: H256::zero(),
            receipt_root: H256::zero(),
            logs_bloom: Bloom::zero(),
            difficulty: self.difficulty,
            number: 0,
            gas_limit: self.gas_limit,
            gas_used: 0,
            timestamp: self.timestamp,
            extra_data: Bytes::new(),
            prev_randao: self.mixhash,
            nonce: self.nonce,
            base_fee_per_gas: 0,
            withdrawals_root: None,
            blob_gas_used: None,
            excess_blob_gas: None,
            parent_beacon_block_root: None,
        }
    }

    fn get_block_body(&self) -> BlockBody {
        BlockBody {
            transactions: vec![],
            ommers: vec![],
            withdrawals: None,
        }
    }

    pub fn compute_state_root(&self) -> H256 {
        let mut pmt = PatriciaMerkleTree::<Vec<u8>, Vec<u8>, Keccak256>::new();

        for (address, genesis_account) in self.alloc.iter() {
            // Key: Keccak(address)
            let k = Keccak256::new_with_prefix(address.to_fixed_bytes())
                .finalize()
                .to_vec();

            let info = AccountInfo {
                code_hash: code_hash(&genesis_account.code),
                balance: genesis_account.balance,
                nonce: genesis_account.nonce,
            };

            // Value: account
            let mut v = Vec::new();
            AccountState::from_info_and_storage(&info, &genesis_account.storage).encode(&mut v);
            pmt.insert(k, v);
        }
        // TODO check if sorting by key and using
        // PatriciaMerkleTree::<_, _, Keccak256>::compute_hash_from_sorted_iter is more efficient

        let &root = pmt.compute_hash();
        H256(root.into())
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

    #[test]
    fn genesis_block() {
        // Deserialize genesis file
        let file = File::open("../../test_data/genesis.json").expect("Failed to open genesis file");
        let reader = BufReader::new(file);
        let genesis: Genesis =
            serde_json::from_reader(reader).expect("Failed to deserialize genesis file");
        let genesis_block = genesis.get_block();
        let header = genesis_block.header;
        let body = genesis_block.body;
        assert_eq!(header.parent_hash, H256::from([0; 32]));
        assert_eq!(header.ommers_hash, *DEFAULT_OMMERS_HASH);
        assert_eq!(header.coinbase, Address::default());
        assert_eq!(
            header.state_root,
            H256::from_str("0x2dab6a1d6d638955507777aecea699e6728825524facbd446bd4e86d44fa5ecd")
                .unwrap()
        );
        assert_eq!(header.transactions_root, H256::from([0; 32]));
        assert_eq!(header.receipt_root, H256::from([0; 32]));
        assert_eq!(header.logs_bloom, Bloom::default());
        assert_eq!(header.difficulty, U256::from(1));
        assert_eq!(header.gas_limit, 25_000_000);
        assert_eq!(header.gas_used, 0);
        assert_eq!(header.timestamp, 1_718_040_081);
        assert_eq!(header.extra_data, Bytes::default());
        assert_eq!(header.prev_randao, H256::from([0; 32]));
        assert_eq!(header.nonce, 4660);
        assert_eq!(header.base_fee_per_gas, 0);
        assert_eq!(header.withdrawals_root, None);
        assert_eq!(header.blob_gas_used, None);
        assert_eq!(header.excess_blob_gas, None);
        assert_eq!(header.parent_beacon_block_root, None);
        assert!(body.transactions.is_empty());
        assert!(body.ommers.is_empty());
        assert_eq!(body.withdrawals, None);
    }
}
