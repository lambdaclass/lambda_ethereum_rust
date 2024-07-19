use ethereum_rust_core::Address;
use ethereum_rust_storage::{error::StoreError, Store};
use revm::primitives::{AccountInfo, Address as RevmAddress, Bytecode, Bytes, B256, U256};

pub struct StoreWrapper(pub Store);

impl revm::Database for StoreWrapper {
    #[doc = " The database error type."]
    type Error = StoreError;

    #[doc = " Get basic account information."]
    fn basic(&mut self, address: RevmAddress) -> Result<Option<AccountInfo>, Self::Error> {
        let acc_info = match self.0.get_account_info(Address::from(address.0.as_ref()))? {
            None => return Ok(None),
            Some(acc_info) => acc_info,
        };
        let code = self
            .0
            .get_account_code(acc_info.code_hash)?
            .map(|b| Bytecode::new_raw(Bytes(b)));

        Ok(Some(AccountInfo {
            balance: U256::from_limbs(acc_info.balance.0),
            nonce: acc_info.nonce,
            code_hash: B256::from(acc_info.code_hash.0),
            code,
        }))
    }

    #[doc = " Get account code by its hash."]
    fn code_by_hash(&mut self, code_hash: B256) -> Result<Bytecode, Self::Error> {
        todo!()
    }

    #[doc = " Get storage value of address at index."]
    fn storage(&mut self, address: RevmAddress, index: U256) -> Result<U256, Self::Error> {
        todo!()
    }

    #[doc = " Get block hash by block number."]
    fn block_hash(&mut self, number: U256) -> Result<B256, Self::Error> {
        todo!()
    }
}
