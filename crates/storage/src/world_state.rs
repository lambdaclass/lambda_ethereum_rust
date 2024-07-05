use std::collections::HashMap;

use anyhow::anyhow;
use ethereum_rust_core::{
    rlp::{decode::RLPDecode, encode::RLPEncode},
    types::{compute_storage_root, AccountInfo, AccountState},
    Address, H256,
};
use libmdbx::orm::Database;
use patricia_merkle_tree::PatriciaMerkleTree;
use sha3::{digest::core_api::CoreWrapper, Keccak256Core};

use crate::{AccountInfos, AccountStorages};

/// A Merkle Tree containing a mapping from account addresses to account states
#[allow(unused)]
#[derive(Default)]
pub struct WorldState(pub PatriciaMerkleTree<Vec<u8>, Vec<u8>, CoreWrapper<Keccak256Core>>);

#[allow(unused)]
impl WorldState {
    pub fn get_account_state(&self, addr: &Address) -> Option<AccountState> {
        self.0
            .get(&addr.encode_to_vec())
            .and_then(|encoded| AccountState::decode(encoded).ok())
    }
}

#[allow(unused)]
pub fn build_world_state(db: &Database) -> Result<WorldState, anyhow::Error> {
    let db = db.begin_read()?;
    // Fetch & Decode AccountInfos
    let mut account_infos = HashMap::<_, _>::new();
    let mut account_infos_db = db.cursor::<AccountInfos>()?;
    while let Some((rlp_address, rlp_info)) = account_infos_db.next()? {
        account_infos.insert(
            Address::decode(&rlp_address.0)?,
            AccountInfo::decode(&rlp_info.0)?,
        );
    }
    // Fetch & Decode Account Storages
    let mut account_storages = HashMap::<Address, HashMap<H256, H256>>::new();
    let mut account_storages_db = db.cursor::<AccountStorages>()?;
    while let Some((rlp_address, (rlp_storage_key, rlp_storage_value))) =
        account_storages_db.next()?
    {
        let entry = account_storages
            .entry(Address::decode(&rlp_address.0)?)
            .or_insert(Default::default());
        entry.insert(
            H256::decode(&rlp_storage_key.0)?,
            H256::decode(&rlp_storage_value.0)?,
        );
    }
    // Fill World State merkle tree
    let mut world_state = WorldState::default();
    for (addr, account_info) in account_infos {
        let storage = account_storages
            .get(&addr)
            .ok_or(anyhow!("No storage found in db for account address {addr}"))?;
        let account_state = AccountState {
            nonce: account_info.nonce,
            balance: account_info.balance,
            storage_root: compute_storage_root(storage),
            code_hash: account_info.code_hash,
        };

        world_state
            .0
            .insert(addr.encode_to_vec(), account_state.encode_to_vec());
    }

    Ok(world_state)
}

#[cfg(test)]
mod tests {
    use ethereum_rust_core::rlp::encode::RLPEncode;
    use std::str::FromStr;

    use crate::{
        account::{AccountInfoRLP, AccountStorageKeyRLP, AccountStorageValueRLP, AddressRLP},
        init_db,
    };

    use super::*;

    #[test]
    fn test_build_world_state() {
        let db = init_db(None::<String>);
        let write = db.begin_readwrite().unwrap();
        let account_info = AccountInfo {
            code_hash: H256::from_slice(
                &hex::decode("3d9209c0aa535c1b05fb6000abf2e8239ac31d12ff08b2c6db0c6ef68cf7795f")
                    .unwrap(),
            ),
            balance: 12.into(),
            nonce: 0,
        };
        let address =
            Address::from_slice(&hex::decode("a94f5374fce5edbc8e2a8697c15331677e6ebf0b").unwrap());

        // TODO: Fix bug with H256 encoding that makes these values [u8;33]
        let storage = vec![
            (
                H256::from_str(
                    "0x1000000000000000000000000000000000000000000000000000000000000022",
                )
                .unwrap(),
                H256::from_str(
                    "0xf5a5fd42d16a20302798ef6ed309979b43003d2320d9f0e8ea9831a92759fb4b",
                )
                .unwrap(),
            ),
            (
                H256::from_str(
                    "0x1000000000000000000000000000000000000000000000000000000000000038",
                )
                .unwrap(),
                H256::from_str(
                    "0xe71f0aa83cc32edfbefa9f4d3e0174ca85182eec9f3a09f6a6c0df6377a510d7",
                )
                .unwrap(),
            ),
        ];
        write
            .upsert::<AccountInfos>(
                AddressRLP(address.encode_to_vec()),
                AccountInfoRLP(account_info.encode_to_vec()),
            )
            .unwrap();
        write
            .upsert::<AccountStorages>(
                AddressRLP(address.encode_to_vec()),
                (
                    AccountStorageKeyRLP(storage[0].0.encode_to_vec().try_into().unwrap()),
                    AccountStorageValueRLP(storage[0].1.encode_to_vec().try_into().unwrap()),
                ),
            )
            .unwrap();
        write
            .upsert::<AccountStorages>(
                AddressRLP(address.encode_to_vec()),
                (
                    AccountStorageKeyRLP(storage[1].0.encode_to_vec().try_into().unwrap()),
                    AccountStorageValueRLP(storage[1].1.encode_to_vec().try_into().unwrap()),
                ),
            )
            .unwrap();
        write.commit().unwrap();

        let world_state = build_world_state(&db).unwrap();
    }
}
