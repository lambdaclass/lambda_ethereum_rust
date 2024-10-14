use std::cmp::Ordering;

use bytes::Bytes;
use ethereum_types::{Address, H256, U256};

use crate::{
    block::{BlockEnv, BLOB_GASPRICE_UPDATE_FRACTION, GAS_PER_BLOB, MIN_BLOB_GASPRICE},
    constants::{
        init_code_cost, MAX_BLOB_NUMBER_PER_BLOCK, MAX_CREATE_CODE_SIZE, TX_BASE_COST,
        TX_CREATE_COST, TX_DATA_COST_PER_NON_ZERO, TX_DATA_COST_PER_ZERO,
        VERSIONED_HASH_VERSION_KZG,
    },
    vm::Account,
    vm_result::{InvalidTx, VMError},
};

pub type AccessList = Vec<(Address, Vec<U256>)>;
// type VersionedHash = H32;

/// Transaction destination.
#[derive(Clone, Debug, Default)]
pub enum TransactTo {
    /// Simple call to an address.
    Call(Address),
    /// Contract creation.
    #[default]
    Create,
}

/// The transaction environment.
#[derive(Clone, Debug, Default)]
pub struct TxEnv {
    /// Caller aka Author aka transaction signer.
    pub msg_sender: Address,
    /// The gas limit of the transaction.
    pub gas_limit: u64,
    /// The gas price of the transaction.
    pub gas_price: Option<U256>,
    /// The destination of the transaction.
    pub transact_to: TransactTo,
    /// The value sent to `transact_to`.
    pub value: U256,
    // The data of the transaction.
    pub data: Bytes,
    // The nonce of the transaction.
    pub nonce: Option<u64>,
    // Caution: If set to `None`, then nonce validation against the account's nonce is skipped: [InvalidTx::NonceTooHigh] and [InvalidTx::NonceTooLow]

    // The chain ID of the transaction. If set to `None`, no checks are performed.
    //
    // Incorporated as part of the Spurious Dragon upgrade via [EIP-155].
    //
    // [EIP-155]: https://eips.ethereum.org/EIPS/eip-155
    pub chain_id: Option<u64>,

    // A list of addresses and storage keys that the transaction plans to access.
    //
    // Added in [EIP-2930].
    //
    // [EIP-2930]: https://eips.ethereum.org/EIPS/eip-2930
    pub access_list: Option<AccessList>,

    /// Maximum number of Wei to be paid to the block's recipient
    /// as an incentive to include the transaction.
    ///
    /// Incorporated as part of the London upgrade via [EIP-1559].
    ///
    /// [EIP-1559]: https://eips.ethereum.org/EIPS/eip-155
    pub max_priority_fee_per_gas: Option<U256>,

    // The list of blob versioned hashes. Per EIP there should be at least
    // one blob present if [`Self::max_fee_per_blob_gas`] is `Some`.
    //
    // Incorporated as part of the Cancun upgrade via [EIP-4844].
    //
    // [EIP-4844]: https://eips.ethereum.org/EIPS/eip-4844
    pub blob_hashes: Vec<H256>,
    // The max fee per blob gas.
    //
    // Incorporated as part of the Cancun upgrade via [EIP-4844].
    //
    // [EIP-4844]: https://eips.ethereum.org/EIPS/eip-4844
    pub max_fee_per_blob_gas: Option<U256>,
    pub max_fee_per_gas: Option<U256>,
}

impl TxEnv {
    pub fn get_tx_type(&self) -> TxType {
        if self.gas_price.is_some() && self.access_list.is_some() {
            TxType::AccessList
        } else if self.gas_price.is_some() {
            TxType::Legacy
        } else if self.max_fee_per_blob_gas.is_some() {
            TxType::Blob
        } else {
            TxType::FeeMarket
        }
    }

    //  Calculates the gas that is charged before execution is started.
    pub fn calculate_intrinsic_cost(&self) -> u64 {
        let data_cost = self.data.clone().iter().fold(0, |acc, byte| {
            acc + if *byte == 0 {
                TX_DATA_COST_PER_ZERO
            } else {
                TX_DATA_COST_PER_NON_ZERO
            }
        });

        let create_cost = match self.transact_to {
            TransactTo::Create => TX_CREATE_COST + init_code_cost(self.data.len()),
            TransactTo::Call(_) => 0,
        };

        // TODO: implement with access lists
        // let access_list_cost = access_list_cost(&current_call_frame.access_list);

        TX_BASE_COST + data_cost + create_cost
    }

    pub fn consume_intrinsic_cost(&mut self) -> Result<u64, InvalidTx> {
        let intrinsic_cost = self.calculate_intrinsic_cost();

        self.gas_limit = self
            .gas_limit
            .checked_sub(intrinsic_cost)
            .ok_or(InvalidTx::CallGasCostMoreThanGasLimit)?;

        Ok(intrinsic_cost)
    }

    /// Reference: https://github.com/ethereum/execution-specs/blob/c854868f4abf2ab0c3e8790d4c40607e0d251147/src/ethereum/cancun/fork.py#L332
    pub fn validate_tx_env(
        &self,
        account: &Account,
        block_env: &BlockEnv,
    ) -> Result<(), InvalidTx> {
        // if nonce is None, nonce check skipped
        // https://github.com/ethereum/execution-specs/blob/c854868f4abf2ab0c3e8790d4c40607e0d251147/src/ethereum/cancun/fork.py#L419
        if let Some(tx) = self.nonce {
            let state = account.nonce;

            match tx.cmp(&state) {
                Ordering::Greater => return Err(InvalidTx::NonceTooHigh { tx, state }),
                Ordering::Less => return Err(InvalidTx::NonceTooLow { tx, state }),
                Ordering::Equal => {}
            }
        }

        let is_create = matches!(self.transact_to, TransactTo::Create);

        // if it's a create tx, check max code size
        // https://github.com/ethereum/execution-specs/blob/c854868f4abf2ab0c3e8790d4c40607e0d251147/src/ethereum/cancun/fork.py#L376
        if is_create && self.data.len() > MAX_CREATE_CODE_SIZE {
            return Err(InvalidTx::CreateInitCodeSizeLimit);
        }

        // if the tx gas limit is greater than the available gas in the block
        // https://github.com/ethereum/execution-specs/blob/c854868f4abf2ab0c3e8790d4c40607e0d251147/src/ethereum/cancun/fork.py#L379
        if self.gas_limit as usize > block_env.gas_limit {
            return Err(InvalidTx::CallerGasLimitMoreThanBlock);
        }

        // transactions from callers with deployed code should be rejected
        // this is formalized on EIP-3607: https://eips.ethereum.org/EIPS/eip-3607
        // https://github.com/ethereum/execution-specs/blob/c854868f4abf2ab0c3e8790d4c40607e0d251147/src/ethereum/cancun/fork.py#L423
        if account.has_code() {
            return Err(InvalidTx::RejectCallerWithCode);
        }

        let tx_type = self.get_tx_type();

        let mut max_gas_fee = U256::zero();

        // if it's a fee market tx (eip-1559)
        // https://eips.ethereum.org/EIPS/eip-1559
        if tx_type == TxType::FeeMarket || tx_type == TxType::Blob {
            // the max tip fee i'm willing to pay can't exceed the
            // max total fee i'm willing to pay
            // https://github.com/ethereum/execution-specs/blob/c854868f4abf2ab0c3e8790d4c40607e0d251147/src/ethereum/cancun/fork.py#L386
            if self.max_fee_per_gas < self.max_priority_fee_per_gas {
                return Err(InvalidTx::PriorityFeeGreaterThanMaxFee);
            }
            // https://github.com/ethereum/execution-specs/blob/c854868f4abf2ab0c3e8790d4c40607e0d251147/src/ethereum/cancun/fork.py#L396
            // in FeeMarket || Blob
            // max_gas_fee = gas_limit * max_fee_per_gas
            max_gas_fee = U256::from(self.gas_limit)
                .checked_mul(self.max_fee_per_gas.unwrap())
                .ok_or(InvalidTx::OverflowPaymentInTransaction)?;
        }

        if tx_type == TxType::Legacy || tx_type == TxType::AccessList {
            // the max fee i'm willing to pay for the tx can't be
            // less than the block's base fee
            // https://github.com/ethereum/execution-specs/blob/c854868f4abf2ab0c3e8790d4c40607e0d251147/src/ethereum/cancun/fork.py#L388
            // in legacy and access list we have gas_price so unwrap is safe
            if self.gas_price.unwrap() < block_env.base_fee_per_gas {
                return Err(InvalidTx::GasPriceLessThanBasefee);
            }
            // https://github.com/ethereum/execution-specs/blob/c854868f4abf2ab0c3e8790d4c40607e0d251147/src/ethereum/cancun/fork.py#L396
            // in Legacy || AccessList
            // max_gas_fee = gas_limit * gas_price
            max_gas_fee = U256::from(self.gas_limit)
                .checked_mul(self.gas_price.unwrap())
                .ok_or(InvalidTx::OverflowPaymentInTransaction)?;
        }

        // if it's a blob tx (eip-4844)
        // https://eips.ethereum.org/EIPS/eip-4844
        if let Some(max_fee_per_blob_gas) = self.max_fee_per_blob_gas {
            // a blob tx must have a recipient
            // https://eips.ethereum.org/EIPS/eip-4844#blob-transaction
            if is_create {
                return Err(InvalidTx::BlobCreateTransaction);
            }

            // there must be at least one blob hash in a blob tx
            // https://github.com/ethereum/execution-specs/blob/c854868f4abf2ab0c3e8790d4c40607e0d251147/src/ethereum/cancun/fork.py#L406
            if self.blob_hashes.is_empty() {
                return Err(InvalidTx::EmptyBlobs);
            }

            // the maximum number of blobs in a block for now is 6
            // https://eips.ethereum.org/EIPS/eip-4844#throughput
            if self.number_of_blobs() > MAX_BLOB_NUMBER_PER_BLOCK as u64 {
                return Err(InvalidTx::TooManyBlobs {
                    have: self.number_of_blobs() as usize,
                    max: MAX_BLOB_NUMBER_PER_BLOCK as usize,
                });
            }

            // each blob's first byte must be `VERSIONED_HASH_VERSION_KZG` (0x01)
            // https://github.com/ethereum/execution-specs/blob/c854868f4abf2ab0c3e8790d4c40607e0d251147/src/ethereum/cancun/fork.py#L408
            for blob in self.blob_hashes.iter() {
                if blob[0] != VERSIONED_HASH_VERSION_KZG {
                    return Err(InvalidTx::BlobVersionNotSupported);
                }
            }

            // check that the tx is willing to pay at least the blob gas price
            // https://github.com/ethereum/execution-specs/blob/c854868f4abf2ab0c3e8790d4c40607e0d251147/src/ethereum/cancun/fork.py#L412
            let excess_blob_gas = block_env
                .excess_blob_gas
                .ok_or(InvalidTx::ExcessBlobGasNotSet)?;
            // TODO: this should probably return an error, but it would violate
            // revms api.

            let blob_gasprice = Self::calculate_blob_gas_price(excess_blob_gas);

            if max_fee_per_blob_gas < U256::from(blob_gasprice) {
                return Err(InvalidTx::BlobGasPriceGreaterThanMax);
            }

            max_gas_fee += self.calculate_total_blob_gas() * max_fee_per_blob_gas;
        } else if !self.blob_hashes.is_empty() {
            // if not a blob tx, but there are blob hashes, it's an error
            return Err(InvalidTx::BlobVersionedHashesNotSupported);
        }

        let fee = max_gas_fee
            .checked_add(self.value)
            .ok_or(InvalidTx::OverflowPaymentInTransaction)?;

        if fee > account.balance {
            return Err(InvalidTx::LackOfFundForMaxFee {
                fee: Box::new(fee),
                balance: Box::new(account.balance),
            });
        }

        // TODO: check if more validations are needed
        Ok(())
    }

    pub fn validate_transaction(
        &mut self,
        account: &Account,
        block_env: &BlockEnv,
    ) -> Result<u64, VMError> {
        let initial_gas_consumed = self
            .consume_intrinsic_cost()
            .map_err(|_| VMError::InvalidTransaction)?;
        self.validate_tx_env(account, block_env)
            .map_err(|_| VMError::InvalidTransaction)?;

        Ok(initial_gas_consumed)
    }

    pub fn number_of_blobs(&self) -> u64 {
        self.blob_hashes.len() as u64
    }

    fn calculate_blob_gas_price(excess_blob_gas: u64) -> u64 {
        Self::taylor_exponential(
            MIN_BLOB_GASPRICE,
            excess_blob_gas,
            BLOB_GASPRICE_UPDATE_FRACTION,
        )
    }

    fn taylor_exponential(factor: u64, numerator: u64, denominator: u64) -> u64 {
        let mut i = 1;
        let mut output = 0;
        let mut numerator_accumulated = factor * denominator;

        while numerator_accumulated > 0 {
            output += numerator_accumulated;
            numerator_accumulated = (numerator_accumulated * numerator) / (denominator * i);
            i += 1;
        }

        output / denominator
    }

    // calculates the total blob gas for the transaction, which is
    // https://github.com/ethereum/execution-specs/blob/c854868f4abf2ab0c3e8790d4c40607e0d251147/src/ethereum/cancun/vm/gas.py#L295
    pub fn calculate_total_blob_gas(&self) -> U256 {
        if self.max_fee_per_blob_gas.is_some() {
            (GAS_PER_BLOB * self.blob_hashes.len() as u64).into()
        } else {
            0.into()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum TxType {
    #[default]
    Legacy,
    AccessList,
    Blob,
    FeeMarket,
}
