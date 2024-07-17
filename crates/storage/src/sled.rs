use super::{Key, StoreEngine, Value};
use crate::error::StoreError;
use crate::rlp::{AccountInfoRLP, AddressRLP};
use ethereum_rust_core::types::{AccountInfo, BlockBody, BlockHash, BlockHeader, BlockNumber};
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
    pub fn new(path: &str) -> Result<Self, StoreError> {
        Ok(Self {
            account_infos: sled::open(format!("{path}.accounts.db"))?,
            values: sled::open(format!("{path}.values.db"))?,
        })
    }
}

impl StoreEngine for Store {
    fn add_account_info(
        &mut self,
        address: Address,
        account_info: AccountInfo,
    ) -> Result<(), StoreError> {
        let address_rlp: AddressRLP = address.into();
        let account_info_rlp: AccountInfoRLP = account_info.into();
        let _ = self
            .account_infos
            .insert(address_rlp.encode(), account_info_rlp.encode());
        Ok(())
    }

    fn get_account_info(&self, address: Address) -> Result<Option<AccountInfo>, StoreError> {
        let address_rlp: AddressRLP = address.into();
        self.account_infos
            .get(address_rlp.encode())?
            .map_or(Ok(None), |value| match AccountInfoRLP::decode(&value) {
                Ok(value) => Ok(Some(value.to())),
                Err(_) => Err(StoreError::DecodeError),
            })
    }

    fn add_block_header(
        &mut self,
        _block_number: BlockNumber,
        _block_header: BlockHeader,
    ) -> Result<(), StoreError> {
        todo!()
    }

    fn get_block_header(
        &self,
        _block_number: BlockNumber,
    ) -> Result<Option<BlockHeader>, StoreError> {
        todo!()
    }

    fn add_block_body(
        &mut self,
        _block_number: BlockNumber,
        _block_body: BlockBody,
    ) -> Result<(), StoreError> {
        todo!()
    }

    fn get_block_body(&self, _block_number: BlockNumber) -> Result<Option<BlockBody>, StoreError> {
        todo!()
    }

    fn add_block_number(
        &mut self,
        _block_hash: BlockHash,
        _block_number: BlockNumber,
    ) -> Result<(), StoreError> {
        todo!()
    }

    fn get_block_number(&self, _block_hash: BlockHash) -> Result<Option<BlockNumber>, StoreError> {
        todo!()
    }

    fn set_value(&mut self, key: Key, value: Value) -> Result<(), StoreError> {
        let _ = self.values.insert(key, value);
        Ok(())
    }

    fn get_value(&self, key: Key) -> Result<Option<Vec<u8>>, StoreError> {
        Ok(self.values.get(key)?.map(|value| value.to_vec()))
    }

    fn add_transaction_location(
        &mut self,
        _transaction_hash: H256,
        _block_number: BlockNumber,
        _index: Index,
    ) -> Result<(), StoreError> {
        todo!()
    }

    fn get_transaction_location(
        &self,
        _transaction_hash: H256,
    ) -> Result<Option<(BlockNumber, Index)>, StoreError> {
        todo!()
    }
}

impl Debug for Store {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Sled Store").finish()
    }
}
