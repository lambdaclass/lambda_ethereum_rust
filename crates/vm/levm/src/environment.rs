use crate::constants::TX_BASE_COST;
use ethereum_rust_core::{Address, H256, U256};

#[derive(Debug, Default, Clone)]
pub struct Environment {
    /// The sender address of the transaction that originated
    /// this execution.
    pub origin: Address,
    pub consumed_gas: U256,
    pub refunded_gas: U256,
    pub gas_limit: U256,
    pub block_number: U256,
    pub coinbase: Address,
    pub timestamp: U256,
    pub prev_randao: Option<H256>,
    pub chain_id: U256,
    pub base_fee_per_gas: U256,
    pub gas_price: U256,
    pub block_excess_blob_gas: Option<U256>,
    pub block_blob_gas_used: Option<U256>,
    pub tx_blob_hashes: Option<Vec<H256>>,
}

// The fee on a 1559 transaction is base_fee + priority_fee
// So because of how things work here, we got priority_fee = gas_price - base_fee_per_gas
//
// Things to do:
// - Fix fee calculations. Use EIP 1559 (base_fee + priority fee etc).
// - Send the coinbase fee to the coinbase_account.
// - Do the full gas discount at the beginning and then refund at the end.
// - Add a method for substracting/adding to the balance of an account. This is done all over the place

/*
    gas_price = effective_gas_price
    base_fee_per_gas = base_fee
    priority_fee_per_gas = gas_price - base_fee_per_gas

    effective_gas_price = priority_fee_per_gas + base_fee_per_gas

    The priority fee per gas field is NOT a part of our VM. It is implicit as the difference
    between the gas_price and the base_fee_per_gas.

    When setting the VM for execution at the beginning, we have to calculate the priority_fee_per_gas as
    priority_fee_per_gas = min(
        tx.max_priority_fee_per_gas,
        tx.max_fee_per_gas - base_fee_per_gas,
    )
    to then set the gas_price as priority_fee_per_gas + base_fee_per_gas


*/

impl Environment {
    pub fn default_from_address(origin: Address) -> Self {
        Self {
            origin,
            consumed_gas: TX_BASE_COST,
            refunded_gas: U256::zero(),
            gas_limit: U256::MAX,
            block_number: Default::default(),
            coinbase: Default::default(),
            timestamp: Default::default(),
            prev_randao: Default::default(),
            chain_id: U256::one(),
            base_fee_per_gas: Default::default(),
            gas_price: Default::default(),
            block_excess_blob_gas: Default::default(),
            block_blob_gas_used: Default::default(),
            tx_blob_hashes: Default::default(),
        }
    }
}
