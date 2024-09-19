use crate::trie::Trie;
use crate::{error::StoreError, AccountUpdate};
use ethereum_rust_core::rlp::decode::RLPDecode;
use ethereum_rust_core::rlp::encode::RLPEncode;
use ethereum_rust_core::types::{
    code_hash, AccountInfo, AccountState, GenesisAccount, EMPTY_TRIE_HASH,
};
use ethereum_types::{Address, H256, U256};
use sha3::{Digest as _, Keccak256};
use std::collections::HashMap;

pub trait StateProvider {
    fn get_account_info(&self, address: Address) -> Result<Option<AccountInfo>, StoreError>;

    fn apply_account_updates(
        &mut self,
        account_updates: &[AccountUpdate],
    ) -> Result<Option<H256>, StoreError>;

    fn setup_genesis_state(
        &mut self,
        genesis_accounts: HashMap<Address, GenesisAccount>,
    ) -> Result<H256, StoreError>;

    fn get_storage_at(
        &self,
        address: Address,
        storage_key: H256,
    ) -> Result<Option<U256>, StoreError>;
}

pub struct TrieStateProvider {
    trie: Trie,
}

impl TrieStateProvider {
    pub fn new(trie: Trie) -> Self {
        Self { trie }
    }
}

impl StateProvider for TrieStateProvider {
    fn get_account_info(&self, address: Address) -> Result<Option<AccountInfo>, StoreError> {
        let hashed_address = hash_address(&address);
        let encoded_state = self.trie.get(&hashed_address)?;

        match encoded_state {
            Some(state) => {
                let account_state = AccountState::decode(&state)?;
                Ok(Some(AccountInfo {
                    code_hash: account_state.code_hash,
                    balance: account_state.balance,
                    nonce: account_state.nonce,
                }))
            }
            None => Ok(None),
        }
    }

    fn setup_genesis_state(
        &mut self,
        genesis_accounts: HashMap<Address, GenesisAccount>,
    ) -> Result<H256, StoreError> {
        for (address, account) in genesis_accounts {
            let code_hash = code_hash(&account.code);
            let storage_root = *EMPTY_TRIE_HASH; // Assuming empty storage for genesis

            let account_state = AccountState {
                nonce: account.nonce,
                balance: account.balance,
                storage_root,
                code_hash,
            };

            let hashed_address = hash_address(&address);
            self.trie
                .insert(hashed_address, account_state.encode_to_vec())?;
        }

        self.trie.hash()
    }

    fn get_storage_at(
        &self,
        address: Address,
        storage_key: H256,
    ) -> Result<Option<U256>, StoreError> {
        // Note: This implementation assumes that the storage trie is part of the main trie.
        // In reality, you might need to handle this differently depending on your trie structure.
        let hashed_address = hash_address(&address);
        let encoded_account = self.trie.get(&hashed_address)?;

        match encoded_account {
            Some(account_data) => {
                let account_state = AccountState::decode(&account_data)?;
                let hashed_key = hash_key(&storage_key);

                // This part might need adjustment based on how you're storing the storage trie
                let storage_value = self.trie.get(&hashed_key)?;

                storage_value
                    .map(|rlp| U256::decode(&rlp).map_err(StoreError::RLPDecode))
                    .transpose()
            }
            None => Ok(None),
        }
    }
}

fn hash_address(address: &Address) -> Vec<u8> {
    Keccak256::new_with_prefix(address.to_fixed_bytes())
        .finalize()
        .to_vec()
}

fn hash_key(key: &H256) -> Vec<u8> {
    Keccak256::new_with_prefix(key.to_fixed_bytes())
        .finalize()
        .to_vec()
}
