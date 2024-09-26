use crate::{
    constants::{
        gas_cost::{
            init_code_cost, MAX_CODE_SIZE, TX_BASE_COST, TX_CREATE_COST, TX_DATA_COST_PER_NON_ZERO,
            TX_DATA_COST_PER_ZERO,
        },
        MAX_BLOB_NUMBER_PER_BLOCK, VERSIONED_HASH_VERSION_KZG,
    },
    primitives::{Address, Bytes, B256, U256},
    result::InvalidTransaction,
    utils::{access_list_cost, calc_blob_gasprice},
};

pub type AccessList = Vec<(Address, Vec<U256>)>;

//This Env struct contains configuration information about the EVM, the block containing the transaction, and the transaction itself.
//Structs inspired by the REVM primitives
//-> https://github.com/bluealloy/revm/blob/main/crates/primitives/src/env.rs
#[derive(Clone, Debug, Default)]
pub struct Env {
    /// Configuration of the EVM itself.
    pub cfg: CfgEnv,
    /// Configuration of the block the transaction is in.
    pub block: BlockEnv,
    /// Configuration of the transaction that is being executed.
    pub tx: TxEnv,
}

impl Env {
    pub fn consume_intrinsic_cost(&mut self) -> Result<u64, InvalidTransaction> {
        let intrinsic_cost = self.calculate_intrinsic_cost();
        if self.tx.gas_limit >= intrinsic_cost {
            self.tx.gas_limit -= intrinsic_cost;
            Ok(intrinsic_cost)
        } else {
            Err(InvalidTransaction::CallGasCostMoreThanGasLimit)
        }
    }

    /// Reference: https://github.com/ethereum/execution-specs/blob/c854868f4abf2ab0c3e8790d4c40607e0d251147/src/ethereum/cancun/fork.py#L332
    pub fn validate_transaction(&mut self) -> Result<(), InvalidTransaction> {
        let is_create = matches!(self.tx.transact_to, TransactTo::Create);

        if is_create && self.tx.data.len() > 2 * MAX_CODE_SIZE {
            return Err(InvalidTransaction::CreateInitCodeSizeLimit);
        }
        if let Some(max) = self.tx.max_fee_per_blob_gas {
            let price = self.block.blob_gasprice.unwrap();
            if U256::from(price) > max {
                return Err(InvalidTransaction::BlobGasPriceGreaterThanMax);
            }
            if self.tx.blob_hashes.is_empty() {
                return Err(InvalidTransaction::EmptyBlobs);
            }
            if is_create {
                return Err(InvalidTransaction::BlobCreateTransaction);
            }
            for blob in self.tx.blob_hashes.iter() {
                if blob[0] != VERSIONED_HASH_VERSION_KZG {
                    return Err(InvalidTransaction::BlobVersionNotSupported);
                }
            }

            let num_blobs = self.tx.blob_hashes.len();
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
        let data_cost = self.tx.data.iter().fold(0, |acc, byte| {
            acc + if *byte == 0 {
                TX_DATA_COST_PER_ZERO
            } else {
                TX_DATA_COST_PER_NON_ZERO
            }
        });
        let create_cost = match self.tx.transact_to {
            TransactTo::Call(_) => 0,
            TransactTo::Create => TX_CREATE_COST + init_code_cost(self.tx.data.len() as u64),
        };
        let access_list_cost = access_list_cost(&self.tx.access_list);
        TX_BASE_COST + data_cost + create_cost + access_list_cost
    }
}

#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct CfgEnv {
    // Chain ID of the EVM, it will be compared to the transaction's Chain ID.
    // Chain ID is introduced EIP-155
    pub chain_id: u64,
    // Bytecode that is created with CREATE/CREATE2 is by default analysed and jumptable is created.
    // This is very beneficial for testing and speeds up execution of that bytecode if called multiple times.
    //
    // Default: Analyse
    //pub perf_analyse_created_bytecodes: AnalysisKind,
    // If some it will effects EIP-170: Contract code size limit. Useful to increase this because of tests.
    // By default it is 0x6000 (~25kb).
    //pub limit_contract_code_size: Option<usize>,
}

#[derive(Clone, Debug, Default)]
pub struct BlockEnv {
    /// The number of ancestor blocks of this block (block height).
    pub number: U256,
    /// Coinbase or miner or address that created and signed the block.
    ///
    /// This is the receiver address of all the gas spent in the block.
    pub coinbase: Address,
    /// The timestamp of the block in seconds since the UNIX epoch.
    pub timestamp: U256,
    // The gas limit of the block.
    //pub gas_limit: U256,
    //
    // The base fee per gas, added in the London upgrade with [EIP-1559].
    //
    // [EIP-1559]: https://eips.ethereum.org/EIPS/eip-1559
    pub basefee: U256,
    // The difficulty of the block.
    //
    // Unused after the Paris (AKA the merge) upgrade, and replaced by `prevrandao`.
    //pub difficulty: U256,
    // The output of the randomness beacon provided by the beacon chain.
    //
    // Replaces `difficulty` after the Paris (AKA the merge) upgrade with [EIP-4399].
    //
    // NOTE: `prevrandao` can be found in a block in place of `mix_hash`.
    //
    // [EIP-4399]: https://eips.ethereum.org/EIPS/eip-4399
    pub prevrandao: Option<B256>,
    // Excess blob gas and blob gasprice.
    // See also [`crate::calc_excess_blob_gas`]
    // and [`calc_blob_gasprice`].
    //
    // Incorporated as part of the Cancun upgrade via [EIP-4844].
    //
    // [EIP-4844]: https://eips.ethereum.org/EIPS/eip-4844
    pub excess_blob_gas: Option<u64>,
    pub blob_gasprice: Option<u128>,
}

impl BlockEnv {
    pub fn set_blob_base_fee(&mut self, excess_blob_gas: u64) {
        self.excess_blob_gas = Some(excess_blob_gas);
        self.blob_gasprice = Some(calc_blob_gasprice(excess_blob_gas));
    }
}

/// The transaction environment.
#[derive(Clone, Debug)]
pub struct TxEnv {
    /// Caller aka Author aka transaction signer.
    pub caller: Address,
    /// The gas limit of the transaction.
    pub gas_limit: u64,
    /// The gas price of the transaction.
    pub gas_price: U256,
    /// The destination of the transaction.
    pub transact_to: TransactTo,
    /// The value sent to `transact_to`.
    pub value: U256,
    // The data of the transaction.
    pub data: Bytes,
    // The nonce of the transaction.
    //
    // Caution: If set to `None`, then nonce validation against the account's nonce is skipped: [InvalidTransaction::NonceTooHigh] and [InvalidTransaction::NonceTooLow]
    // pub nonce: Option<u64>,

    // The chain ID of the transaction. If set to `None`, no checks are performed.
    //
    // Incorporated as part of the Spurious Dragon upgrade via [EIP-155].
    //
    // [EIP-155]: https://eips.ethereum.org/EIPS/eip-155
    // pub chain_id: Option<u64>,

    // A list of addresses and storage keys that the transaction plans to access.
    //
    // Added in [EIP-2930].
    //
    // [EIP-2930]: https://eips.ethereum.org/EIPS/eip-2930
    pub access_list: AccessList,

    // The priority fee per gas.
    //
    // Incorporated as part of the London upgrade via [EIP-1559].
    //
    // [EIP-1559]: https://eips.ethereum.org/EIPS/eip-1559
    // pub gas_priority_fee: Option<U256>,

    // The list of blob versioned hashes. Per EIP there should be at least
    // one blob present if [`Self::max_fee_per_blob_gas`] is `Some`.
    //
    // Incorporated as part of the Cancun upgrade via [EIP-4844].
    //
    // [EIP-4844]: https://eips.ethereum.org/EIPS/eip-4844
    pub blob_hashes: Vec<B256>,
    // The max fee per blob gas.
    //
    // Incorporated as part of the Cancun upgrade via [EIP-4844].
    //
    // [EIP-4844]: https://eips.ethereum.org/EIPS/eip-4844
    pub max_fee_per_blob_gas: Option<U256>,
}

impl Default for TxEnv {
    fn default() -> Self {
        Self {
            caller: Address::zero(),
            // TODO: we are using signed comparison for the gas counter
            gas_limit: i64::MAX as _,
            gas_price: U256::zero(),
            // gas_priority_fee: None,
            transact_to: TransactTo::Call(Address::zero()),
            value: U256::zero(),
            data: Bytes::new(),
            // chain_id: None,
            // nonce: None,
            access_list: Default::default(),
            blob_hashes: Vec::new(),
            max_fee_per_blob_gas: None,
        }
    }
}

/// Transaction destination.
#[derive(Clone, Debug)]
pub enum TransactTo {
    /// Simple call to an address.
    Call(Address),
    /// Contract creation.
    Create,
}

impl TxEnv {
    pub fn get_address(&self) -> Address {
        match self.transact_to {
            TransactTo::Call(addr) => addr,
            TransactTo::Create => self.caller,
        }
    }
}
