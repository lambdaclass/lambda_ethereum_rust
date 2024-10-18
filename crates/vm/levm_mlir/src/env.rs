use ethereum_types::{H160, H256};

use crate::{
    constants::{
        gas_cost::{
            init_code_cost, MAX_CODE_SIZE, TX_BASE_COST, TX_CREATE_COST, TX_DATA_COST_PER_NON_ZERO,
            TX_DATA_COST_PER_ZERO,
        },
        MAX_BLOB_NUMBER_PER_BLOCK, VERSIONED_HASH_VERSION_KZG,
    },
    primitives::{Address, Bytes, B256, U256},
    report::InvalidTransaction,
    utils::{access_list_cost, calc_blob_gasprice},
};

pub type AccessList = Vec<(Address, Vec<U256>)>;

#[derive(Clone, Debug)]
pub struct Environment {
    pub chain_id: u64,
    pub block_number: U256,
    pub block_coinbase_address: Address,
    pub block_timestamp: U256,
    pub block_basefee: U256,
    pub block_prevrandao: Option<B256>,
    pub block_excess_blob_gas: Option<u64>,
    pub block_blob_gasprice: Option<u128>,
    pub tx_caller: Address,
    // If this is None, the transaction is a `create` transaction, otherwise
    // it's a regular one.
    pub tx_to: Option<Address>,
    pub tx_gas_limit: u64,
    pub tx_gas_price: U256,
    pub tx_value: U256,
    pub tx_calldata: Bytes,
    pub tx_access_list: AccessList,
    pub tx_blob_hashes: Vec<B256>,
    pub tx_max_fee_per_blob_gas: Option<U256>,
}

impl Default for Environment {
    fn default() -> Self {
        Self {
            tx_to: Some(Address::zero()),
            chain_id: 1,
            block_number: U256::zero(),
            block_coinbase_address: Address::zero(),
            block_timestamp: U256::one(),
            block_basefee: U256::zero(),
            block_prevrandao: Some(H256::zero()),
            block_excess_blob_gas: Some(0),
            block_blob_gasprice: Some(0),
            tx_caller: H160::zero(),
            tx_gas_limit: i64::MAX as _,
            tx_gas_price: U256::zero(),
            tx_value: U256::zero(),
            tx_calldata: Bytes::new(),
            tx_access_list: vec![],
            tx_blob_hashes: vec![],
            tx_max_fee_per_blob_gas: None,
        }
    }
}

impl Environment {
    pub fn consume_intrinsic_cost(&mut self) -> Result<u64, InvalidTransaction> {
        let intrinsic_cost = self.calculate_intrinsic_cost();
        if self.tx_gas_limit >= intrinsic_cost {
            self.tx_gas_limit -= intrinsic_cost;
            Ok(intrinsic_cost)
        } else {
            Err(InvalidTransaction::CallGasCostMoreThanGasLimit)
        }
    }

    /// Reference: https://github.com/ethereum/execution-specs/blob/c854868f4abf2ab0c3e8790d4c40607e0d251147/src/ethereum/cancun/fork.py#L332
    pub fn validate_transaction(&mut self) -> Result<(), InvalidTransaction> {
        let is_create = self.tx_to.is_none();

        if is_create && self.tx_calldata.len() > 2 * MAX_CODE_SIZE {
            return Err(InvalidTransaction::CreateInitCodeSizeLimit);
        }
        if let Some(max) = self.tx_max_fee_per_blob_gas {
            let price = self.block_blob_gasprice.unwrap();
            if U256::from(price) > max {
                return Err(InvalidTransaction::BlobGasPriceGreaterThanMax);
            }
            if self.tx_blob_hashes.is_empty() {
                return Err(InvalidTransaction::EmptyBlobs);
            }
            if is_create {
                return Err(InvalidTransaction::BlobCreateTransaction);
            }
            for blob in self.tx_blob_hashes.iter() {
                if blob[0] != VERSIONED_HASH_VERSION_KZG {
                    return Err(InvalidTransaction::BlobVersionNotSupported);
                }
            }

            let num_blobs = self.tx_blob_hashes.len();
            if num_blobs > MAX_BLOB_NUMBER_PER_BLOCK as usize {
                return Err(InvalidTransaction::TooManyBlobs {
                    have: num_blobs,
                    max: MAX_BLOB_NUMBER_PER_BLOCK as usize,
                });
            }
        }
        // TODO: check if more validations are needed
        Ok(())
    }

    ///  Calculates the gas that is charged before execution is started.
    pub fn calculate_intrinsic_cost(&self) -> u64 {
        let data_cost = self.tx_calldata.iter().fold(0, |acc, byte| {
            acc + if *byte == 0 {
                TX_DATA_COST_PER_ZERO
            } else {
                TX_DATA_COST_PER_NON_ZERO
            }
        });
        let create_cost = match self.tx_to {
            Some(_) => 0,
            None => TX_CREATE_COST + init_code_cost(self.tx_calldata.len() as u64),
        };
        let access_list_cost = access_list_cost(&self.tx_access_list);
        TX_BASE_COST + data_cost + create_cost + access_list_cost
    }

    pub fn get_tx_code_address(&self) -> Address {
        match self.tx_to {
            Some(address) => address,
            None => self.tx_caller,
        }
    }

    pub fn set_block_blob_base_fee(&mut self, excess_blob_gas: u64) {
        self.block_excess_blob_gas = Some(excess_blob_gas);
        self.block_blob_gasprice = Some(calc_blob_gasprice(excess_blob_gas));
    }
}
