pub const SUCCESS_FOR_CALL: i32 = 1;
pub const REVERT_FOR_CALL: i32 = 0;
pub const HALT_FOR_CALL: i32 = 2;
pub const SUCCESS_FOR_RETURN: i32 = 1;
pub const REVERT_FOR_CREATE: i32 = 0;
pub const MAX_CODE_SIZE: usize = 0x6000;
pub const TX_BASE_COST: u64 = 21_000;
pub const WORD_SIZE: usize = 32;

/// Contains the gas costs of the EVM instructions
pub mod gas_cost {
    pub const ADD: u64 = 3;
    pub const MUL: u64 = 5;
    pub const SUB: u64 = 3;
    pub const DIV: u64 = 5;
    pub const SDIV: u64 = 5;
    pub const MOD: u64 = 5;
    pub const SMOD: u64 = 5;
    pub const ADDMOD: u64 = 8;
    pub const MULMOD: u64 = 8;
    pub const EXP_STATIC: u64 = 10;
    pub const EXP_DYNAMIC_BASE: u64 = 50;
    pub const SIGNEXTEND: u64 = 5;
    pub const LT: u64 = 3;
    pub const GT: u64 = 3;
    pub const SLT: u64 = 3;
    pub const SGT: u64 = 3;
    pub const EQ: u64 = 3;
    pub const ISZERO: u64 = 3;
    pub const AND: u64 = 3;
    pub const OR: u64 = 3;
    pub const XOR: u64 = 3;
    pub const NOT: u64 = 3;
    pub const BYTE: u64 = 3;
    pub const SHL: u64 = 3;
    pub const SHR: u64 = 3;
    pub const SAR: u64 = 3;
    pub const KECCAK25_STATIC: u64 = 30;
    pub const KECCAK25_DYNAMIC_BASE: u64 = 6;
    pub const CALLDATALOAD: u64 = 3;
    pub const CALLDATASIZE: u64 = 2;
    pub const CALLDATACOPY_STATIC: u64 = 3;
    pub const CALLDATACOPY_DYNAMIC_BASE: u64 = 3;
    pub const RETURNDATASIZE: u64 = 2;
    pub const RETURNDATACOPY_STATIC: u64 = 3;
    pub const RETURNDATACOPY_DYNAMIC_BASE: u64 = 3;
    pub const ADDRESS: u64 = 2;
    pub const BALANCE: u64 = 100;
    pub const ORIGIN: u64 = 2;
    pub const CALLER: u64 = 2;
    pub const BLOCKHASH: u64 = 20;
    pub const COINBASE: u64 = 2;
    pub const TIMESTAMP: u64 = 2;
    pub const NUMBER: u64 = 2;
    pub const PREVRANDAO: u64 = 2;
    pub const GASLIMIT: u64 = 2;
    pub const CHAINID: u64 = 2;
    pub const SELFBALANCE: u64 = 5;
    pub const BASEFEE: u64 = 2;
    pub const BLOBHASH: u64 = 3;
    pub const BLOBBASEFEE: u64 = 2;
    pub const POP: u64 = 2;
    pub const MLOAD_STATIC: u64 = 3;
    pub const MSTORE_STATIC: u64 = 3;
    pub const MSTORE8_STATIC: u64 = 3;
    pub const JUMP: u64 = 8;
    pub const JUMPI: u64 = 10;
    pub const PC: u64 = 2;
    pub const MSIZE: u64 = 2;
    pub const GAS: u64 = 2;
    pub const JUMPDEST: u64 = 1;
    pub const TLOAD: u64 = 100;
    pub const TSTORE: u64 = 100;
    pub const MCOPY_STATIC: u64 = 3;
    pub const MCOPY_DYNAMIC_BASE: u64 = 3;
    pub const PUSH0: u64 = 2;
    pub const PUSHN: u64 = 3;
    pub const DUPN: u64 = 3;
    pub const SWAPN: u64 = 3;
    pub const LOGN_STATIC: u64 = 375;
    pub const LOGN_DYNAMIC_BASE: u64 = 375;
    pub const LOGN_DYNAMIC_BYTE_BASE: u64 = 8;
    pub const CALLVALUE: u64 = 2;
    pub const CODESIZE: u64 = 2;
    pub const CODECOPY_STATIC: u64 = 3;
    pub const CODECOPY_DYNAMIC_BASE: u64 = 3;
    pub const GASPRICE: u64 = 2;
    pub const EXTCODECOPY_DYNAMIC_BASE: u64 = 3;
}

pub mod call_opcode {
    pub const WARM_ADDRESS_ACCESS_COST: u64 = 100;
    pub const COLD_ADDRESS_ACCESS_COST: u64 = 2_600;
    pub const NON_ZERO_VALUE_COST: u64 = 9_000;
    pub const BASIC_FALLBACK_FUNCTION_STIPEND: u64 = 2_300;
    pub const VALUE_TO_EMPTY_ACCOUNT_COST: u64 = 25_000;
}
pub const STACK_LIMIT: usize = 1024;

pub const GAS_REFUND_DENOMINATOR: u64 = 5;

pub const EMPTY_CODE_HASH_STR: &str =
    "0xc5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470";

pub const MEMORY_EXPANSION_QUOTIENT: usize = 512;
