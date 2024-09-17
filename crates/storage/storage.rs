#[cfg(feature = "in_memory")]
use self::engines::in_memory::Store as InMemoryStore;
#[cfg(feature = "libmdbx")]
use self::engines::libmdbx::Store as LibmdbxStore;
use self::error::StoreError;
use bytes::Bytes;
use engines::api::StoreEngine;
use ethereum_rust_core::rlp::decode::RLPDecode;
use ethereum_rust_core::rlp::encode::RLPEncode;
use ethereum_rust_core::types::{
    code_hash, AccountInfo, AccountState, Block, BlockBody, BlockHash, BlockHeader, BlockNumber,
    ChainConfig, Genesis, GenesisAccount, Index, Receipt, Transaction,
};
use ethereum_types::{Address, H256, U256};
use sha3::{Digest as _, Keccak256};
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::{Arc, Mutex};
use tracing::info;

mod engines;
pub mod error;
mod rlp;
mod trie;

#[derive(Debug, Clone)]
pub struct Store {
    // TODO: Check if we can remove this mutex and move it to the in_memory::Store struct
    engine: Arc<Mutex<dyn StoreEngine>>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub enum EngineType {
    #[cfg(feature = "in_memory")]
    InMemory,
    #[cfg(feature = "libmdbx")]
    Libmdbx,
}

#[derive(Default)]
pub struct AccountUpdate {
    pub address: Address,
    pub removed: bool,
    pub info: Option<AccountInfo>,
    pub code: Option<Bytes>,
    pub added_storage: HashMap<H256, U256>,
    // Matches TODO in code
    // removed_storage_keys: Vec<H256>,
}

impl AccountUpdate {
    /// Creates new empty update for the given account
    pub fn new(address: Address) -> AccountUpdate {
        AccountUpdate {
            address,
            ..Default::default()
        }
    }

    /// Creates new update representing an account removal
    pub fn removed(address: Address) -> AccountUpdate {
        AccountUpdate {
            address,
            removed: true,
            ..Default::default()
        }
    }
}

impl Store {
    pub fn new(path: &str, engine_type: EngineType) -> Result<Self, StoreError> {
        info!("Starting storage engine ({engine_type:?})");
        let store = match engine_type {
            #[cfg(feature = "libmdbx")]
            EngineType::Libmdbx => Self {
                engine: Arc::new(Mutex::new(LibmdbxStore::new(path)?)),
            },
            #[cfg(feature = "in_memory")]
            EngineType::InMemory => Self {
                engine: Arc::new(Mutex::new(InMemoryStore::new()?)),
            },
        };
        info!("Started store engine");
        Ok(store)
    }

    pub fn get_account_info(
        &self,
        block_number: BlockNumber,
        address: Address,
    ) -> Result<Option<AccountInfo>, StoreError> {
        let Some(state_trie) = self.engine.lock().unwrap().state_trie(block_number)? else {
            return Ok(None);
        };
        let hashed_address = hash_address(&address);
        let Some(encoded_state) = state_trie.get(&hashed_address)? else {
            return Ok(None);
        };
        let account_state = AccountState::decode(&encoded_state)?;
        Ok(Some(AccountInfo {
            code_hash: account_state.code_hash,
            balance: account_state.balance,
            nonce: account_state.nonce,
        }))
    }

    pub fn add_block_header(
        &self,
        block_number: BlockNumber,
        block_header: BlockHeader,
    ) -> Result<(), StoreError> {
        self.engine
            .clone()
            .lock()
            .unwrap()
            .add_block_header(block_number, block_header)
    }

    pub fn get_block_header(
        &self,
        block_number: BlockNumber,
    ) -> Result<Option<BlockHeader>, StoreError> {
        self.engine
            .clone()
            .lock()
            .unwrap()
            .get_block_header(block_number)
    }

    pub fn add_block_body(
        &self,
        block_number: BlockNumber,
        block_body: BlockBody,
    ) -> Result<(), StoreError> {
        self.engine
            .clone()
            .lock()
            .unwrap()
            .add_block_body(block_number, block_body)
    }

    pub fn get_block_body(
        &self,
        block_number: BlockNumber,
    ) -> Result<Option<BlockBody>, StoreError> {
        self.engine
            .clone()
            .lock()
            .unwrap()
            .get_block_body(block_number)
    }

    pub fn add_block_number(
        &self,
        block_hash: BlockHash,
        block_number: BlockNumber,
    ) -> Result<(), StoreError> {
        self.engine
            .clone()
            .lock()
            .unwrap()
            .add_block_number(block_hash, block_number)
    }

    pub fn get_block_number(
        &self,
        block_hash: BlockHash,
    ) -> Result<Option<BlockNumber>, StoreError> {
        self.engine
            .clone()
            .lock()
            .unwrap()
            .get_block_number(block_hash)
    }

    pub fn add_transaction_location(
        &self,
        transaction_hash: H256,
        block_number: BlockNumber,
        index: Index,
    ) -> Result<(), StoreError> {
        self.engine
            .lock()
            .unwrap()
            .add_transaction_location(transaction_hash, block_number, index)
    }

    pub fn get_transaction_location(
        &self,
        transaction_hash: H256,
    ) -> Result<Option<(BlockNumber, Index)>, StoreError> {
        self.engine
            .lock()
            .unwrap()
            .get_transaction_location(transaction_hash)
    }

    fn add_account_code(&self, code_hash: H256, code: Bytes) -> Result<(), StoreError> {
        self.engine
            .clone()
            .lock()
            .unwrap()
            .add_account_code(code_hash, code)
    }

    pub fn get_account_code(&self, code_hash: H256) -> Result<Option<Bytes>, StoreError> {
        self.engine
            .clone()
            .lock()
            .unwrap()
            .get_account_code(code_hash)
    }

    pub fn get_code_by_account_address(
        &self,
        block_number: BlockNumber,
        address: Address,
    ) -> Result<Option<Bytes>, StoreError> {
        let Some(state_trie) = self.engine.lock().unwrap().state_trie(block_number)? else {
            return Ok(None);
        };
        let hashed_address = hash_address(&address);
        let Some(encoded_state) = state_trie.get(&hashed_address)? else {
            return Ok(None);
        };
        let account_state = AccountState::decode(&encoded_state)?;
        self.get_account_code(account_state.code_hash)
    }
    pub fn get_nonce_by_account_address(
        &self,
        block_number: BlockNumber,
        address: Address,
    ) -> Result<Option<u64>, StoreError> {
        let Some(state_trie) = self.engine.lock().unwrap().state_trie(block_number)? else {
            return Ok(None);
        };
        let hashed_address = hash_address(&address);
        let Some(encoded_state) = state_trie.get(&hashed_address)? else {
            return Ok(None);
        };
        let account_state = AccountState::decode(&encoded_state)?;
        Ok(Some(account_state.nonce))
    }

    /// Applies account updates based on the block's latest storage state
    /// and returns the new state root after the updates have been aplied
    pub fn apply_account_updates(
        &self,
        block_number: BlockNumber,
        account_updates: &[AccountUpdate],
    ) -> Result<Option<H256>, StoreError> {
        let Some(mut state_trie) = self.engine.lock().unwrap().state_trie(block_number)? else {
            return Ok(None);
        };
        for update in account_updates.iter() {
            let hashed_address = hash_address(&update.address);
            if update.removed {
                // Remove account from trie
                state_trie.remove(hashed_address)?;
            } else {
                // Add or update AccountState in the trie
                // Fetch current state or create a new state to be inserted
                let mut account_state = match state_trie.get(&hashed_address)? {
                    Some(encoded_state) => AccountState::decode(&encoded_state)?,
                    None => AccountState::default(),
                };
                if let Some(info) = &update.info {
                    account_state.nonce = info.nonce;
                    account_state.balance = info.balance;
                    account_state.code_hash = info.code_hash;
                    // Store updated code in DB
                    if let Some(code) = &update.code {
                        self.add_account_code(info.code_hash, code.clone())?;
                    }
                }
                // Store the added storage in the account's storage trie and compute its root
                // TODO(TrieIntegration): We dont have the storage trie yet so we will insert into the DB table and compute the root
                if !update.added_storage.is_empty() {
                    for (storage_key, storage_value) in &update.added_storage {
                        self.add_storage_at(update.address, *storage_key, *storage_value)?;
                    }
                    account_state.storage_root = ethereum_rust_core::types::compute_storage_root(
                        &self.account_storage_iter(update.address)?.collect(),
                    );
                }
                state_trie.insert(hashed_address, account_state.encode_to_vec())?;
            }
        }
        Ok(Some(state_trie.hash()?))
    }

    /// Adds all genesis accounts and returns the genesis block's state_root
    pub fn setup_genesis_state_trie(
        &self,
        genesis_accounts: HashMap<Address, GenesisAccount>,
    ) -> Result<H256, StoreError> {
        let mut genesis_state_trie = self.engine.lock().unwrap().new_state_trie()?;
        for (address, account) in genesis_accounts {
            // Store account code (as this won't be stored in the trie)
            let code_hash = code_hash(&account.code);
            self.add_account_code(code_hash, account.code)?;
            // Store the accounts storage in the storage trie and compute its root
            // TODO(TrieIntegration): We dont have the storage trie yet so we will insert into DB table and compute the root
            let storage_root = ethereum_rust_core::types::compute_storage_root(&account.storage);
            for (storage_key, storage_value) in account.storage {
                self.add_storage_at(address, storage_key, storage_value)?;
            }
            // Add account to trie
            let account_state = AccountState {
                nonce: account.nonce,
                balance: account.balance,
                storage_root,
                code_hash,
            };
            let hashed_address = hash_address(&address);
            genesis_state_trie.insert(hashed_address, account_state.encode_to_vec())?;
        }
        genesis_state_trie.hash()
    }

    pub fn add_receipt(
        &self,
        block_number: BlockNumber,
        index: Index,
        receipt: Receipt,
    ) -> Result<(), StoreError> {
        self.engine
            .clone()
            .lock()
            .unwrap()
            .add_receipt(block_number, index, receipt)
    }

    pub fn get_receipt(
        &self,
        block_number: BlockNumber,
        index: Index,
    ) -> Result<Option<Receipt>, StoreError> {
        self.engine
            .clone()
            .lock()
            .unwrap()
            .get_receipt(block_number, index)
    }

    pub fn add_block(&self, block: Block) -> Result<(), StoreError> {
        // TODO Maybe add both in a single tx?
        let header = block.header;
        let number = header.number;
        let hash = header.compute_block_hash();
        self.add_transaction_locations(&block.body.transactions, number)?;
        self.add_block_body(number, block.body)?;
        self.add_block_header(number, header)?;
        self.add_block_number(hash, number)?;
        self.update_latest_block_number(number)
    }

    fn add_transaction_locations(
        &self,
        transactions: &[Transaction],
        block_number: BlockNumber,
    ) -> Result<(), StoreError> {
        for (index, transaction) in transactions.iter().enumerate() {
            self.add_transaction_location(
                transaction.compute_hash(),
                block_number,
                index as Index,
            )?;
        }
        Ok(())
    }

    pub fn add_initial_state(&mut self, genesis: Genesis) -> Result<(), StoreError> {
        info!("Storing initial state from genesis");

        // Obtain genesis block
        let genesis_block = genesis.get_block();
        let genesis_block_number = genesis_block.header.number;

        if let Some(header) = self.get_block_header(genesis_block_number)? {
            if header.compute_block_hash() == genesis_block.header.compute_block_hash() {
                info!("Received genesis file matching a previously stored one, nothing to do");
                return Ok(());
            } else {
                panic!("tried to run genesis twice with different blocks");
            }
        }
        // Store genesis accounts
        // TODO: Should we use this root instead of computing it before the block hash check?
        let genesis_state_root = self.setup_genesis_state_trie(genesis.alloc)?;
        debug_assert_eq!(genesis_state_root, genesis_block.header.state_root);

        // Store genesis block
        self.update_earliest_block_number(genesis_block_number)?;
        self.add_block(genesis_block)?;

        // Set chain config
        self.set_chain_config(&genesis.config)
    }

    pub fn get_transaction_by_hash(
        &self,
        transaction_hash: H256,
    ) -> Result<Option<Transaction>, StoreError> {
        self.engine
            .lock()
            .unwrap()
            .get_transaction_by_hash(transaction_hash)
    }

    pub fn add_storage_at(
        &self,
        address: Address,
        storage_key: H256,
        storage_value: U256,
    ) -> Result<(), StoreError> {
        self.engine
            .lock()
            .unwrap()
            .add_storage_at(address, storage_key, storage_value)
    }

    pub fn get_storage_at(
        &self,
        address: Address,
        storage_key: H256,
    ) -> Result<Option<U256>, StoreError> {
        self.engine
            .lock()
            .unwrap()
            .get_storage_at(address, storage_key)
    }

    pub fn remove_account_storage(&self, address: Address) -> Result<(), StoreError> {
        self.engine.lock().unwrap().remove_account_storage(address)
    }

    pub fn account_storage_iter(
        &self,
        address: Address,
    ) -> Result<Box<dyn Iterator<Item = (H256, U256)>>, StoreError> {
        self.engine.lock().unwrap().account_storage_iter(address)
    }

    pub fn set_chain_config(&self, chain_config: &ChainConfig) -> Result<(), StoreError> {
        self.engine.lock().unwrap().set_chain_config(chain_config)
    }

    pub fn get_chain_config(&self) -> Result<ChainConfig, StoreError> {
        self.engine.lock().unwrap().get_chain_config()
    }

    pub fn update_earliest_block_number(
        &self,
        block_number: BlockNumber,
    ) -> Result<(), StoreError> {
        self.engine
            .lock()
            .unwrap()
            .update_earliest_block_number(block_number)
    }

    pub fn get_earliest_block_number(&self) -> Result<Option<BlockNumber>, StoreError> {
        self.engine.lock().unwrap().get_earliest_block_number()
    }

    pub fn update_finalized_block_number(
        &self,
        block_number: BlockNumber,
    ) -> Result<(), StoreError> {
        self.engine
            .lock()
            .unwrap()
            .update_finalized_block_number(block_number)
    }

    pub fn get_finalized_block_number(&self) -> Result<Option<BlockNumber>, StoreError> {
        self.engine.lock().unwrap().get_finalized_block_number()
    }

    pub fn update_safe_block_number(&self, block_number: BlockNumber) -> Result<(), StoreError> {
        self.engine
            .lock()
            .unwrap()
            .update_safe_block_number(block_number)
    }

    pub fn get_safe_block_number(&self) -> Result<Option<BlockNumber>, StoreError> {
        self.engine.lock().unwrap().get_safe_block_number()
    }

    pub fn update_latest_block_number(&self, block_number: BlockNumber) -> Result<(), StoreError> {
        self.engine
            .lock()
            .unwrap()
            .update_latest_block_number(block_number)
    }

    pub fn get_latest_block_number(&self) -> Result<Option<BlockNumber>, StoreError> {
        self.engine.lock().unwrap().get_latest_block_number()
    }

    pub fn update_pending_block_number(&self, block_number: BlockNumber) -> Result<(), StoreError> {
        self.engine
            .lock()
            .unwrap()
            .update_pending_block_number(block_number)
    }

    pub fn get_pending_block_number(&self) -> Result<Option<BlockNumber>, StoreError> {
        self.engine.lock().unwrap().get_pending_block_number()
    }
}

fn hash_address(address: &Address) -> Vec<u8> {
    Keccak256::new_with_prefix(address.to_fixed_bytes())
        .finalize()
        .to_vec()
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, fs, panic, str::FromStr};

    use bytes::Bytes;
    use ethereum_rust_core::{
        rlp::decode::RLPDecode,
        types::{Transaction, TxType},
        Bloom,
    };
    use ethereum_types::{H256, U256};

    use super::*;

    #[cfg(feature = "in_memory")]
    #[test]
    fn test_in_memory_store() {
        test_store_suite(EngineType::InMemory);
    }

    #[cfg(feature = "libmdbx")]
    #[test]
    fn test_libmdbx_store() {
        test_store_suite(EngineType::Libmdbx);
    }

    // Creates an empty store, runs the test and then removes the store (if needed)
    fn run_test(test_func: &dyn Fn(Store), engine_type: EngineType) {
        // Remove preexistent DBs in case of a failed previous test
        if matches!(engine_type, EngineType::Libmdbx) {
            remove_test_dbs("store-test-db");
        };
        // Build a new store
        let store = Store::new("store-test-db", engine_type).expect("Failed to create test db");
        // Run the test
        test_func(store);
        // Remove store (if needed)
        if matches!(engine_type, EngineType::Libmdbx) {
            remove_test_dbs("store-test-db");
        };
    }

    fn test_store_suite(engine_type: EngineType) {
        run_test(&test_store_block, engine_type);
        run_test(&test_store_block_number, engine_type);
        run_test(&test_store_transaction_location, engine_type);
        run_test(&test_store_block_receipt, engine_type);
        run_test(&test_store_account_code, engine_type);
        run_test(&test_store_account_storage, engine_type);
        run_test(&test_remove_account_storage, engine_type);
        run_test(&test_store_block_tags, engine_type);
        run_test(&test_account_storage_iter, engine_type);
        run_test(&test_chain_config_storage, engine_type);
        run_test(&test_genesis_block, engine_type);
    }

    fn test_genesis_block(mut store: Store) {
        const GENESIS_KURTOSIS: &str = include_str!("../../test_data/genesis-kurtosis.json");
        const GENESIS_HIVE: &str = include_str!("../../test_data/genesis-hive.json");
        assert_ne!(GENESIS_KURTOSIS, GENESIS_HIVE);
        let genesis_kurtosis: Genesis =
            serde_json::from_str(GENESIS_KURTOSIS).expect("deserialize genesis-kurtosis.json");
        let genesis_hive: Genesis =
            serde_json::from_str(GENESIS_HIVE).expect("deserialize genesis-hive.json");
        store
            .add_initial_state(genesis_kurtosis.clone())
            .expect("first genesis");
        store
            .add_initial_state(genesis_kurtosis)
            .expect("second genesis with same block");
        panic::catch_unwind(move || {
            let _ = store.add_initial_state(genesis_hive);
        })
        .expect_err("genesis with a different block should panic");
    }

    fn remove_test_dbs(path: &str) {
        // Removes all test databases from filesystem
        if std::path::Path::new(path).exists() {
            fs::remove_dir_all(path).expect("Failed to clean test db dir");
        }
    }

    fn test_store_block(store: Store) {
        let (block_header, block_body) = create_block_for_testing();
        let block_number = 6;

        store
            .add_block_header(block_number, block_header.clone())
            .unwrap();
        store
            .add_block_body(block_number, block_body.clone())
            .unwrap();

        let stored_header = store.get_block_header(block_number).unwrap().unwrap();
        let stored_body = store.get_block_body(block_number).unwrap().unwrap();

        assert_eq!(stored_header, block_header);
        assert_eq!(stored_body, block_body);
    }

    fn create_block_for_testing() -> (BlockHeader, BlockBody) {
        let block_header = BlockHeader {
            parent_hash: H256::from_str(
                "0x1ac1bf1eef97dc6b03daba5af3b89881b7ae4bc1600dc434f450a9ec34d44999",
            )
            .unwrap(),
            ommers_hash: H256::from_str(
                "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
            )
            .unwrap(),
            coinbase: Address::from_str("0x2adc25665018aa1fe0e6bc666dac8fc2697ff9ba").unwrap(),
            state_root: H256::from_str(
                "0x9de6f95cb4ff4ef22a73705d6ba38c4b927c7bca9887ef5d24a734bb863218d9",
            )
            .unwrap(),
            transactions_root: H256::from_str(
                "0x578602b2b7e3a3291c3eefca3a08bc13c0d194f9845a39b6f3bcf843d9fed79d",
            )
            .unwrap(),
            receipts_root: H256::from_str(
                "0x035d56bac3f47246c5eed0e6642ca40dc262f9144b582f058bc23ded72aa72fa",
            )
            .unwrap(),
            logs_bloom: Bloom::from([0; 256]),
            difficulty: U256::zero(),
            number: 1,
            gas_limit: 0x016345785d8a0000,
            gas_used: 0xa8de,
            timestamp: 0x03e8,
            extra_data: Bytes::new(),
            prev_randao: H256::zero(),
            nonce: 0x0000000000000000,
            base_fee_per_gas: Some(0x07),
            withdrawals_root: Some(
                H256::from_str(
                    "0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421",
                )
                .unwrap(),
            ),
            blob_gas_used: Some(0x00),
            excess_blob_gas: Some(0x00),
            parent_beacon_block_root: Some(H256::zero()),
        };
        let block_body = BlockBody {
            transactions: vec![Transaction::decode(&hex::decode("b86f02f86c8330182480114e82f618946177843db3138ae69679a54b95cf345ed759450d870aa87bee53800080c080a0151ccc02146b9b11adf516e6787b59acae3e76544fdcd75e77e67c6b598ce65da064c5dd5aae2fbb535830ebbdad0234975cd7ece3562013b63ea18cc0df6c97d4").unwrap()).unwrap(),
            Transaction::decode(&hex::decode("f86d80843baa0c4082f618946177843db3138ae69679a54b95cf345ed759450d870aa87bee538000808360306ba0151ccc02146b9b11adf516e6787b59acae3e76544fdcd75e77e67c6b598ce65da064c5dd5aae2fbb535830ebbdad0234975cd7ece3562013b63ea18cc0df6c97d4").unwrap()).unwrap()],
            ommers: Default::default(),
            withdrawals: Default::default(),
        };
        (block_header, block_body)
    }

    fn test_store_block_number(store: Store) {
        let block_hash = H256::random();
        let block_number = 6;

        store.add_block_number(block_hash, block_number).unwrap();

        let stored_number = store.get_block_number(block_hash).unwrap().unwrap();

        assert_eq!(stored_number, block_number);
    }

    fn test_store_transaction_location(store: Store) {
        let transaction_hash = H256::random();
        let block_number = 6;
        let index = 3;

        store
            .add_transaction_location(transaction_hash, block_number, index)
            .unwrap();

        let stored_location = store
            .get_transaction_location(transaction_hash)
            .unwrap()
            .unwrap();

        assert_eq!(stored_location, (block_number, index));
    }

    fn test_store_block_receipt(store: Store) {
        let receipt = Receipt {
            tx_type: TxType::EIP2930,
            succeeded: true,
            cumulative_gas_used: 1747,
            bloom: Bloom::random(),
            logs: vec![],
        };
        let block_number = 6;
        let index = 4;

        store
            .add_receipt(block_number, index, receipt.clone())
            .unwrap();

        let stored_receipt = store.get_receipt(block_number, index).unwrap().unwrap();

        assert_eq!(stored_receipt, receipt);
    }

    fn test_store_account_code(store: Store) {
        let code_hash = H256::random();
        let code = Bytes::from("kiwi");

        store.add_account_code(code_hash, code.clone()).unwrap();

        let stored_code = store.get_account_code(code_hash).unwrap().unwrap();

        assert_eq!(stored_code, code);
    }

    fn test_store_account_storage(store: Store) {
        let address = Address::random();
        let storage_key_a = H256::random();
        let storage_key_b = H256::random();
        let storage_value_a = U256::from(50);
        let storage_value_b = U256::from(100);

        store
            .add_storage_at(address, storage_key_a, storage_value_a)
            .unwrap();
        store
            .add_storage_at(address, storage_key_b, storage_value_b)
            .unwrap();

        let stored_value_a = store
            .get_storage_at(address, storage_key_a)
            .unwrap()
            .unwrap();
        let stored_value_b = store
            .get_storage_at(address, storage_key_b)
            .unwrap()
            .unwrap();

        assert_eq!(stored_value_a, storage_value_a);
        assert_eq!(stored_value_b, storage_value_b);
    }

    fn test_remove_account_storage(store: Store) {
        let address_alpha = Address::random();
        let address_beta = Address::random();

        let storage_key_a = H256::random();
        let storage_key_b = H256::random();
        let storage_value_a = U256::from(50);
        let storage_value_b = U256::from(100);

        store
            .add_storage_at(address_alpha, storage_key_a, storage_value_a)
            .unwrap();
        store
            .add_storage_at(address_alpha, storage_key_b, storage_value_b)
            .unwrap();

        store
            .add_storage_at(address_beta, storage_key_a, storage_value_a)
            .unwrap();
        store
            .add_storage_at(address_beta, storage_key_b, storage_value_b)
            .unwrap();

        store.remove_account_storage(address_alpha).unwrap();

        let stored_value_alpha_a = store.get_storage_at(address_alpha, storage_key_a).unwrap();
        let stored_value_alpha_b = store.get_storage_at(address_alpha, storage_key_b).unwrap();

        let stored_value_beta_a = store.get_storage_at(address_beta, storage_key_a).unwrap();
        let stored_value_beta_b = store.get_storage_at(address_beta, storage_key_b).unwrap();

        assert!(stored_value_alpha_a.is_none());
        assert!(stored_value_alpha_b.is_none());

        assert!(stored_value_beta_a.is_some());
        assert!(stored_value_beta_b.is_some());
    }

    fn test_store_block_tags(store: Store) {
        let earliest_block_number = 0;
        let finalized_block_number = 7;
        let safe_block_number = 6;
        let latest_block_number = 8;
        let pending_block_number = 9;

        store
            .update_earliest_block_number(earliest_block_number)
            .unwrap();
        store
            .update_finalized_block_number(finalized_block_number)
            .unwrap();
        store.update_safe_block_number(safe_block_number).unwrap();
        store
            .update_latest_block_number(latest_block_number)
            .unwrap();
        store
            .update_pending_block_number(pending_block_number)
            .unwrap();

        let stored_earliest_block_number = store.get_earliest_block_number().unwrap().unwrap();
        let stored_finalized_block_number = store.get_finalized_block_number().unwrap().unwrap();
        let stored_safe_block_number = store.get_safe_block_number().unwrap().unwrap();
        let stored_latest_block_number = store.get_latest_block_number().unwrap().unwrap();
        let stored_pending_block_number = store.get_pending_block_number().unwrap().unwrap();

        assert_eq!(earliest_block_number, stored_earliest_block_number);
        assert_eq!(finalized_block_number, stored_finalized_block_number);
        assert_eq!(safe_block_number, stored_safe_block_number);
        assert_eq!(latest_block_number, stored_latest_block_number);
        assert_eq!(pending_block_number, stored_pending_block_number);
    }

    fn test_account_storage_iter(store: Store) {
        let address = Address::random();
        // Build preset account storage
        let account_storage = HashMap::from([
            (H256::random(), U256::from(7)),
            (H256::random(), U256::from(17)),
            (H256::random(), U256::from(77)),
            (H256::random(), U256::from(707)),
        ]);

        // Store account storage
        for (key, value) in account_storage.clone() {
            store.add_storage_at(address, key, value).unwrap();
        }

        // Fetch account storage from db and compare against preset
        let account_storage_iter = store.account_storage_iter(address).unwrap();
        let account_storage_from_iter = HashMap::from_iter(account_storage_iter);
        assert_eq!(account_storage, account_storage_from_iter)
    }

    fn test_chain_config_storage(store: Store) {
        let chain_config = example_chain_config();
        store.set_chain_config(&chain_config).unwrap();
        let retrieved_chain_config = store.get_chain_config().unwrap();
        assert_eq!(chain_config, retrieved_chain_config);
    }

    fn example_chain_config() -> ChainConfig {
        ChainConfig {
            chain_id: 3151908_u64,
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
            terminal_total_difficulty: Some(58750000000000000000000),
            terminal_total_difficulty_passed: true,
            ..Default::default()
        }
    }
}
