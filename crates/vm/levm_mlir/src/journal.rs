use crate::{
    constants::EMPTY_CODE_HASH_STR,
    db::{AccountInfo, Bytecode, Database, Db},
    env::AccessList,
    primitives::{Address, B256, U256},
    state::{Account, AccountStatus, EvmStorageSlot},
};

use sha3::{Digest, Keccak256};
use std::collections::{hash_map::Entry, HashMap};
use std::str::FromStr;

#[derive(Clone, Default, Debug, PartialEq)]
pub struct JournalStorageSlot {
    /// Original value of the storage slot.
    pub original_value: U256,
    /// Present value of the storage slot.
    pub present_value: U256,
}

impl From<U256> for JournalStorageSlot {
    fn from(value: U256) -> Self {
        Self {
            original_value: value,
            present_value: value,
        }
    }
}

// NOTE: We could store the bytecode inside this `JournalAccount` instead of
// having a separate HashMap for it.
#[derive(Clone, Default, Debug, PartialEq)]
pub struct JournalAccount {
    pub nonce: u64,
    pub balance: U256,
    pub storage: HashMap<U256, JournalStorageSlot>,
    pub bytecode_hash: B256,
    pub status: AccountStatus,
}

impl JournalAccount {
    pub fn has_code(&self) -> bool {
        !(self.bytecode_hash == B256::zero()
            || self.bytecode_hash == B256::from_str(EMPTY_CODE_HASH_STR).unwrap())
    }

    pub fn new_created(balance: U256) -> Self {
        Self {
            nonce: 0,
            balance,
            storage: Default::default(),
            bytecode_hash: B256::from_str(EMPTY_CODE_HASH_STR).unwrap(),
            status: AccountStatus::Created,
        }
    }
}

impl From<AccountInfo> for JournalAccount {
    fn from(info: AccountInfo) -> JournalAccount {
        JournalAccount {
            nonce: info.nonce,
            balance: info.balance,
            storage: Default::default(),
            bytecode_hash: info.code_hash,
            status: AccountStatus::Cold,
        }
    }
}

impl From<&JournalAccount> for AccountInfo {
    fn from(acc: &JournalAccount) -> Self {
        Self {
            balance: acc.balance,
            nonce: acc.nonce,
            code_hash: acc.bytecode_hash,
            code: None,
        }
    }
}

type AccountState = HashMap<Address, JournalAccount>;
type ContractState = HashMap<B256, Bytecode>;

#[derive(Default, Debug)]
pub struct Journal<'a> {
    accounts: AccountState,
    contracts: ContractState,
    block_hashes: HashMap<U256, B256>,
    db: Option<&'a mut Db>,
}

// TODO: Handle unwraps and panics
// TODO: Improve overall performance
//  -> Performance is not the focus currently
//  -> Many copies, clones and Db fetches that may be reduced
//  -> For the moment we seek for something that works.
//  -> We can optimize in the future.
impl<'a> Journal<'a> {
    pub fn new(db: &'a mut Db) -> Self {
        Self {
            db: Some(db),
            ..Default::default()
        }
    }

    pub fn with_prefetch(mut self, accounts: &AccessList) -> Self {
        self.accounts = Self::get_prefetch_accounts(accounts);
        self
    }

    fn get_prefetch_accounts(prefetch_accounts: &AccessList) -> HashMap<Address, JournalAccount> {
        let mut accounts = HashMap::new();
        for (address, storage) in prefetch_accounts {
            let account = accounts
                .entry(*address)
                .or_insert_with(JournalAccount::default);
            let storage: Vec<(U256, JournalStorageSlot)> = storage
                .iter()
                .map(|key| (*key, JournalStorageSlot::default()))
                .collect();
            account.storage.extend(storage);
        }
        accounts
    }

    /* ACCOUNT HANDLING */

    pub fn new_account(&mut self, address: Address, balance: U256) {
        // TODO: Check if account already exists and return error or panic
        let account = JournalAccount::new_created(balance);
        self.accounts.insert(address, account);
    }

    pub fn add_account_as_warm(&mut self, address: Address) {
        let account = self
            .accounts
            .entry(address)
            .or_insert(JournalAccount::new_created(U256::zero()));

        account.status &= !AccountStatus::Cold;
    }

    pub fn new_contract(&mut self, address: Address, bytecode: Bytecode, balance: U256) {
        let mut hasher = Keccak256::new();
        hasher.update(&bytecode);
        let hash = B256::from_slice(&hasher.finalize());
        let account = JournalAccount {
            bytecode_hash: hash,
            balance,
            nonce: 1,
            status: AccountStatus::Created,
            ..Default::default()
        };

        self.accounts.insert(address, account);
        self.contracts.insert(hash, bytecode);
    }

    pub fn set_balance(&mut self, address: &Address, balance: U256) {
        if let Some(acc) = self._get_account_mut(address) {
            acc.balance = balance;
            acc.status |= AccountStatus::Touched;
        }
    }

    pub fn set_nonce(&mut self, address: &Address, nonce: u64) {
        if let Some(acc) = self._get_account_mut(address) {
            acc.nonce = nonce;
            acc.status |= AccountStatus::Touched;
        }
    }

    pub fn set_status(&mut self, address: &Address, status: AccountStatus) {
        if let Some(acc) = self._get_account_mut(address) {
            acc.status |= status;
        }
    }

    pub fn get_account(&mut self, address: &Address) -> Option<AccountInfo> {
        self._get_account(address).map(AccountInfo::from)
    }

    pub fn code_by_address(&mut self, address: &Address) -> Bytecode {
        let default = Bytecode::default();
        let Some(acc) = self._get_account_mut(address) else {
            return default;
        };
        acc.status &= !AccountStatus::Cold;
        if !acc.has_code() {
            return default;
        }

        let hash = acc.bytecode_hash;
        self.contracts.get(&hash).cloned().unwrap_or(
            self.db
                .as_mut()
                .and_then(|db| db.code_by_hash(hash).ok())
                .unwrap_or(default),
        )
    }

    /* WARM COLD HANDLING */

    pub fn account_is_warm(&self, address: &Address) -> bool {
        self.accounts
            .get(address)
            .map(|acc| !acc.status.contains(AccountStatus::Cold))
            .unwrap_or(false)
    }

    pub fn prefetch_account(&mut self, address: &Address) {
        let _ = self._get_account(address);
    }

    pub fn prefetch_account_keys(&mut self, address: &Address, keys: &[U256]) {
        if self._get_account(address).is_none() {
            return;
        };

        let slots: HashMap<U256, JournalStorageSlot> = keys
            .iter()
            .map(|key| (*key, self._fetch_storage_from_db(address, key)))
            .collect();

        let acc = self._get_account_mut(address).unwrap();
        acc.storage.extend(slots);
    }

    // We ignore the `EvmStorageSlot::is_cold` attribute
    pub fn key_is_warm(&self, address: &Address, key: &U256) -> bool {
        self.accounts
            .get(address)
            .and_then(|acc| acc.storage.get(key))
            .is_some()
    }

    /* STORAGE HANDLING */

    pub fn read_storage(&mut self, address: &Address, key: &U256) -> Option<JournalStorageSlot> {
        //TODO: If AccountStatus::Created, then we don't need to fetch DB
        let acc = self._get_account(address)?;
        let slot = acc
            .storage
            .get(key)
            .cloned()
            .unwrap_or(self._fetch_storage_from_db(address, key));
        let acc = self._get_account_mut(address).unwrap();
        acc.storage.insert(*key, slot.clone()); // Now this key is warm
        Some(slot)
    }

    pub fn write_storage(&mut self, address: &Address, key: U256, value: U256) {
        let acc = self._get_account(address).unwrap(); //TODO handle error here
        let mut slot = acc
            .storage
            .get(&key)
            .cloned()
            .unwrap_or(self._fetch_storage_from_db(address, &key));

        slot.present_value = value;
        let acc = self._get_account_mut(address).unwrap();
        acc.storage.insert(key, slot.clone());
        acc.status |= AccountStatus::Touched;
    }

    /* BLOCK HASH */

    pub fn get_block_hash(&mut self, number: &U256) -> B256 {
        match self.block_hashes.get(number).cloned() {
            Some(hash) => hash,
            None => {
                let block_hash = self
                    .db
                    .as_mut()
                    .and_then(|db| db.block_hash(*number).ok())
                    .unwrap_or_default();
                self.block_hashes.insert(*number, block_hash);
                block_hash
            }
        }
    }

    /* OTHER METHODS */

    pub fn into_state(&self) -> HashMap<Address, Account> {
        self.accounts
            .iter()
            .map(|(address, acc)| {
                let code = acc
                    .has_code()
                    .then_some(self.contracts.get(&acc.bytecode_hash))
                    .flatten()
                    .cloned();

                let storage = acc
                    .storage
                    .iter()
                    .map(|(&key, slot)| {
                        (
                            key,
                            EvmStorageSlot {
                                original_value: slot.original_value,
                                present_value: slot.present_value,
                                is_cold: false,
                            },
                        )
                    })
                    .collect();
                (
                    *address,
                    Account {
                        info: AccountInfo {
                            balance: acc.balance,
                            nonce: acc.nonce,
                            code_hash: acc.bytecode_hash,
                            code,
                        },
                        storage,
                        status: acc.status,
                    },
                )
            })
            .collect()
    }

    pub fn eject_base(&mut self) -> Self {
        Self {
            accounts: self.accounts.clone(),
            contracts: self.contracts.clone(),
            block_hashes: self.block_hashes.clone(),
            db: self.db.take(),
        }
    }

    pub fn extend_from_successful(&mut self, other: Journal<'a>) {
        self.accounts = other.accounts;
        self.contracts = other.contracts;
        self.block_hashes = other.block_hashes;
        self.db = other.db;
    }

    pub fn extend_from_reverted(&mut self, other: Journal<'a>) {
        // TODO: Maybe warm/cold state on both addresses and storage keys should be preserved
        // but if that's the case, then:
        // - What happens if the reverted callee context created a new account/storage key?
        //   -> Should the warm state be preserved? What value should we introduce into the
        //      self.accounts HashMap? A default one?
        //
        // For the moment, we will just discard changes and take the database back
        self.db = other.db
    }

    /* PRIVATE AUXILIARY METHODS */

    fn _get_account(&mut self, address: &Address) -> Option<&JournalAccount> {
        self._get_account_mut(address).map(|acc| &*acc)
    }

    fn _get_account_mut(&mut self, address: &Address) -> Option<&mut JournalAccount> {
        let Some(db) = &mut self.db else {
            return None;
        };

        let maybe_acc = match self.accounts.entry(*address) {
            Entry::Occupied(e) => Some(e.into_mut()),
            Entry::Vacant(e) => {
                let acc = db.basic(*address).ok().flatten()?;
                let mut acc = JournalAccount::from(acc);
                acc.status = AccountStatus::Loaded;
                Some(e.insert(acc))
            }
        };

        maybe_acc
    }

    fn _fetch_storage_from_db(&mut self, address: &Address, key: &U256) -> JournalStorageSlot {
        let value = self
            .db
            .as_mut()
            .and_then(|db| db.storage(*address, *key).ok())
            .unwrap_or_default();
        JournalStorageSlot::from(value)
    }
}
