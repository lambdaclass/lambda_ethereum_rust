use super::{Key, StoreEngine, Value};
use crate::rlp::account::{AccountInfoRLP, AddressRLP};
use anyhow::Result;
use ethereum_rust_core::types::AccountInfo;
use ethereum_types::Address;
use libmdbx::orm::{Decodable, Encodable};
use sled::Db;
use std::fmt::Debug;

#[derive(Clone)]
pub struct Store {
    account_infos: Db,
    values: Db,
}

impl Store {
    pub fn new(path: &str) -> Result<Self> {
        Ok(Self {
            account_infos: sled::open(format!("{path}.accounts.db"))?,
            values: sled::open(format!("{path}.values.db"))?,
        })
    }
}

impl StoreEngine for Store {
    fn add_account_info(&mut self, address: Address, account_info: AccountInfo) -> Result<()> {
        let address_rlp: AddressRLP = address.into();
        let account_info_rlp: AccountInfoRLP = account_info.into();
        let _ = self
            .account_infos
            .insert(address_rlp.encode(), account_info_rlp.encode());
        Ok(())
    }

    fn get_account_info(&self, address: Address) -> Result<Option<AccountInfo>> {
        let address_rlp: AddressRLP = address.into();
        self.account_infos
            .get(address_rlp.encode())?
            .map_or(Ok(None), |value| {
                Ok(Some(AccountInfoRLP::decode(&value)?.to()))
            })
    }

    fn set_value(&mut self, key: Key, value: Value) -> Result<()> {
        let _ = self.values.insert(key, value);
        Ok(())
    }

    fn get_value(&self, key: Key) -> Result<Option<Vec<u8>>> {
        Ok(self.values.get(key)?.map(|value| value.to_vec()))
    }
}

impl Debug for Store {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Sled Store").finish()
    }
}
