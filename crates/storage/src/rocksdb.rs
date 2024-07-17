use super::{Key, StoreEngine, Value};
use crate::error::StoreError;
use crate::rlp::{AccountInfoRLP, AddressRLP};
use bytes::Bytes;
use ethereum_rust_core::types::{AccountInfo, BlockBody, BlockHash, BlockHeader, BlockNumber};
use ethereum_types::{Address, H256};
use libmdbx::orm::{Decodable, Encodable};
use std::fmt::Debug;
use std::sync::mpsc::{channel, sync_channel, Receiver, Sender, SyncSender};
use std::thread;
use tracing::log::error;

#[derive(Debug)]
enum StoreCommand {
    Put(DbSelector, Key, Value, SyncSender<Result<(), StoreError>>),
    Get(
        DbSelector,
        Key,
        SyncSender<Result<Option<Value>, StoreError>>,
    ),
}

#[derive(Debug)]
enum DbSelector {
    AccountInfos,
    Values,
}

#[derive(Clone)]
pub struct Store {
    command_sender: Sender<StoreCommand>,
}

impl Store {
    pub fn new(path: &str) -> Result<Self, StoreError> {
        let account_infos = rocksdb::DB::open_default(format!("{path}.account_infos.db"))?;
        let values = rocksdb::DB::open_default(format!("{path}.values.db"))?;
        let (command_sender, command_receiver): (Sender<StoreCommand>, Receiver<StoreCommand>) =
            channel();
        thread::spawn(move || {
            while let Ok(command) = command_receiver.recv() {
                match command {
                    StoreCommand::Put(db_selector, id, value, reply_to) => {
                        let db = match db_selector {
                            DbSelector::AccountInfos => &account_infos,
                            DbSelector::Values => &values,
                        };
                        let result = Ok(db
                            .put(id, value)
                            .unwrap_or_else(|e| error!("failed to write to db {}", e)));

                        reply_to.send(result).unwrap_or_else(|e| error!("{}", e));
                    }
                    StoreCommand::Get(db_selector, id, reply_to) => {
                        let db = match db_selector {
                            DbSelector::AccountInfos => &account_infos,
                            DbSelector::Values => &values,
                        };
                        let result = db.get(id).unwrap_or(None);

                        reply_to
                            .send(Ok(result))
                            .unwrap_or_else(|e| error!("{}", e));
                    }
                };
            }
        });
        Ok(Self { command_sender })
    }
}

impl StoreEngine for Store {
    fn add_account_info(
        &mut self,
        address: Address,
        account_info: AccountInfo,
    ) -> Result<(), StoreError> {
        let (reply_sender, reply_receiver) = sync_channel(0);
        let address_rlp: AddressRLP = address.into();
        let account_info_rlp: AccountInfoRLP = account_info.into();
        self.command_sender.send(StoreCommand::Put(
            DbSelector::AccountInfos,
            address_rlp.encode(),
            account_info_rlp.encode(),
            reply_sender,
        ))?;
        reply_receiver.recv()?
    }

    fn get_account_info(&self, address: Address) -> Result<Option<AccountInfo>, StoreError> {
        let (reply_sender, reply_receiver) = sync_channel(0);
        let address_rlp: AddressRLP = address.into();
        self.command_sender
            .send(StoreCommand::Get(
                DbSelector::AccountInfos,
                address_rlp.encode(),
                reply_sender,
            ))
            .unwrap();

        // TODO: properly handle errors
        reply_receiver
            .recv()??
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
        let (reply_sender, reply_receiver) = sync_channel(0);
        self.command_sender.send(StoreCommand::Put(
            DbSelector::Values,
            key,
            value,
            reply_sender,
        ))?;
        reply_receiver.recv()?
    }

    fn get_value(&self, key: Key) -> Result<Option<Vec<u8>>, StoreError> {
        let (reply_sender, reply_receiver) = sync_channel(0);

        self.command_sender
            .send(StoreCommand::Get(DbSelector::Values, key, reply_sender))
            .unwrap();

        reply_receiver.recv()?
    }

    fn add_account_code(&mut self, _code_hash: H256, _code: Bytes) -> Result<(), StoreError> {
        todo!()
    }

    fn get_account_code(&self, _code_hash: H256) -> Result<Option<Bytes>, StoreError> {
        todo!()
    }

    fn add_storage_at(
        &mut self,
        _address: Address,
        _storage_key: H256,
        _storage_value: H256,
    ) -> Result<(), StoreError> {
        todo!()
    }

    fn get_storage_at(
        &self,
        _address: Address,
        _storage_key: H256,
    ) -> Result<Option<H256>, StoreError> {
        todo!()
    }
}

impl Debug for Store {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RocksDB Store").finish()
    }
}
