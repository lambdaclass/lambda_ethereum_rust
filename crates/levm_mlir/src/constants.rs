use thiserror::Error;

pub const MAX_STACK_SIZE: usize = 1024;
pub const GAS_COUNTER_GLOBAL: &str = "levm_mlir__gas_counter";
pub const STACK_BASEPTR_GLOBAL: &str = "levm_mlir__stack_baseptr";
pub const CODE_PTR_GLOBAL: &str = "levm_mlir__code_ptr";
pub const STACK_PTR_GLOBAL: &str = "levm_mlir__stack_ptr";
pub const MEMORY_PTR_GLOBAL: &str = "levm_mlir__memory_ptr";
pub const MEMORY_SIZE_GLOBAL: &str = "levm_mlir__memory_size";
pub const CALLDATA_PTR_GLOBAL: &str = "levm_mlir__calldata_ptr";
pub const CALLDATA_SIZE_GLOBAL: &str = "levm_mlir__calldata_size";
pub const MAIN_ENTRYPOINT: &str = "main";

// An empty bytecode has the following Keccak256 hash
pub const EMPTY_CODE_HASH_STR: &str =
    "0xc5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470";

pub const VERSIONED_HASH_VERSION_KZG: u8 = 0x01;
pub const MAX_BLOB_NUMBER_PER_BLOCK: u8 = 0x01;

//TODO: Add missing opcodes gas consumption costs
//  -> This implies refactoring codegen/operations.rs
/// Contains the gas costs of the EVM instructions
pub mod gas_cost {
    pub const ADD: i64 = 3;
    pub const MUL: i64 = 5;
    pub const SUB: i64 = 3;
    pub const DIV: i64 = 5;
    pub const SDIV: i64 = 5;
    pub const MOD: i64 = 5;
    pub const SMOD: i64 = 5;
    pub const ADDMOD: i64 = 8;
    pub const MULMOD: i64 = 8;
    pub const EXP: i64 = 10;
    pub const SIGNEXTEND: i64 = 5;
    pub const LT: i64 = 3;
    pub const GT: i64 = 3;
    pub const SLT: i64 = 3;
    pub const SGT: i64 = 3;
    pub const EQ: i64 = 3;
    pub const ISZERO: i64 = 3;
    pub const AND: i64 = 3;
    pub const OR: i64 = 3;
    pub const XOR: i64 = 3;
    pub const NOT: i64 = 3;
    pub const BYTE: i64 = 3;
    pub const SHL: i64 = 3;
    pub const SAR: i64 = 3;
    pub const BALANCE_WARM: i64 = 100;
    pub const BALANCE_COLD: i64 = 2600;
    pub const ORIGIN: i64 = 2;
    pub const CALLER: i64 = 2;
    pub const CALLVALUE: i64 = 2;
    pub const CALLDATALOAD: i64 = 3;
    pub const CALLDATASIZE: i64 = 2;
    pub const CALLDATACOPY: i64 = 3;
    pub const CODESIZE: i64 = 2;
    pub const COINBASE: i64 = 2;
    pub const GASPRICE: i64 = 2;
    pub const SELFBALANCE: i64 = 5;
    pub const NUMBER: i64 = 2;
    pub const PREVRANDAO: i64 = 2;
    pub const BLOBBASEFEE: i64 = 2;
    pub const CHAINID: i64 = 2;
    pub const BASEFEE: i64 = 2;
    pub const BLOBHASH: i64 = 3;
    pub const POP: i64 = 2;
    pub const MLOAD: i64 = 3;
    pub const MSTORE: i64 = 3;
    pub const MSTORE8: i64 = 3;
    pub const SLOAD_WARM: i64 = 100;
    pub const SLOAD_COLD: i64 = 2100;
    pub const JUMP: i64 = 8;
    pub const JUMPI: i64 = 10;
    pub const PC: i64 = 2;
    pub const MSIZE: i64 = 2;
    pub const GAS: i64 = 2;
    pub const JUMPDEST: i64 = 1;
    pub const MCOPY: i64 = 3;
    pub const PUSH0: i64 = 2;
    pub const PUSHN: i64 = 3;
    pub const DUPN: i64 = 3;
    pub const SWAPN: i64 = 3;
    pub const TIMESTAMP: i64 = 2;
    pub const KECCAK256: i64 = 30;
    pub const CODECOPY: i64 = 3;
    pub const LOG: i64 = 375;
    pub const BLOCKHASH: i64 = 20;
    pub const CALL_WARM: i64 = 100;
    pub const CALL_COLD: i64 = 2600;
    pub const EXTCODEHASH_WARM: i64 = 100;
    pub const EXTCODEHASH_COLD: i64 = 2600;
    pub const EXTCODESIZE_WARM: i64 = 100;
    pub const EXTCODESIZE_COLD: i64 = 2600;
    pub const EXTCODECOPY_WARM: i64 = 100;
    pub const EXTCODECOPY_COLD: i64 = 2600;
    pub const RETURNDATASIZE: i64 = 2;
    pub const RETURNDATACOPY: i64 = 3;
    pub const ADDRESS: i64 = 2;
    pub const GASLIMIT: i64 = 2;
    pub const SSTORE_MIN_REMAINING_GAS: i64 = 2_300;
    pub const CREATE: i64 = 32_000;
    pub const TLOAD: i64 = 100;
    pub const TSTORE: i64 = 100;
    pub const SELFDESTRUCT: i64 = 5_000;
    pub const SELFDESTRUCT_DYNAMIC_GAS: i64 = 25_000;

    pub const MIN_BLOB_GASPRICE: u64 = 1;
    pub const BLOB_GASPRICE_UPDATE_FRACTION: u64 = 3338477;

    pub const BYTE_DEPOSIT_COST: i64 = 200;
    pub const INIT_WORD_COST: i64 = 2;
    pub const HASH_WORD_COST: i64 = 6;

    // Transaction costs
    pub const TX_BASE_COST: u64 = 21000;
    pub const TX_DATA_COST_PER_NON_ZERO: u64 = 16;
    pub const TX_DATA_COST_PER_ZERO: u64 = 4;
    pub const TX_CREATE_COST: u64 = 32000;
    pub const TX_ACCESS_LIST_ADDRESS_COST: u64 = 2400;
    pub const TX_ACCESS_LIST_STORAGE_KEY_COST: u64 = 1900;
    pub const MAX_CODE_SIZE: usize = 0x6000;

    /// calculates the init_code_cost of create transactions as specified by the eip 3860
    /// -> https://eips.ethereum.org/EIPS/eip-3860
    pub fn init_code_cost(init_code_length: u64) -> u64 {
        assert!(init_code_length <= ((MAX_CODE_SIZE * 2) as u64));
        let number_of_words = init_code_length.saturating_add(31) / 32;
        INIT_WORD_COST as u64 * number_of_words
    }

    pub fn memory_expansion_cost(last_size: u32, new_size: u32) -> i64 {
        let new_memory_size_word = (new_size + 31) / 32;
        let new_memory_cost =
            (new_memory_size_word * new_memory_size_word) / 512 + (3 * new_memory_size_word);
        let last_memory_size_word = (last_size + 31) / 32;
        let last_memory_cost =
            (last_memory_size_word * last_memory_size_word) / 512 + (3 * last_memory_size_word);
        (new_memory_cost - last_memory_cost).into()
    }

    pub fn memory_copy_cost(size: u32) -> i64 {
        let memory_word_size = (size + 31) / 32;

        (memory_word_size * 3).into()
    }
    pub fn log_dynamic_gas_cost(size: u32, topic_count: u32) -> i64 {
        (super::gas_cost::LOG * topic_count as i64) + (8 * size as i64)
    }

    fn exponent_byte_size(exponent: u64) -> i64 {
        (((64 - exponent.leading_zeros()) + 7) / 8).into()
    }

    pub fn exp_dynamic_cost(exponent: u64) -> i64 {
        10 + 50 * exponent_byte_size(exponent)
    }
}

pub mod call_opcode {
    // Gas related constants
    pub const WARM_MEMORY_ACCESS_COST: u64 = 100;
    pub const NOT_ZERO_VALUE_COST: u64 = 9000;
    pub const EMPTY_CALLEE_COST: u64 = 25000;
    pub const STIPEND_GAS_ADDITION: u64 = 2300;
    pub const GAS_CAP_DIVISION_FACTOR: u64 = 64;
}

pub mod return_codes {
    pub const REVERT_RETURN_CODE: u8 = 0;
    pub const SUCCESS_RETURN_CODE: u8 = 1;
    pub const HALT_RETURN_CODE: u8 = 2;
}

pub mod precompiles {
    pub fn identity_dynamic_cost(len: u64) -> u64 {
        (len + 31) / 32 * 3
    }
    pub fn sha2_256_dynamic_cost(len: u64) -> u64 {
        (len + 31) / 32 * 12
    }
    pub fn ripemd_160_dynamic_cost(len: u64) -> u64 {
        (len + 31) / 32 * 120
    }
    pub fn ecpairing_dynamic_cost(len: u64) -> u64 {
        ECPAIRING_PAIRING_COST * (len / 192)
    }
    pub fn blake2_gas_cost(rounds: u32) -> u64 {
        rounds as u64
    }

    pub const fn ecpairing_g2_point1_start(pos: usize) -> usize {
        pos + ECP_FIELD_SIZE
    }

    pub const fn ecpairing_g2_point1_end(pos: usize) -> usize {
        pos + double_field_size()
    }

    pub const fn double_field_size() -> usize {
        ECP_FIELD_SIZE * 2
    }

    // ecRecover
    /// (0; 32) => Keccack-256 hash of the transaction.
    pub const ECR_HASH_END: usize = 32;
    /// The position of V in the signature.
    pub const ECR_V_POS: usize = 63;
    /// v ∈ {27, 28} => Recovery identifier, expected to be either 27 or 28.
    pub const ECR_V_BASE: i32 = 27;
    /// (64; 128) => signature, containing r and s.
    pub const ECR_SIG_END: usize = 128;
    pub const ECR_PARAMS_OFFSET: usize = 128;
    /// The padding len is 12, as the return value is a publicAddress => the recovered 20-byte address right aligned to 32 bytes.
    pub const ECR_PADDING_LEN: usize = 12;
    pub const ECRECOVER_COST: u64 = 3000;
    pub const ECRECOVER_ADDRESS: u64 = 0x01;

    // sha256
    pub const SHA2_256_STATIC_COST: u64 = 60;
    pub const SHA2_256_ADDRESS: u64 = 0x02;

    // ripemd160
    pub const RIPEMD_OUTPUT_LEN: usize = 32;
    /// Used to aligned to 32 bytes a 20-byte hash.
    pub const RIPEMD_PADDING_LEN: usize = 12;
    pub const RIPEMD_160_COST: u64 = 600;
    pub const RIPEMD_160_ADDRESS: u64 = 0x03;

    // identity
    pub const IDENTITY_STATIC_COST: u64 = 15;
    pub const IDENTITY_ADDRESS: u64 = 0x04;

    // modexp
    /// (0; 32) contains byte size of B.
    pub const BSIZE_END: usize = 32;
    /// (32; 64) contains byte size of E.
    pub const ESIZE_END: usize = 64;
    /// (64; 96) contains byte size of M.
    pub const MSIZE_END: usize = 96;
    /// Used to get values of B, E and M.
    pub const MXP_PARAMS_OFFSET: usize = 96;
    pub const MODEXP_ADDRESS: u64 = 0x05;
    pub const MIN_MODEXP_COST: u64 = 200;

    // ecadd
    pub const ECADD_PARAMS_OFFSET: usize = 128;
    /// (0; 32) contains x1.
    pub const ECADD_X1_END: usize = 32;
    /// (32; 64) contains y1.
    pub const ECADD_Y1_END: usize = 64;
    /// (64; 96) contains x2.
    pub const ECADD_X2_END: usize = 96;
    /// (96; 128) contains y2.
    pub const ECADD_Y2_END: usize = 128;
    pub const ECADD_ADDRESS: u64 = 0x06;
    pub const ECADD_COST: u64 = 150;

    // ecmul
    pub const ECMUL_PARAMS_OFFSET: usize = 96;
    /// (0; 32) contains x1.
    pub const ECMUL_X1_END: usize = 32;
    /// (32; 64) contains y1.
    pub const ECMUL_Y1_END: usize = 64;
    /// (64; 96) contains s => Scalar to use for the multiplication.
    pub const ECMUL_S_END: usize = 96;
    pub const ECMUL_ADDRESS: u64 = 0x07;
    pub const ECMUL_COST: u64 = 6000;

    // ecpairing
    /// Ecpairing loops over the calldata in chunks of 192 bytes.
    pub const ECP_INPUT_SIZE: usize = 192;
    /// Each field is of size 32 bytes.
    pub const ECP_FIELD_SIZE: usize = 32;
    /// The position of point G1.
    pub const G1_POINT_POS: usize = 64;
    /// The position of point G2.
    pub const G2_POINT_POS: usize = 128;
    pub const ECPAIRING_ADDRESS: u64 = 0x08;
    pub const ECPAIRING_STATIC_COST: u64 = 45000;
    pub const ECPAIRING_PAIRING_COST: u64 = 34000;

    // blake2f
    /// (0; 4) contains the rounds.
    pub const BF2_ROUND_END: usize = 4;
    /// (212; 213) postion of the block flag.
    pub const BF2_BLOCK_FLAG: usize = 212;
    /// Each element of the vectors is of size 8 bytes.
    pub const BF2_VEC_ELEM_SIZE: usize = 8;
    /// (4; 68) contains the State vector, which contins 8 elements of size BF2_VEC_ELEM_SIZE.
    pub const BF2_STATEVEC_INIT: usize = 4;
    /// (68; 196) contains the Message block vector, which contains 16 BF2_VEC_ELEM_SIZE.
    pub const BF2_MSGVEC_INIT: usize = 68;
    /// (196; 212) contains the Offset counters vector, which contains 2 BF2_VEC_ELEM_SIZE.
    pub const BF2_OFFSET_COUNT_INIT: usize = 196;
    pub const BLAKE2F_ADDRESS: u64 = 0x09;
}

#[derive(PartialEq, Debug)]
pub enum CallType {
    Call,
    StaticCall,
    DelegateCall,
    CallCode,
}

#[derive(Error, Debug)]
#[error("Couldn't parse CallType from u8")]
pub struct CallTypeParseError;

impl TryFrom<u8> for CallType {
    type Error = CallTypeParseError;
    fn try_from(call_type: u8) -> Result<CallType, Self::Error> {
        match call_type {
            x if x == CallType::Call as u8 => Ok(CallType::Call),
            x if x == CallType::StaticCall as u8 => Ok(CallType::StaticCall),
            x if x == CallType::DelegateCall as u8 => Ok(CallType::DelegateCall),
            x if x == CallType::CallCode as u8 => Ok(CallType::CallCode),
            _ => Err(CallTypeParseError),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exp_dynamic_gas_cost() {
        assert_eq!(gas_cost::exp_dynamic_cost(255), 60);
        assert_eq!(gas_cost::exp_dynamic_cost(256), 110);
        assert_eq!(gas_cost::exp_dynamic_cost(65536), 160);
        assert_eq!(gas_cost::exp_dynamic_cost(16777216), 210);
        assert_eq!(gas_cost::exp_dynamic_cost(4294967296), 260);
    }
}
