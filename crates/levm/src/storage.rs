use std::collections::HashMap;

use ethereum_types::{Address, U256};

#[derive(Debug, Clone, Default)]
pub struct TransientStorage(HashMap<(Address, U256), U256>);

impl TransientStorage {
    /// Returns a new empty transient storage.
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    /// Returns `true` if the storage contains no elements.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Get a value at a storage key for an account.
    ///
    /// Returns `U256::zero()` if there is no such value. See [implementation reference]
    /// or [EIP-1153].
    ///
    /// [implementation reference]: https://github.com/ethereum/execution-specs/blob/51fac24740e662844446439ceeb96a460aae0ba0/src/ethereum/cancun/state.py#L641
    /// [EIP-1153]: https://eips.ethereum.org/EIPS/eip-1153#reference-implementation
    pub fn get(&self, address: Address, key: U256) -> U256 {
        if let Some(value) = self.0.get(&(address, key)) {
            *value
        } else {
            U256::zero()
        }
    }

    /// Set a value at a storage key for an account.
    pub fn set(&mut self, address: Address, key: U256, value: U256) {
        self.0.insert((address, key), value);
    }
}
