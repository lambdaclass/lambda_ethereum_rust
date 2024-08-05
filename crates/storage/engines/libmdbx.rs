use super::api::StoreEngine;
use crate::error::StoreError;
use crate::rlp::{
    AccountCodeHashRLP, AccountCodeRLP, AccountInfoRLP, AddressRLP, BlockBodyRLP, BlockHashRLP,
    BlockHeaderRLP, ReceiptRLP, TransactionHashRLP,
};
use anyhow::Result;
use bytes::Bytes;
use ethereum_rust_core::rlp::decode::RLPDecode;
use ethereum_rust_core::rlp::encode::RLPEncode;
use ethereum_rust_core::types::{
    AccountInfo, BlockBody, BlockHash, BlockHeader, BlockNumber, ChainConfig, Index, Receipt,
};
use ethereum_types::{Address, H256, U256};
use libmdbx::orm::{Decodable, Encodable};
use libmdbx::{
    dupsort,
    orm::{table, Database},
    table_info,
};
use std::fmt::{Debug, Formatter};
use std::path::Path;

pub struct Store {
    db: Database,
}

impl Store {
    pub fn new(path: &str) -> Result<Self, StoreError> {
        Ok(Self {
            db: init_db(Some(path)),
        })
    }

    // Helper method to write into a libmdx table
    fn write<T: libmdbx::orm::Table>(
        &self,
        key: T::Key,
        value: T::Value,
    ) -> Result<(), StoreError> {
        let txn = self
            .db
            .begin_readwrite()
            .map_err(StoreError::LibmdbxError)?;
        txn.upsert::<T>(key, value)
            .map_err(StoreError::LibmdbxError)?;
        txn.commit().map_err(StoreError::LibmdbxError)
    }

    // Helper method to read from a libmdx table
    fn read<T: libmdbx::orm::Table>(&self, key: T::Key) -> Result<Option<T::Value>, StoreError> {
        let txn = self.db.begin_read().map_err(StoreError::LibmdbxError)?;
        txn.get::<T>(key).map_err(StoreError::LibmdbxError)
    }

    // Helper method to remove an entry from a libmdx table
    fn remove<T: libmdbx::orm::Table>(&self, key: T::Key) -> Result<(), StoreError> {
        let txn = self
            .db
            .begin_readwrite()
            .map_err(StoreError::LibmdbxError)?;
        txn.delete::<T>(key, None)
            .map_err(StoreError::LibmdbxError)?;
        txn.commit().map_err(StoreError::LibmdbxError)
    }
}

impl StoreEngine for Store {
    fn add_account_info(
        &mut self,
        address: Address,
        account_info: AccountInfo,
    ) -> Result<(), StoreError> {
        self.write::<AccountInfos>(address.into(), account_info.into())
    }

    fn get_account_info(&self, address: Address) -> Result<Option<AccountInfo>, StoreError> {
        Ok(self.read::<AccountInfos>(address.into())?.map(|a| a.to()))
    }

    fn remove_account_info(&mut self, address: Address) -> Result<(), StoreError> {
        self.remove::<AccountInfos>(address.into())
    }

    fn add_block_header(
        &mut self,
        block_number: BlockNumber,
        block_header: BlockHeader,
    ) -> std::result::Result<(), StoreError> {
        self.write::<Headers>(block_number, block_header.into())
    }

    fn get_block_header(
        &self,
        block_number: BlockNumber,
    ) -> Result<Option<BlockHeader>, StoreError> {
        Ok(self.read::<Headers>(block_number)?.map(|a| a.to()))
    }

    fn add_block_body(
        &mut self,
        block_number: BlockNumber,
        block_body: BlockBody,
    ) -> std::result::Result<(), StoreError> {
        self.write::<Bodies>(block_number, block_body.into())
    }

    fn get_block_body(
        &self,
        block_number: BlockNumber,
    ) -> std::result::Result<Option<BlockBody>, StoreError> {
        Ok(self.read::<Bodies>(block_number)?.map(|b| b.to()))
    }

    fn add_block_number(
        &mut self,
        block_hash: BlockHash,
        block_number: BlockNumber,
    ) -> std::result::Result<(), StoreError> {
        self.write::<BlockNumbers>(block_hash.into(), block_number)
    }

    fn get_block_number(
        &self,
        block_hash: BlockHash,
    ) -> std::result::Result<Option<BlockNumber>, StoreError> {
        self.read::<BlockNumbers>(block_hash.into())
    }

    fn add_account_code(&mut self, code_hash: H256, code: Bytes) -> Result<(), StoreError> {
        self.write::<AccountCodes>(code_hash.into(), code.into())
    }

    fn get_account_code(&self, code_hash: H256) -> Result<Option<Bytes>, StoreError> {
        Ok(self.read::<AccountCodes>(code_hash.into())?.map(|b| b.to()))
    }

    fn add_receipt(
        &mut self,
        block_number: BlockNumber,
        index: Index,
        receipt: Receipt,
    ) -> Result<(), StoreError> {
        self.write::<Receipts>((block_number, index), receipt.into())
    }

    fn get_receipt(
        &self,
        block_number: BlockNumber,
        index: Index,
    ) -> Result<Option<Receipt>, StoreError> {
        Ok(self
            .read::<Receipts>((block_number, index))?
            .map(|r| r.to()))
    }

    fn add_transaction_location(
        &mut self,
        transaction_hash: H256,
        block_number: BlockNumber,
        index: Index,
    ) -> Result<(), StoreError> {
        self.write::<TransactionLocations>(transaction_hash.into(), (block_number, index))
    }

    fn get_transaction_location(
        &self,
        transaction_hash: H256,
    ) -> Result<Option<(BlockNumber, Index)>, StoreError> {
        self.read::<TransactionLocations>(transaction_hash.into())
    }

    fn add_storage_at(
        &mut self,
        address: Address,
        storage_key: H256,
        storage_value: H256,
    ) -> Result<(), StoreError> {
        self.write::<AccountStorages>(address.into(), (storage_key.into(), storage_value.into()))
    }

    fn get_storage_at(
        &self,
        address: Address,
        storage_key: H256,
    ) -> std::result::Result<Option<H256>, StoreError> {
        // Read storage from mdbx
        let txn = self.db.begin_read().map_err(StoreError::LibmdbxError)?;
        let mut cursor = txn
            .cursor::<AccountStorages>()
            .map_err(StoreError::LibmdbxError)?;
        Ok(cursor
            .seek_value(address.into(), storage_key.into())
            .map_err(StoreError::LibmdbxError)?
            .map(|s| s.1.into()))
    }

    fn remove_account_storage(&mut self, address: Address) -> Result<(), StoreError> {
        self.remove::<AccountStorages>(address.into())
    }

    fn set_chain_config(&mut self, chain_config: &ChainConfig) -> Result<(), StoreError> {
        // Store cancun timestamp
        if let Some(cancun_time) = chain_config.cancun_time {
            self.write::<ChainData>(ChainDataIndex::CancunTime, cancun_time.encode_to_vec())?;
        };
        // Store chain id
        self.write::<ChainData>(
            ChainDataIndex::ChainId,
            chain_config.chain_id.encode_to_vec(),
        )
    }

    fn get_chain_id(&self) -> Result<Option<U256>, StoreError> {
        match self.read::<ChainData>(ChainDataIndex::ChainId)? {
            None => Ok(None),
            Some(ref rlp) => RLPDecode::decode(rlp)
                .map(Some)
                .map_err(|_| StoreError::DecodeError),
        }
    }

    fn get_cancun_time(&self) -> Result<Option<u64>, StoreError> {
        match self.read::<ChainData>(ChainDataIndex::CancunTime)? {
            None => Ok(None),
            Some(ref rlp) => RLPDecode::decode(rlp)
                .map(Some)
                .map_err(|_| StoreError::DecodeError),
        }
    }
}

impl Debug for Store {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Libmdbx Store").finish()
    }
}

// Define tables

table!(
    /// Block hash to number table.
    ( BlockNumbers ) BlockHashRLP => BlockNumber
);

table!(
    /// Block headers table.
    ( Headers ) BlockNumber => BlockHeaderRLP
);
table!(
    /// Block bodies table.
    ( Bodies ) BlockNumber => BlockBodyRLP
);
table!(
    /// Account infos table.
    ( AccountInfos ) AddressRLP => AccountInfoRLP
);
dupsort!(
    /// Account storages table.
    ( AccountStorages ) AddressRLP => (AccountStorageKeyBytes, AccountStorageValueBytes) [AccountStorageKeyBytes]
);
table!(
    /// Account codes table.
    ( AccountCodes ) AccountCodeHashRLP => AccountCodeRLP
);
dupsort!(
    /// Receipts table.
    ( Receipts ) (BlockNumber, Index)[Index] => ReceiptRLP
);

table!(
    /// Transaction locations table.
    ( TransactionLocations ) TransactionHashRLP => (BlockNumber, Index)
);

table!(
    /// Stores chain data, each value is unique and stored as its rlp encoding
    /// See [ChainDataIndex] for available chain values
    ( ChainData ) ChainDataIndex => Vec<u8>
);

// Storage values are stored as bytes instead of using their rlp encoding
// As they are stored in a dupsort table, they need to have a fixed size, and encoding them doesn't preserve their size
pub struct AccountStorageKeyBytes(pub [u8; 32]);
pub struct AccountStorageValueBytes(pub [u8; 32]);

impl Encodable for AccountStorageKeyBytes {
    type Encoded = [u8; 32];

    fn encode(self) -> Self::Encoded {
        self.0
    }
}

impl Decodable for AccountStorageKeyBytes {
    fn decode(b: &[u8]) -> anyhow::Result<Self> {
        Ok(AccountStorageKeyBytes(b.try_into()?))
    }
}

impl Encodable for AccountStorageValueBytes {
    type Encoded = [u8; 32];

    fn encode(self) -> Self::Encoded {
        self.0
    }
}

impl Decodable for AccountStorageValueBytes {
    fn decode(b: &[u8]) -> anyhow::Result<Self> {
        Ok(AccountStorageValueBytes(b.try_into()?))
    }
}

impl From<H256> for AccountStorageKeyBytes {
    fn from(value: H256) -> Self {
        AccountStorageKeyBytes(value.0)
    }
}

impl From<H256> for AccountStorageValueBytes {
    fn from(value: H256) -> Self {
        AccountStorageValueBytes(value.0)
    }
}

impl From<AccountStorageValueBytes> for H256 {
    fn from(value: AccountStorageValueBytes) -> Self {
        H256(value.0)
    }
}

/// Represents the key for each unique value of the chain data stored in the db
// (TODO: Remove this comment once full) Will store chain-specific data such as chain id and latest finalized/pending/safe block number
pub enum ChainDataIndex {
    ChainId = 0,
    CancunTime = 1,
}

impl Encodable for ChainDataIndex {
    type Encoded = [u8; 4];

    fn encode(self) -> Self::Encoded {
        (self as u32).encode()
    }
}

/// Initializes a new database with the provided path. If the path is `None`, the database
/// will be temporary.
pub fn init_db(path: Option<impl AsRef<Path>>) -> Database {
    let tables = [
        table_info!(BlockNumbers),
        table_info!(Headers),
        table_info!(Bodies),
        table_info!(AccountInfos),
        table_info!(AccountStorages),
        table_info!(AccountCodes),
        table_info!(Receipts),
        table_info!(TransactionLocations),
        table_info!(ChainData),
    ]
    .into_iter()
    .collect();
    let path = path.map(|p| p.as_ref().to_path_buf());
    Database::create(path, &tables).unwrap()
}

#[cfg(test)]
mod tests {
    use libmdbx::{
        dupsort,
        orm::{table, Database, Decodable, Encodable},
        table_info,
    };

    #[test]
    fn mdbx_smoke_test() {
        // Declare tables used for the smoke test
        table!(
            /// Example table.
            ( Example ) String => String
        );

        // Assemble database chart
        let tables = [table_info!(Example)].into_iter().collect();

        let key = "Hello".to_string();
        let value = "World!".to_string();

        let db = Database::create(None, &tables).unwrap();

        // Write values
        {
            let txn = db.begin_readwrite().unwrap();
            txn.upsert::<Example>(key.clone(), value.clone()).unwrap();
            txn.commit().unwrap();
        }
        // Read written values
        let read_value = {
            let txn = db.begin_read().unwrap();
            txn.get::<Example>(key).unwrap()
        };
        assert_eq!(read_value, Some(value));
    }

    #[test]
    fn mdbx_structs_smoke_test() {
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub struct ExampleKey([u8; 32]);

        impl Encodable for ExampleKey {
            type Encoded = [u8; 32];

            fn encode(self) -> Self::Encoded {
                Encodable::encode(self.0)
            }
        }

        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub struct ExampleValue {
            x: u64,
            y: [u8; 32],
        }

        impl Encodable for ExampleValue {
            type Encoded = [u8; 40];

            fn encode(self) -> Self::Encoded {
                let mut encoded = [0u8; 40];
                encoded[..8].copy_from_slice(&self.x.to_ne_bytes());
                encoded[8..].copy_from_slice(&self.y);
                encoded
            }
        }

        impl Decodable for ExampleValue {
            fn decode(b: &[u8]) -> anyhow::Result<Self> {
                let x = u64::from_ne_bytes(b[..8].try_into()?);
                let y = b[8..].try_into()?;
                Ok(Self { x, y })
            }
        }

        // Declare tables used for the smoke test
        table!(
            /// Example table.
            ( StructsExample ) ExampleKey => ExampleValue
        );

        // Assemble database chart
        let tables = [table_info!(StructsExample)].into_iter().collect();
        let key = ExampleKey([151; 32]);
        let value = ExampleValue { x: 42, y: [42; 32] };

        let db = Database::create(None, &tables).unwrap();

        // Write values
        {
            let txn = db.begin_readwrite().unwrap();
            txn.upsert::<StructsExample>(key, value).unwrap();
            txn.commit().unwrap();
        }
        // Read written values
        let read_value = {
            let txn = db.begin_read().unwrap();
            txn.get::<StructsExample>(key).unwrap()
        };
        assert_eq!(read_value, Some(value));
    }

    #[test]
    fn mdbx_dupsort_smoke_test() {
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub struct ExampleKey(u8);

        impl Encodable for ExampleKey {
            type Encoded = [u8; 1];

            fn encode(self) -> Self::Encoded {
                [self.0]
            }
        }
        impl Decodable for ExampleKey {
            fn decode(b: &[u8]) -> anyhow::Result<Self> {
                if b.len() != 1 {
                    anyhow::bail!("Invalid length");
                }
                Ok(Self(b[0]))
            }
        }

        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub struct ExampleValue {
            x: u64,
            y: [u8; 32],
        }

        impl Encodable for ExampleValue {
            type Encoded = [u8; 40];

            fn encode(self) -> Self::Encoded {
                let mut encoded = [0u8; 40];
                encoded[..8].copy_from_slice(&self.x.to_ne_bytes());
                encoded[8..].copy_from_slice(&self.y);
                encoded
            }
        }

        impl Decodable for ExampleValue {
            fn decode(b: &[u8]) -> anyhow::Result<Self> {
                let x = u64::from_ne_bytes(b[..8].try_into()?);
                let y = b[8..].try_into()?;
                Ok(Self { x, y })
            }
        }

        // Declare tables used for the smoke test
        dupsort!(
            /// Example table.
            ( DupsortExample ) ExampleKey => (ExampleKey, ExampleValue) [ExampleKey]
        );

        // Assemble database chart
        let tables = [table_info!(DupsortExample)].into_iter().collect();
        let key = ExampleKey(151);
        let subkey1 = ExampleKey(16);
        let subkey2 = ExampleKey(42);
        let value = ExampleValue { x: 42, y: [42; 32] };

        let db = Database::create(None, &tables).unwrap();

        // Write values
        {
            let txn = db.begin_readwrite().unwrap();
            txn.upsert::<DupsortExample>(key, (subkey1, value)).unwrap();
            txn.upsert::<DupsortExample>(key, (subkey2, value)).unwrap();
            txn.commit().unwrap();
        }
        // Read written values
        {
            let txn = db.begin_read().unwrap();
            let mut cursor = txn.cursor::<DupsortExample>().unwrap();
            let value1 = cursor.seek_exact(key).unwrap().unwrap();
            assert_eq!(value1, (key, (subkey1, value)));
            let value2 = cursor.seek_value(key, subkey2).unwrap().unwrap();
            assert_eq!(value2, (subkey2, value));
        };
    }
}
