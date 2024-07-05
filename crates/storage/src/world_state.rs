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
        self.0.get(&addr.encode_to_vec()).and_then(|encoded| AccountState::decode(encoded).ok())
    }
}

#[allow(unused)]
pub fn build_world_state(db: &Database) -> Result<WorldState, anyhow::Error> {
    let db = db.begin_read()?;
    // Fetch & Decode AccountInfos
    let mut account_infos = HashMap::<_,_>::new();
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

        world_state.0.insert(addr.encode_to_vec(), account_state.encode_to_vec());
    }

    Ok(world_state)
}

#[cfg(test)]
mod tests {
    use ethereum_rust_core::rlp::encode::RLPEncode;

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
            code_hash: H256([7; 32]),
            balance: 12.into(),
            nonce: 0,
        };
        let address =
            Address::from_slice(&hex::decode("a94f5374fce5edbc8e2a8697c15331677e6ebf0b").unwrap());
        let mut address_rlp = vec![];
        address.encode(&mut address_rlp);
        let mut account_info_rlp = vec![];
        account_info.encode(&mut account_info_rlp);
        write
            .upsert::<AccountInfos>(
                AddressRLP(address_rlp.clone()),
                AccountInfoRLP(account_info_rlp),
            )
            .unwrap();
        write
            .upsert::<AccountStorages>(
                AddressRLP(address_rlp.clone()),
                (
                    AccountStorageKeyRLP([1; 32]),
                    AccountStorageValueRLP([2; 32]),
                ),
            )
            .unwrap();
        write
            .upsert::<AccountStorages>(
                AddressRLP(address_rlp),
                (
                    AccountStorageKeyRLP([2; 32]),
                    AccountStorageValueRLP([3; 32]),
                ),
            )
            .unwrap();
        write.commit().unwrap();

        let world_state = build_world_state(&db).unwrap();
    }
}
