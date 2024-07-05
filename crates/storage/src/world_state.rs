
use std::collections::HashMap;

use ethereum_rust_core::{rlp::decode::RLPDecode, types::{AccountInfo, AccountState}, Address, H256};
use libmdbx::orm::Database;

use crate::{AccountInfos, AccountStorages};


pub type WorldStateMap = HashMap<Address, AccountState>;

pub fn build_world_state(db: &Database) {
    let db = db.begin_read().unwrap();
    // Fetch & Decode AccountInfos
    let mut account_infos = HashMap::<Address, AccountInfo>::new();
    let mut account_infos_db = db.cursor::<AccountInfos>().unwrap();
    while let Some((rlp_address, rlp_info)) = account_infos_db.next().unwrap() {
        account_infos.insert(Address::decode(&rlp_address.0).unwrap(), AccountInfo::decode(&rlp_info.0).unwrap());
    };
    // Fetch & Decode Account Storages
    let mut account_storages = HashMap::<Address, HashMap<H256, H256>>::new();
    let mut account_storages_db = db.cursor::<AccountStorages>().unwrap();
    // while let Some((rlp_address, rlp_storage_value)) = account_storages_db.next().unwrap() {
        
    // };
  
}