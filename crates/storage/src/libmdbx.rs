use super::{Key, StoreEngine, Value};
use crate::error::StoreError;
use crate::rlp::{
    AccountCodeHashRLP, AccountCodeRLP, AccountInfoRLP, AccountStorageKeyRLP,
    AccountStorageValueRLP, AddressRLP, BlockBodyRLP, BlockHashRLP, BlockHeaderRLP, ReceiptRLP,
    TransactionHashRLP,
};
use anyhow::Result;
use bytes::Bytes;
use ethereum_rust_core::types::{
    AccountInfo, BlockBody, BlockHash, BlockHeader, BlockNumber, Index, Receipt,
};
use ethereum_types::{Address, H256};
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
}

impl StoreEngine for Store {
    fn add_account_info(
        &mut self,
        address: Address,
        account_info: AccountInfo,
    ) -> Result<(), StoreError> {
        // Write account to mdbx
        let txn = self
            .db
            .begin_readwrite()
            .map_err(StoreError::LibmdbxError)?;
        txn.upsert::<AccountInfos>(address.into(), account_info.into())
            .map_err(StoreError::LibmdbxError)?;
        txn.commit().map_err(StoreError::LibmdbxError)
    }

    fn get_account_info(&self, address: Address) -> Result<Option<AccountInfo>, StoreError> {
        // Read account from mdbx
        let txn = self.db.begin_read().map_err(StoreError::LibmdbxError)?;
        Ok(txn
            .get::<AccountInfos>(address.into())
            .map_err(StoreError::LibmdbxError)?
            .map(|a| a.to()))
    }

    fn add_block_header(
        &mut self,
        block_number: BlockNumber,
        block_header: BlockHeader,
    ) -> std::result::Result<(), StoreError> {
        // Write block header to mdbx
        let txn = self
            .db
            .begin_readwrite()
            .map_err(StoreError::LibmdbxError)?;
        txn.upsert::<Headers>(block_number, block_header.into())
            .map_err(StoreError::LibmdbxError)?;
        txn.commit().map_err(StoreError::LibmdbxError)
    }

    fn get_block_header(
        &self,
        block_number: BlockNumber,
    ) -> std::result::Result<Option<BlockHeader>, StoreError> {
        // Read block header from mdbx
        let txn = self.db.begin_read().map_err(StoreError::LibmdbxError)?;
        Ok(txn
            .get::<Headers>(block_number)
            .map_err(StoreError::LibmdbxError)?
            .map(|h| h.to()))
    }

    fn add_block_body(
        &mut self,
        block_number: BlockNumber,
        block_body: BlockBody,
    ) -> std::result::Result<(), StoreError> {
        // Write block body to mdbx
        let txn = self
            .db
            .begin_readwrite()
            .map_err(StoreError::LibmdbxError)?;
        txn.upsert::<Bodies>(block_number, block_body.into())
            .map_err(StoreError::LibmdbxError)?;
        txn.commit().map_err(StoreError::LibmdbxError)
    }

    fn get_block_body(
        &self,
        block_number: BlockNumber,
    ) -> std::result::Result<Option<BlockBody>, StoreError> {
        // Read block body from mdbx
        let txn = self.db.begin_read().map_err(StoreError::LibmdbxError)?;
        Ok(txn
            .get::<Bodies>(block_number)
            .map_err(StoreError::LibmdbxError)?
            .map(|b| b.to()))
    }

    fn add_block_number(
        &mut self,
        block_hash: BlockHash,
        block_number: BlockNumber,
    ) -> std::result::Result<(), StoreError> {
        // Write block number to mdbx
        let txn = self
            .db
            .begin_readwrite()
            .map_err(StoreError::LibmdbxError)?;
        txn.upsert::<BlockNumbers>(block_hash.into(), block_number)
            .map_err(StoreError::LibmdbxError)?;
        txn.commit().map_err(StoreError::LibmdbxError)
    }

    fn get_block_number(
        &self,
        block_hash: BlockHash,
    ) -> std::result::Result<Option<BlockNumber>, StoreError> {
        // Read block number from mdbx
        let txn = self.db.begin_read().map_err(StoreError::LibmdbxError)?;
        txn.get::<BlockNumbers>(block_hash.into())
            .map_err(StoreError::LibmdbxError)
    }

    fn set_value(&mut self, _key: Key, _value: Value) -> Result<(), StoreError> {
        todo!()
    }

    fn get_value(&self, _key: Key) -> Result<Option<Value>, StoreError> {
        todo!()
    }

    fn add_account_code(&mut self, code_hash: H256, code: Bytes) -> Result<(), StoreError> {
        // Write account code to mdbx
        let txn = self
            .db
            .begin_readwrite()
            .map_err(StoreError::LibmdbxError)?;
        txn.upsert::<AccountCodes>(code_hash.into(), code.into())
            .map_err(StoreError::LibmdbxError)?;
        txn.commit().map_err(StoreError::LibmdbxError)
    }

    fn get_account_code(&self, code_hash: H256) -> Result<Option<Bytes>, StoreError> {
        // Read account code from mdbx
        let txn = self.db.begin_read().map_err(StoreError::LibmdbxError)?;
        Ok(txn
            .get::<AccountCodes>(code_hash.into())
            .map_err(StoreError::LibmdbxError)?
            .map(|b| b.to()))
    }

    fn add_receipt(
        &mut self,
        block_number: BlockNumber,
        index: Index,
        receipt: Receipt,
    ) -> Result<(), StoreError> {
        // Write block number to mdbx
        let txn = self
            .db
            .begin_readwrite()
            .map_err(StoreError::LibmdbxError)?;
        txn.upsert::<Receipts>((block_number, index), receipt.into())
            .map_err(StoreError::LibmdbxError)?;
        txn.commit().map_err(StoreError::LibmdbxError)
    }

    fn get_receipt(
        &self,
        block_number: BlockNumber,
        index: Index,
    ) -> Result<Option<Receipt>, StoreError> {
        // Read block number from mdbx
        let txn = self.db.begin_read().map_err(StoreError::LibmdbxError)?;
        Ok(txn
            .get::<Receipts>((block_number, index))
            .map_err(StoreError::LibmdbxError)?
            .map(|r| r.to()))
    }

    fn add_transaction_location(
        &mut self,
        transaction_hash: H256,
        block_number: BlockNumber,
        index: Index,
    ) -> Result<(), StoreError> {
        // Write block number to mdbx
        let txn = self
            .db
            .begin_readwrite()
            .map_err(StoreError::LibmdbxError)?;
        txn.upsert::<TransactionLocations>(transaction_hash.into(), (block_number, index))
            .map_err(StoreError::LibmdbxError)?;
        txn.commit().map_err(StoreError::LibmdbxError)
    }

    fn get_transaction_location(
        &self,
        transaction_hash: H256,
    ) -> Result<Option<(BlockNumber, Index)>, StoreError> {
        // Read tx location from mdbx
        let txn = self.db.begin_read().map_err(StoreError::LibmdbxError)?;
        txn.get::<TransactionLocations>(transaction_hash.into())
            .map_err(StoreError::LibmdbxError)
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
    ( AccountStorages ) AddressRLP => (AccountStorageKeyRLP, AccountStorageValueRLP) [AccountStorageKeyRLP]
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
