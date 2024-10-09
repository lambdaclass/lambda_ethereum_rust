use std::cmp::Ordering;

use bytes::Bytes;
use ethereum_types::{Address, H256, U256};

use crate::{
    block::BlockEnv,
    constants::{
        init_code_cost, MAX_CODE_SIZE, TX_BASE_COST, TX_CREATE_COST, TX_DATA_COST_PER_NON_ZERO,
        TX_DATA_COST_PER_ZERO,
    },
    vm::Account,
    vm_result::{InvalidTx, VMError},
};

type AccessList = Vec<(Address, Vec<U256>)>;
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
    // Caution: If set to `None`, then nonce validation against the account's nonce is skipped: [InvalidTransaction::NonceTooHigh] and [InvalidTransaction::NonceTooLow]

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
    fn get_tx_type(&self) -> TxType {
        if let Some(_gas_price) = self.gas_price {
            if let Some(_access_list) = &self.access_list {
                TxType::AccessList
            } else {
                TxType::Legacy
            }
        } else if let Some(_max_fee_per_blob_gas) = self.max_fee_per_blob_gas {
            TxType::Blob
        } else {
            TxType::FeeMarket
        }
    }

    //  Calculates the gas that is charged before execution is started.
    fn calculate_intrinsic_cost(&self) -> u64 {
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

    fn consume_intrinsic_cost(&mut self) -> Result<u64, InvalidTx> {
        let intrinsic_cost = self.calculate_intrinsic_cost();
        if self.gas_limit >= intrinsic_cost {
            self.gas_limit -= intrinsic_cost;
            Ok(intrinsic_cost)
        } else {
            Err(InvalidTx::CallGasCostMoreThanGasLimit)
        }
    }

    /// Reference: https://github.com/ethereum/execution-specs/blob/c854868f4abf2ab0c3e8790d4c40607e0d251147/src/ethereum/cancun/fork.py#L332
    fn validate_tx_env(&self, account: &Account, block_env: &BlockEnv) -> Result<(), InvalidTx> {
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

        // let is_create = matches!(current_call_frame.to, TransactTo::Create);
        let is_create = match self.transact_to {
            TransactTo::Create => true,
            _ => false,
        };

        // if it's a create tx, check max code size
        // https://github.com/ethereum/execution-specs/blob/c854868f4abf2ab0c3e8790d4c40607e0d251147/src/ethereum/cancun/fork.py#L376
        if is_create && self.data.len() > 2 * MAX_CODE_SIZE {
            return Err(InvalidTx::CreateInitCodeSizeLimit);
        }

        // if the tx gas limit is greater than the available gas in the block
        // https://github.com/ethereum/execution-specs/blob/c854868f4abf2ab0c3e8790d4c40607e0d251147/src/ethereum/cancun/fork.py#L379
        if U256::from(self.gas_limit) > U256::from(block_env.gas_limit) {
            return Err(InvalidTx::CallerGasLimitMoreThanBlock);
        }

        // transactions from callers with deployed code should be rejected
        // this is formalized on EIP-3607: https://eips.ethereum.org/EIPS/eip-3607
        // https://github.com/ethereum/execution-specs/blob/c854868f4abf2ab0c3e8790d4c40607e0d251147/src/ethereum/cancun/fork.py#L423
        if account.has_code() {
            return Err(InvalidTx::RejectCallerWithCode);
        }

        let tx_type = self.get_tx_type();

        // if it's a fee market tx (eip-1559)
        // https://eips.ethereum.org/EIPS/eip-1559
        if tx_type == TxType::FeeMarket {
            // the max tip fee i'm willing to pay can't exceed the
            // max total fee i'm willing to pay
            // https://github.com/ethereum/execution-specs/blob/c854868f4abf2ab0c3e8790d4c40607e0d251147/src/ethereum/cancun/fork.py#L386
            if self.max_fee_per_gas < self.max_priority_fee_per_gas {
                return Err(InvalidTx::PriorityFeeGreaterThanMaxFee);
            }
            // https://github.com/ethereum/execution-specs/blob/c854868f4abf2ab0c3e8790d4c40607e0d251147/src/ethereum/cancun/fork.py#L396
            let mut max_gas_fee = U256::from(self.gas_limit)
            .checked_mul(self.gas_price)
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
            let mut max_gas_fee = U256::from(self.gas_limit)
            .checked_mul(self.gas_price)
            .ok_or(InvalidTx::OverflowPaymentInTransaction)?;
        }

        

        

        let sender = account.address;
        let tx_type = self.get_tx_type();
        let base_fee_per_gas = block_env.base_fee_per_gas;
        let effective_gas_price = U256::zero();
        let max_gas_fee = U256::zero();

        if tx_type == TxType::FeeMarket || tx_type == TxType::Blob {
            let max_fee_per_gas = self.max_fee_per_gas.unwrap();
            if max_fee_per_gas < self.max_priority_fee_per_gas.unwrap() {
                return Err(InvalidTx::MaxFeePerGasLessThanMaxPriorityFeePerGas);
            }
            if max_fee_per_gas < base_fee_per_gas {
                return Err(InvalidTx::MaxFeePerGasLessThanBlockBaseFeePerGas);
            }

            let priority_fee_per_gas = std::cmp::min(
                self.max_priority_fee_per_gas.unwrap(),
                max_fee_per_gas - base_fee_per_gas,
            );

            effective_gas_price = priority_fee_per_gas + base_fee_per_gas;
            max_gas_fee = self.gas * self.max_fee_per_gas;
        } else {
            // as it is Legacy or AccessList, gas_price is not None
            if self.gas_price.unwrap() < base_fee_per_gas {
                return Err(InvalidTx::InvalidGasPrice);
            }
            effective_gas_price = self.gas_price.unwrap();
            max_gas_fee = self.gas * self.gas_price.unwrap();
        }

        // else
        //     if tx.gas_price < base_fee_per_gas:
        //     raise InvalidBlock
        // effective_gas_price = tx.gas_price
        // max_gas_fee = tx.gas * tx.gas_price
        if let Some(max) = self.max_fee_per_blob_gas {
            let price = self.block.blob_gasprice.unwrap();
            if U256::from(price) > max {
                return Err(InvalidTx::BlobGasPriceGreaterThanMax);
            }
            if self.tx.blob_hashes.is_empty() {
                return Err(InvalidTx::EmptyBlobs);
            }
            if is_create {
                return Err(InvalidTx::BlobCreateTransaction);
            }
            for blob in self.tx.blob_hashes.iter() {
                if blob[0] != VERSIONED_HASH_VERSION_KZG {
                    return Err(InvalidTx::BlobVersionNotSupported);
                }
            }

            let num_blobs = self.tx.blob_hashes.len();
            if num_blobs > MAX_BLOB_NUMBER_PER_BLOCK as usize {
                return Err(InvalidTx::TooManyBlobs {
                    have: num_blobs,
                    max: MAX_BLOB_NUMBER_PER_BLOCK as usize,
                });
            }
        }

        // TODO: check if more validations are needed
        Ok(())
    }

    pub fn validate_transaction(
        &mut self,
        account: &Account,
        block_env: &BlockEnv,
    ) -> Result<u64, VMError> {
        let initial_gas_consumed = self.consume_intrinsic_cost()?;
        self.validate_tx_env(account, block_env)?;

        Ok(initial_gas_consumed)
    }

    // /// Checks if the transaction is valid.
    // ///
    // /// See the [execution spec] for reference.
    // ///
    // /// [execution spec]: https://github.com/ethereum/execution-specs/blob/c854868f4abf2ab0c3e8790d4c40607e0d251147/src/ethereum/cancun/fork.py#L332
    // pub fn validate_transaction(&self, account: &AccountInfo) -> Result<(), InvalidTransaction> {
    //     // if initial tx gas cost (intrinsic cost) is greater that tx limit
    //     // https://github.com/ethereum/execution-specs/blob/c854868f4abf2ab0c3e8790d4c40607e0d251147/src/ethereum/cancun/fork.py#L372
    //     // https://github.com/bluealloy/revm/blob/66adad00d8b89f1ab4057297b95b975564575fd4/crates/interpreter/src/gas/calc.rs#L362
    //     let intrinsic_cost = self.calculate_intrinsic_cost();

    //     if intrinsic_cost > self.tx.gas_limit {
    //         return Err(InvalidTransaction::CallGasCostMoreThanGasLimit);
    //     }

    //     // if nonce is None, nonce check skipped
    //     // https://github.com/ethereum/execution-specs/blob/c854868f4abf2ab0c3e8790d4c40607e0d251147/src/ethereum/cancun/fork.py#L419
    //     if let Some(tx) = self.tx.nonce {
    //         let state = account.nonce;

    //         match tx.cmp(&state) {
    //             Ordering::Greater => return Err(InvalidTransaction::NonceTooHigh { tx, state }),
    //             Ordering::Less => return Err(InvalidTransaction::NonceTooLow { tx, state }),
    //             Ordering::Equal => {}
    //         }
    //     }

    //     // if it's a create tx, check max code size
    //     // https://github.com/ethereum/execution-specs/blob/c854868f4abf2ab0c3e8790d4c40607e0d251147/src/ethereum/cancun/fork.py#L376
    //     if self.tx.is_create() && self.tx.data.len() > 2 * MAX_CODE_SIZE {
    //         return Err(InvalidTransaction::CreateInitCodeSizeLimit);
    //     }

    //     // if the tx gas limit is greater than the available gas in the block
    //     // https://github.com/ethereum/execution-specs/blob/c854868f4abf2ab0c3e8790d4c40607e0d251147/src/ethereum/cancun/fork.py#L379
    //     if U256::from(self.tx.gas_limit) > self.block.gas_limit {
    //         return Err(InvalidTransaction::CallerGasLimitMoreThanBlock);
    //     }

    //     // transactions from callers with deployed code should be rejected
    //     // this is formalized on EIP-3607: https://eips.ethereum.org/EIPS/eip-3607
    //     // https://github.com/ethereum/execution-specs/blob/c854868f4abf2ab0c3e8790d4c40607e0d251147/src/ethereum/cancun/fork.py#L423
    //     if account.has_code() {
    //         return Err(InvalidTransaction::RejectCallerWithCode);
    //     }

    //     // if it's a fee market tx (eip-1559)
    //     // https://eips.ethereum.org/EIPS/eip-1559
    //     if let Some(max_priority_fee_per_gas) = self.tx.gas_priority_fee {
    //         // the max tip fee i'm willing to pay can't exceed the
    //         // max total fee i'm willing to pay
    //         // https://github.com/ethereum/execution-specs/blob/c854868f4abf2ab0c3e8790d4c40607e0d251147/src/ethereum/cancun/fork.py#L386
    //         if self.tx.gas_price < max_priority_fee_per_gas {
    //             return Err(InvalidTransaction::PriorityFeeGreaterThanMaxFee);
    //         }
    //     }

    //     // the max fee i'm willing to pay for the tx can't be
    //     // less than the block's base fee
    //     // https://github.com/ethereum/execution-specs/blob/c854868f4abf2ab0c3e8790d4c40607e0d251147/src/ethereum/cancun/fork.py#L388
    //     if self.tx.gas_price < self.block.basefee {
    //         return Err(InvalidTransaction::GasPriceLessThanBasefee);
    //     }

    //     // https://github.com/ethereum/execution-specs/blob/c854868f4abf2ab0c3e8790d4c40607e0d251147/src/ethereum/cancun/fork.py#L396
    //     let mut max_gas_fee = U256::from(self.tx.gas_limit)
    //         .checked_mul(self.tx.gas_price)
    //         .ok_or(InvalidTransaction::OverflowPaymentInTransaction)?;

    //     // if it's a blob tx (eip-4844)
    //     // https://eips.ethereum.org/EIPS/eip-4844
    //     if let Some(max) = self.tx.max_fee_per_blob_gas {
    //         // a blob tx must have a recipient
    //         // https://eips.ethereum.org/EIPS/eip-4844#blob-transaction
    //         if self.tx.is_create() {
    //             return Err(InvalidTransaction::BlobCreateTransaction);
    //         }

    //         // there must be at least one blob hash in a blob tx
    //         // https://github.com/ethereum/execution-specs/blob/c854868f4abf2ab0c3e8790d4c40607e0d251147/src/ethereum/cancun/fork.py#L406
    //         if self.tx.blob_hashes.is_empty() {
    //             return Err(InvalidTransaction::EmptyBlobs);
    //         }

    //         // the maximum number of blobs in a block for now is 6
    //         // https://eips.ethereum.org/EIPS/eip-4844#throughput
    //         if self.tx.number_of_blobs() > MAX_BLOB_NUMBER_PER_BLOCK as u64 {
    //             return Err(InvalidTransaction::TooManyBlobs {
    //                 have: self.tx.number_of_blobs() as usize,
    //                 max: MAX_BLOB_NUMBER_PER_BLOCK as usize,
    //             });
    //         }

    //         // each blob's first byte must be `VERSIONED_HASH_VERSION_KZG` (0x01)
    //         // https://github.com/ethereum/execution-specs/blob/c854868f4abf2ab0c3e8790d4c40607e0d251147/src/ethereum/cancun/fork.py#L408
    //         for blob in self.tx.blob_hashes.iter() {
    //             if blob[0] != VERSIONED_HASH_VERSION_KZG {
    //                 return Err(InvalidTransaction::BlobVersionNotSupported);
    //             }
    //         }

    //         // check that the tx is willing to pay at least the blob gas price
    //         // https://github.com/ethereum/execution-specs/blob/c854868f4abf2ab0c3e8790d4c40607e0d251147/src/ethereum/cancun/fork.py#L412
    //         let price = self
    //             .block
    //             .blob_gasprice
    //             .expect("it's a blob tx, but the block has no blob gas price");
    //         // TODO: this should probably return an error, but it would violate
    //         // revms api.

    //         if U256::from(price) > max {
    //             return Err(InvalidTransaction::BlobGasPriceGreaterThanMax);
    //         }

    //         max_gas_fee += self.calculate_total_blob_gas() * max;
    //     } else if !self.tx.blob_hashes.is_empty() {
    //         return Err(InvalidTransaction::BlobVersionedHashesNotSupported);
    //     }

    //     let fee = max_gas_fee
    //         .checked_add(self.tx.value)
    //         .ok_or(InvalidTransaction::OverflowPaymentInTransaction)?;

    //     if fee > account.balance {
    //         return Err(InvalidTransaction::LackOfFundForMaxFee {
    //             fee: Box::new(fee),
    //             balance: Box::new(account.balance),
    //         });
    //     }

    //     Ok(())
    // }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TxType {
    Legacy,
    AccessList,
    Blob,
    FeeMarket,
}

impl Default for TxType {
    fn default() -> Self {
        TxType::Legacy
    }
}
