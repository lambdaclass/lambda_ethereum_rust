use super::{Key, StoreEngine, Value};
use crate::error::StoreError;
use crate::rlp::{
    AccountCodeHashRLP, AccountCodeRLP, AccountInfoRLP, AccountStorageKeyRLP,
    AccountStorageValueRLP, AddressRLP, BlockBodyRLP, BlockHeaderRLP, ReceiptRLP,
};
use anyhow::Result;
use ethereum_rust_core::types::{AccountInfo, BlockBody, BlockHeader};
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
        {
            let txn = self.db.begin_readwrite().unwrap();
            match txn.upsert::<AccountInfos>(address.into(), account_info.into()) {
                Ok(_) => txn.commit().unwrap(),
                Err(err) => return Err(StoreError::LibmdbxError(err)),
            }
        }
        Ok(())
    }

    fn get_account_info(&self, address: Address) -> Result<Option<AccountInfo>, StoreError> {
        // Read account from mdbx
        let read_value = {
            let txn = self.db.begin_read().unwrap();
            txn.get::<AccountInfos>(address.into())
        };
        match read_value {
            Ok(value) => Ok(value.map(|a| a.to())),
            Err(err) => Err(StoreError::LibmdbxError(err)),
        }
    }

    fn add_block_header(
        &mut self,
        block_number: BlockNumber,
        block_header: BlockHeader,
    ) -> std::result::Result<(), StoreError> {
        // Write block header to mdbx
        {
            let txn = self.db.begin_readwrite().unwrap();
            match txn.upsert::<Headers>(block_number.into(), block_header.into()) {
                Ok(_) => txn.commit().unwrap(),
                Err(err) => return Err(StoreError::LibmdbxError(err)),
            }
        }
        Ok(())
    }

    fn get_block_header(
        &self,
        block_number: BlockNumber,
    ) -> std::result::Result<Option<BlockHeader>, StoreError> {
        // Read block header from mdbx
        let read_value = {
            let txn = self.db.begin_read().unwrap();
            txn.get::<Headers>(block_number.into())
        };
        match read_value {
            Ok(value) => Ok(value.map(|a| a.to())),
            Err(err) => Err(StoreError::LibmdbxError(err)),
        }
    }

    fn add_block_body(
        &mut self,
        block_number: BlockNumber,
        block_body: BlockBody,
    ) -> std::result::Result<(), StoreError> {
        // Write block body to mdbx
        {
            let txn = self.db.begin_readwrite().unwrap();
            match txn.upsert::<Bodies>(block_number.into(), block_body.into()) {
                Ok(_) => txn.commit().unwrap(),
                Err(err) => return Err(StoreError::LibmdbxError(err)),
            }
        }
        Ok(())
    }

    fn get_block_body(
        &self,
        block_number: BlockNumber,
    ) -> std::result::Result<Option<BlockBody>, StoreError> {
        // Read block body from mdbx
        let read_value = {
            let txn = self.db.begin_read().unwrap();
            txn.get::<Bodies>(block_number.into())
        };
        match read_value {
            Ok(value) => Ok(value.map(|a| a.to())),
            Err(err) => Err(StoreError::LibmdbxError(err)),
        }
    }

    fn set_value(&mut self, _key: Key, _value: Value) -> Result<(), StoreError> {
        todo!()
    }

    fn get_value(&self, _key: Key) -> Result<Option<Value>, StoreError> {
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
    ( AccountStorages ) AddressRLP => (AccountStorageKeyRLP, AccountStorageValueRLP) [AccountStorageKeyRLP]
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
    use std::str::FromStr;

    use bytes::Bytes;
    use ethereum_rust_core::{
        rlp::decode::RLPDecode,
        types::{BlockBody, BlockHeader, Bloom, Transaction},
    };
    use ethereum_types::{Address, H256, U256};
    use libmdbx::{
        orm::{table, Database, Decodable, Encodable},
        table_info,
    };

    use crate::StoreEngine;

    use super::init_db;

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
    fn store_and_fetch_block() {
        // Create block
        let block_header = BlockHeader {
            parent_hash: H256::from_str(
                "0x1ac1bf1eef97dc6b03daba5af3b89881b7ae4bc1600dc434f450a9ec34d44999",
            )
            .unwrap(),
            ommers_hash: H256::from_str(
                "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
            )
            .unwrap(),
            coinbase: Address::from_str("0x2adc25665018aa1fe0e6bc666dac8fc2697ff9ba").unwrap(),
            state_root: H256::from_str(
                "0x9de6f95cb4ff4ef22a73705d6ba38c4b927c7bca9887ef5d24a734bb863218d9",
            )
            .unwrap(),
            transactions_root: H256::from_str(
                "0x578602b2b7e3a3291c3eefca3a08bc13c0d194f9845a39b6f3bcf843d9fed79d",
            )
            .unwrap(),
            receipt_root: H256::from_str(
                "0x035d56bac3f47246c5eed0e6642ca40dc262f9144b582f058bc23ded72aa72fa",
            )
            .unwrap(),
            logs_bloom: Bloom::from([0; 256]),
            difficulty: U256::zero(),
            number: 1,
            gas_limit: 0x016345785d8a0000,
            gas_used: 0xa8de,
            timestamp: 0x03e8,
            extra_data: Bytes::new(),
            prev_randao: H256::zero(),
            nonce: 0x0000000000000000,
            base_fee_per_gas: 0x07,
            withdrawals_root: H256::from_str(
                "0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421",
            )
            .unwrap(),
            blob_gas_used: 0x00,
            excess_blob_gas: 0x00,
            parent_beacon_block_root: H256::zero(),
        };
        let block_body = BlockBody {
            transactions: vec![Transaction::decode(&hex::decode("02f86c8330182480114e82f618946177843db3138ae69679a54b95cf345ed759450d870aa87bee53800080c080a0151ccc02146b9b11adf516e6787b59acae3e76544fdcd75e77e67c6b598ce65da064c5dd5aae2fbb535830ebbdad0234975cd7ece3562013b63ea18cc0df6c97d4").unwrap()).unwrap(),
            Transaction::decode(&hex::decode("f86d80843baa0c4082f618946177843db3138ae69679a54b95cf345ed759450d870aa87bee538000808360306ba0151ccc02146b9b11adf516e6787b59acae3e76544fdcd75e77e67c6b598ce65da064c5dd5aae2fbb535830ebbdad0234975cd7ece3562013b63ea18cc0df6c97d4").unwrap()).unwrap()],
            ommers: Default::default(),
            withdrawals: Default::default(),
        };

        let block_number = 6;

        let mut storage = super::Store {
            db: init_db(None::<String>),
        };

        storage
            .add_block_body(block_number, block_body.clone())
            .unwrap();
        storage
            .add_block_header(block_number, block_header.clone())
            .unwrap();

        let fetched_body = storage.get_block_body(block_number).unwrap().unwrap();
        let fetched_header = storage.get_block_header(block_number).unwrap().unwrap();

        assert_eq!(block_body, fetched_body);
        assert_eq!(block_header, fetched_header);
    }
}
