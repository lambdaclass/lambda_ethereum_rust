use ethereum_rust_storage::{error::StoreError, Store};

use crate::rlpx::snap::{AccountRange, GetAccountRange};

pub fn process_account_range_request(
    request: GetAccountRange,
    store: Store,
) -> Result<AccountRange, StoreError> {
    let mut accounts = vec![];
    // Fetch account range
    let mut iter = store.iter_accounts(request.root_hash);
    let mut start_found = false;
    while let Some((k, v)) = iter.next() {
        dbg!(&k);
        if k >= request.limit_hash {
            break;
        }
        if k >= request.starting_hash {
            start_found = true;
        }
        if start_found {
            accounts.push((k, v.into()))
        }
    }
    let proof = store.get_account_range_proof(request.root_hash, request.starting_hash)?;

    Ok(AccountRange {
        id: request.id,
        accounts,
        proof,
    })
}
