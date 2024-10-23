use ethereum_rust_core::types::AccountState;
use ethereum_rust_rlp::encode::RLPEncode;
use ethereum_rust_storage::{error::StoreError, Store};

use crate::rlpx::snap::{AccountRange, AccountStateSlim, GetAccountRange};

pub fn process_account_range_request(
    request: GetAccountRange,
    store: Store,
) -> Result<AccountRange, StoreError> {
    let mut accounts = vec![];
    // Fetch account range
    let mut iter = store.iter_accounts(request.root_hash);
    let mut start_found = false;
    let mut bytes_used = 0;
    while let Some((k, v)) = iter.next() {
        if k >= request.limit_hash {
            break;
        }
        if k >= request.starting_hash {
            start_found = true;
        }
        if start_found {
            let acc = AccountStateSlim::from(v);
            bytes_used += bytes_per_entry(&acc);
            accounts.push((k, acc));
        }
        if bytes_used >= request.response_bytes {
            break;
        }
    }
    let proof = store.get_account_range_proof(request.root_hash, request.starting_hash)?;

    Ok(AccountRange {
        id: request.id,
        accounts,
        proof,
    })
}
