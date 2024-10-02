use crate::{db::AccountInfo, primitives::U256};
use bitflags::bitflags;
use core::hash::Hash;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Account {
    /// Balance, nonce, and code.
    pub info: AccountInfo,
    /// Storage cache
    pub storage: HashMap<U256, EvmStorageSlot>,
    /// Account status flags.
    pub status: AccountStatus,
}

impl Account {
    /// Is account marked for selfdestruction.
    pub fn is_selfdestructed(&self) -> bool {
        self.status.contains(AccountStatus::SelfDestructed)
    }

    /// Is account marked as created.
    pub fn is_created(&self) -> bool {
        self.status.contains(AccountStatus::Created)
    }

    /// If account status is marked as touched.
    pub fn is_touched(&self) -> bool {
        self.status.contains(AccountStatus::Touched)
    }
}

// The `bitflags!` macro generates `struct`s that manage a set of flags.
bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct AccountStatus: u8 {
        /// When account is loaded but not touched or interacted with.
        /// This is the default state.
        const Loaded = 0b00000000;
        /// When account is newly created we will not access database
        /// to fetch storage values
        const Created = 0b00000001;
        /// If account is marked for self destruction.
        const SelfDestructed = 0b00000010;
        /// Only when account is marked as touched we will save it to database.
        const Touched = 0b00000100;
        /// used only for pre spurious dragon hardforks where existing and empty were two separate states.
        /// it became same state after EIP-161: State trie clearing
        const LoadedAsNotExisting = 0b0001000;
        /// used to mark account as cold
        const Cold = 0b0010000;
    }
}

impl Default for AccountStatus {
    fn default() -> Self {
        Self::Loaded
    }
}

impl From<AccountInfo> for Account {
    fn from(info: AccountInfo) -> Self {
        Self {
            info,
            storage: HashMap::new(),
            status: AccountStatus::Loaded,
        }
    }
}

/// This type keeps track of the current value of a storage slot.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct EvmStorageSlot {
    /// Original value of the storage slot.
    pub original_value: U256,
    /// Present value of the storage slot.
    pub present_value: U256,
    /// Represents if the storage slot is cold.
    pub is_cold: bool,
}

impl From<U256> for EvmStorageSlot {
    fn from(value: U256) -> Self {
        Self {
            present_value: value,
            original_value: value,
            is_cold: true,
        }
    }
}
