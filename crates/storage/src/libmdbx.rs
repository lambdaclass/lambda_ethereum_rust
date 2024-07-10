use super::{Key, StoreEngine, Value};
use crate::rlp::{
    account::{
        AccountCodeHashRLP, AccountCodeRLP, AccountInfoRLP, AccountStorageKeyRLP,
        AccountStorageValueRLP, AddressRLP,
    },
    block::{BlockBodyRLP, BlockHeaderRLP},
    receipt::ReceiptRLP,
};
use anyhow::Result;
use ethereum_rust_core::types::AccountInfo;
use ethereum_rust_core::types::{BlockNumber, Index};
use ethereum_types::Address;
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
    pub fn new(path: &str) -> Result<Self> {
        Ok(Self {
            db: init_db(Some(path)),
        })
    }
}

impl StoreEngine for Store {
    fn add_account_info(&mut self, address: Address, account_info: AccountInfo) -> Result<()> {
        // Write account to mdbx
        {
            let txn = self.db.begin_readwrite().unwrap();
            txn.upsert::<AccountInfos>(address.into(), account_info.into())?;
            txn.commit().unwrap();
        }
        Ok(())
    }

    fn get_account_info(&self, address: Address) -> Result<Option<AccountInfo>> {
        // Read account from mdbx
        let read_value = {
            let txn = self.db.begin_read().unwrap();
            txn.get::<AccountInfos>(address.into())
        };
        Ok(read_value?.map(|a| a.to()))
    }

    fn set_value(&mut self, _key: Key, _value: Value) -> Result<()> {
        todo!()
    }

    fn get_value(&self, _key: Key) -> Result<Option<Value>> {
        todo!()
    }
}

impl Debug for Store {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Libmdbx Store").finish()
    }
}

// Define tables
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
    ( AccountStorages ) AddressRLP[AccountStorageKeyRLP] => AccountStorageValueRLP
);
table!(
    /// Account codes table.
    ( AccountCodes ) AccountCodeHashRLP => AccountCodeRLP
);
dupsort!(
    /// Receipts table.
    ( Receipts ) BlockNumber[Index] => ReceiptRLP
);

/// Initializes a new database with the provided path. If the path is `None`, the database
/// will be temporary.
pub fn init_db(path: Option<impl AsRef<Path>>) -> Database {
    let tables = [
        table_info!(Headers),
        table_info!(Bodies),
        table_info!(AccountInfos),
        table_info!(AccountStorages),
        table_info!(AccountCodes),
        table_info!(Receipts),
    ]
    .into_iter()
    .collect();
    let path = path.map(|p| p.as_ref().to_path_buf());
    Database::create(path, &tables).unwrap()
}

#[cfg(test)]
mod tests {
    use libmdbx::{
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
}
