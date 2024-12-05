use std::collections::HashMap;

use ethereum_types::H160;
use ethrex_core::{
    types::{AccountState, Block, ChainConfig},
    Address, H256, U256,
};
use ethrex_rlp::decode::RLPDecode;
use ethrex_storage::{hash_address, hash_key, Store};
use ethrex_trie::{NodeRLP, Trie};
use revm::{
    primitives::{
        AccountInfo as RevmAccountInfo, Address as RevmAddress, Bytecode as RevmBytecode,
        B256 as RevmB256, U256 as RevmU256,
    },
    DatabaseRef,
};
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::{errors::ExecutionDBError, evm_state, execute_block, get_state_transitions};

/// In-memory EVM database for caching execution data.
///
/// This is mainly used to store the relevant state data for executing a particular block and then
/// feeding the DB into a zkVM program to prove the execution.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExecutionDB {
    /// indexed by account address
    pub accounts: HashMap<RevmAddress, AccountState>,
    /// indexed by code hash
    pub code: HashMap<RevmB256, RevmBytecode>,
    /// indexed by account address and storage key
    pub storage: HashMap<RevmAddress, HashMap<RevmU256, RevmU256>>,
    /// indexed by block number
    pub block_hashes: HashMap<u64, RevmB256>,
    /// stored chain config
    pub chain_config: ChainConfig,
    /// encoded nodes to reconstruct a state trie, but only including relevant data (pruned).
    /// root node is stored separately from the rest.
    pruned_state_trie: (Option<NodeRLP>, Vec<NodeRLP>),
    /// encoded nodes to reconstruct every storage trie, but only including relevant data (pruned)
    /// root nodes are stored separately from the rest.
    pruned_storage_tries: HashMap<H160, (Option<NodeRLP>, Vec<NodeRLP>)>,
}

impl ExecutionDB {
    /// Creates a new [ExecutionDB] from raw values.
    pub fn new(
        accounts: HashMap<Address, AccountState>,
        storage: HashMap<Address, HashMap<U256, U256>>,
        account_proofs: (Option<NodeRLP>, Vec<NodeRLP>),
        storage_proofs: HashMap<Address, (Option<NodeRLP>, Vec<NodeRLP>)>,
        chain_config: ChainConfig,
    ) -> Self {
        let accounts = accounts
            .into_iter()
            .map(|(address, value)| (RevmAddress::from_slice(address.as_bytes()), value))
            .collect();

        let storage = storage
            .into_iter()
            .map(|(address, storage)| {
                (
                    RevmAddress::from_slice(address.as_bytes()),
                    storage
                        .into_iter()
                        .map(|(key, value)| {
                            let mut key_bytes = [0; 32];
                            let mut value_bytes = [0; 32];

                            key.to_big_endian(&mut key_bytes);
                            value.to_big_endian(&mut value_bytes);

                            (
                                RevmU256::from_be_slice(&key_bytes),
                                RevmU256::from_be_slice(&value_bytes),
                            )
                        })
                        .collect(),
                )
            })
            .collect();

        let pruned_state_trie = account_proofs;
        let pruned_storage_tries = storage_proofs
            .into_iter()
            .map(|(address, proofs)| (H160::from_slice(address.as_bytes()), proofs))
            .collect();

        Self {
            accounts,
            code: HashMap::new(),
            storage,
            block_hashes: HashMap::new(),
            chain_config,
            pruned_state_trie,
            pruned_storage_tries,
        }
    }

    /// Creates a database by executing a block, without performing any validation.
    pub fn from_exec(block: &Block, store: &Store) -> Result<Self, ExecutionDBError> {
        // TODO: perform validation to exit early

        // Execute and obtain account updates
        let mut state = evm_state(store.clone(), block.header.parent_hash);
        let chain_config = store.get_chain_config()?;
        execute_block(block, &mut state).map_err(Box::new)?;
        let account_updates = get_state_transitions(&mut state);

        // Store data touched by updates and get all touched storage keys for each account
        let mut accounts = HashMap::new();
        let code = HashMap::new(); // TODO: `code` remains empty for now
        let mut storage = HashMap::new();
        let block_hashes = HashMap::new(); // TODO: `block_hashes` remains empty for now

        let mut address_storage_keys = HashMap::new();

        for account_update in account_updates.iter() {
            let address = RevmAddress::from_slice(account_update.address.as_bytes());
            let account_state = match store.get_account_state_by_hash(
                block.header.parent_hash,
                H160::from_slice(address.as_slice()),
            )? {
                Some(state) => state,
                None => continue,
            };
            accounts.insert(address, account_state);

            let account_storage = account_update
                .added_storage
                .iter()
                .map(|(key, value)| {
                    let mut value_bytes = [0u8; 32];
                    value.to_big_endian(&mut value_bytes);
                    (
                        RevmU256::from_be_bytes(key.to_fixed_bytes()),
                        RevmU256::from_be_slice(&value_bytes),
                    )
                })
                .collect();
            storage.insert(address, account_storage);
            address_storage_keys.insert(
                account_update.address,
                account_update
                    .added_storage
                    .keys()
                    .cloned()
                    .collect::<Vec<_>>(),
            );
        }

        // Get pruned state and storage tries. For this we get the "state" (all relevant nodes) of every trie.
        // "Pruned" because we're only getting the nodes that make paths to the relevant
        // key-values.
        let state_trie = store.state_trie(block.header.parent_hash)?.ok_or(
            ExecutionDBError::NewMissingStateTrie(block.header.parent_hash),
        )?;

        // Get pruned state trie
        let state_paths: Vec<_> = address_storage_keys.keys().map(hash_address).collect();
        let pruned_state_trie = state_trie.get_proofs(&state_paths)?;

        // Get pruned storage tries for every account
        let mut pruned_storage_tries = HashMap::new();
        for (address, keys) in address_storage_keys {
            let storage_trie = store
                .storage_trie(block.header.parent_hash, address)?
                .ok_or(ExecutionDBError::NewMissingStorageTrie(
                    block.header.parent_hash,
                    address,
                ))?;
            let storage_paths: Vec<_> = keys.iter().map(hash_key).collect();
            let (storage_trie_root, storage_trie_nodes) =
                storage_trie.get_proofs(&storage_paths)?;
            pruned_storage_tries.insert(address, (storage_trie_root, storage_trie_nodes));
        }

        Ok(Self {
            accounts,
            code,
            storage,
            block_hashes,
            chain_config,
            pruned_state_trie,
            pruned_storage_tries,
        })
    }

    pub fn get_chain_config(&self) -> ChainConfig {
        self.chain_config
    }

    /// Verifies that all data in [self] is included in the stored tries, and then builds the
    /// pruned tries from the stored nodes.
    pub fn build_tries(&mut self) -> Result<(Trie, HashMap<H160, Trie>), ExecutionDBError> {
        let (state_trie_root, state_trie_nodes) = &self.pruned_state_trie;
        let state_trie = Trie::from_nodes(state_trie_root.as_ref(), state_trie_nodes)?;
        let mut storage_tries = HashMap::new();

        // we'll remove any accounts and storage which we don't have a proof of inclusion for
        let mut accounts_to_remove = Vec::new();
        let mut storage_to_remove = Vec::new();

        for (revm_address, account) in &self.accounts {
            let address = H160::from_slice(revm_address.as_slice());

            // check account is in state trie
            if state_trie.get(&hash_address(&address))?.is_none() {
                warn!(
                    "Account {} not found in state trie, will be removed from ExecutionDB",
                    address
                );
                accounts_to_remove.push(revm_address.to_owned());
                continue;
            }

            // validate storage, note that an ExecutionDB only stores values relevant to some
            // execution, so an account might have storage whilest there's none stored in
            // self.storage[address]
            match self.storage.get(revm_address) {
                Some(storage) if !storage.is_empty() => {
                    // first get the tries
                    let (storage_trie_root, storage_trie_nodes) = self
                        .pruned_storage_tries
                        .get(&address)
                        .ok_or(ExecutionDBError::MissingStorageTrie(address))?;

                    // compare account storage root with storage trie root
                    let storage_trie =
                        Trie::from_nodes(storage_trie_root.as_ref(), storage_trie_nodes)?;
                    let storage_root = storage_trie.hash_no_commit();
                    if storage_root != account.storage_root {
                        return Err(ExecutionDBError::InvalidStorageTrieRoot(
                            address,
                            storage_root,
                            account.storage_root,
                        ));
                    }

                    // check all storage keys are in storage trie and compare values
                    for (revm_key, value) in storage {
                        let key = H256::from_slice(&revm_key.to_be_bytes_vec());
                        let value = U256::from_big_endian(&value.to_be_bytes_vec());

                        if let Some(retrieved_value) = &storage_trie.get(&hash_key(&key))? {
                            let retrieved_value = RLPDecode::decode(retrieved_value)?;
                            if value != retrieved_value {
                                return Err(ExecutionDBError::InvalidStorageTrieValue(
                                    address,
                                    retrieved_value,
                                    value,
                                ));
                            }
                        } else {
                            warn!("Storage key {} not found in storage trie of account {}, will be removed from ExecutionDB", key, address);
                            storage_to_remove.push((revm_address.to_owned(), revm_key.to_owned()));
                        }
                    }

                    storage_tries.insert(address, storage_trie);
                }
                _ => (),
            }
        }

        // remove any accounts adn storage which we don't have a proof of inclusion for
        for address in accounts_to_remove {
            self.accounts.remove(&address);
            self.storage.remove(&address);
        }
        for (address, key) in storage_to_remove {
            self.storage
                .get_mut(&address)
                .ok_or(ExecutionDBError::StorageNotFound(address))?
                .remove(&key);
        }

        Ok((state_trie, storage_tries))
    }
}

impl DatabaseRef for ExecutionDB {
    /// The database error type.
    type Error = ExecutionDBError;

    /// Get basic account information.
    fn basic_ref(&self, address: RevmAddress) -> Result<Option<RevmAccountInfo>, Self::Error> {
        let Some(account_state) = self.accounts.get(&address) else {
            return Ok(None);
        };

        Ok(Some(RevmAccountInfo {
            balance: {
                let mut balance_bytes = [0; 32];
                account_state.balance.to_big_endian(&mut balance_bytes);
                RevmU256::from_be_bytes(balance_bytes)
            },
            nonce: account_state.nonce,
            code_hash: RevmB256::from_slice(account_state.code_hash.as_bytes()),
            code: None,
        }))
    }

    /// Get account code by its hash.
    fn code_by_hash_ref(&self, code_hash: RevmB256) -> Result<RevmBytecode, Self::Error> {
        self.code
            .get(&code_hash)
            .cloned()
            .ok_or(ExecutionDBError::CodeNotFound(code_hash))
    }

    /// Get storage value of address at index.
    fn storage_ref(&self, address: RevmAddress, index: RevmU256) -> Result<RevmU256, Self::Error> {
        self.storage
            .get(&address)
            .ok_or(ExecutionDBError::AccountNotFound(address))?
            .get(&index)
            .cloned()
            .ok_or(ExecutionDBError::StorageValueNotFound(address, index))
    }

    /// Get block hash by block number.
    fn block_hash_ref(&self, number: u64) -> Result<RevmB256, Self::Error> {
        self.block_hashes
            .get(&number)
            .cloned()
            .ok_or(ExecutionDBError::BlockHashNotFound(number))
    }
}

pub mod touched_state {
    use ethrex_core::{types::Block, Address, U256};
    use revm::{inspectors::TracerEip3155, DatabaseCommit, DatabaseRef, Evm};
    use revm_primitives::{
        Account as RevmAccount, AccountStatus as RevmAccountStatus, Address as RevmAddress,
        EVMError, SpecId, U256 as RevmU256,
    };

    use crate::{block_env, tx_env};

    type TouchedStateDBError = ();

    /// Dummy DB for storing touched account addresses and storage keys while executing a block.
    #[derive(Default)]
    struct TouchedStateDB {
        touched_state: Vec<(RevmAddress, Vec<RevmU256>)>,
    }

    #[allow(unused_variables)]
    impl DatabaseRef for TouchedStateDB {
        type Error = TouchedStateDBError;

        fn basic_ref(
            &self,
            address: RevmAddress,
        ) -> Result<Option<revm_primitives::AccountInfo>, Self::Error> {
            Ok(Some(Default::default()))
        }
        fn storage_ref(
            &self,
            address: RevmAddress,
            index: RevmU256,
        ) -> Result<RevmU256, Self::Error> {
            Ok(Default::default())
        }
        fn block_hash_ref(&self, number: u64) -> Result<revm_primitives::B256, Self::Error> {
            Ok(Default::default())
        }
        fn code_by_hash_ref(
            &self,
            code_hash: revm_primitives::B256,
        ) -> Result<revm_primitives::Bytecode, Self::Error> {
            Ok(Default::default())
        }
    }

    impl DatabaseCommit for TouchedStateDB {
        fn commit(&mut self, changes: revm_primitives::HashMap<RevmAddress, RevmAccount>) {
            for (address, account) in changes {
                self.touched_state
                    .push((address, account.storage.keys().cloned().collect()));
            }
        }
    }

    /// Get all touched account addresses and storage keys during the execution of a block,
    /// ignoring newly created accounts.
    ///
    /// Generally used for building an [super::ExecutionDB].
    pub fn get_touched_state(
        block: &Block,
        chain_id: u64,
        spec_id: SpecId,
    ) -> Result<Vec<(Address, Vec<U256>)>, EVMError<TouchedStateDBError>> {
        let block_env = block_env(&block.header);
        let mut db = TouchedStateDB::default();

        for transaction in &block.body.transactions {
            let mut tx_env = tx_env(transaction);

            // disable nonce check (we're executing with empty accounts, nonce 0)
            tx_env.nonce = None;

            // execute tx
            let evm_builder = Evm::builder()
                .with_block_env(block_env.clone())
                .with_tx_env(tx_env)
                .modify_cfg_env(|cfg| {
                    cfg.chain_id = chain_id;
                    // we're executing with empty accounts, balance 0
                    cfg.disable_balance_check = true;
                })
                .with_spec_id(spec_id)
                .with_external_context(
                    TracerEip3155::new(Box::new(std::io::stderr())).without_summary(),
                );
            let mut evm = evm_builder.with_ref_db(&mut db).build();
            evm.transact_commit()?;
        }

        let mut touched_state: Vec<(Address, Vec<U256>)> = db
            .touched_state
            .into_iter()
            .map(|(address, storage_keys)| {
                (
                    Address::from_slice(address.as_slice()),
                    storage_keys
                        .into_iter()
                        .map(|key| U256::from_big_endian(&key.to_be_bytes_vec()))
                        .collect(),
                )
            })
            .collect();

        // add withdrawal accounts
        if let Some(ref withdrawals) = block.body.withdrawals {
            touched_state.extend(withdrawals.iter().map(|w| (w.address, Vec::new())))
        }

        Ok(touched_state)
    }
}
