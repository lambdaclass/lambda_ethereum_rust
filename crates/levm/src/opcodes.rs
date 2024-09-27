#[derive(Debug, PartialEq, Eq, Clone, PartialOrd)]
pub enum Opcode {
    // Stop and Arithmetic Operations
    STOP = 0x00,
    ADD = 0x01,
    MUL = 0x02,
    SUB = 0x03,
    DIV = 0x04,
    SDIV = 0x05,
    MOD = 0x06,
    SMOD = 0x07,
    ADDMOD = 0x08,
    MULMOD = 0x09,
    EXP = 0x0A,
    SIGNEXTEND = 0x0B,

    // Comparison & Bitwise Logic Operations
    LT = 0x10,
    GT = 0x11,
    SLT = 0x12,
    SGT = 0x13,
    EQ = 0x14,
    ISZERO = 0x15,
    AND = 0x16,
    OR = 0x17,
    XOR = 0x18,
    NOT = 0x19,
    BYTE = 0x1A,
    SHL = 0x1B,
    SHR = 0x1C,
    SAR = 0x1D,

    // KECCAK256
    KECCAK256 = 0x20,

    // // Environmental Information
    // ADDRESS = 0x30,
    // BALANCE = 0x31,
    // ORIGIN = 0x32,
    // CALLER = 0x33,
    // CALLVALUE = 0x34,
    // CALLDATALOAD = 0x35,
    // CALLDATASIZE = 0x36,
    // CALLDATACOPY = 0x37,
    // CODESIZE = 0x38,
    // CODECOPY = 0x39,
    // GASPRICE = 0x3A,
    // EXTCODESIZE = 0x3B,
    // EXTCODECOPY = 0x3C,
    // RETURNDATASIZE = 0x3D,
    // RETURNDATACOPY = 0x3E,
    // EXTCODEHASH = 0x3F,

    // // Block Information
    // BLOCKHASH = 0x40,
    // COINBASE = 0x41,
    // TIMESTAMP = 0x42,
    // NUMBER = 0x43,
    // PREVRANDAO = 0x44,
    // GASLIMIT = 0x45,
    // CHAINID = 0x46,
    // SELFBALANCE = 0x47,
    // BASEFEE = 0x48,
    // BLOBBASEFEE = 0x4A

    // // Stack, Memory, Storage, and Flow Operations
    POP = 0x50,
    MLOAD = 0x51,
    MSTORE = 0x52,
    MSTORE8 = 0x53,
    // SLOAD = 0x54,
    // SSTORE = 0x55,
    // JUMP = 0x56,
    // JUMPI = 0x57,
    // PC = 0x58,
    MSIZE = 0x59,
    // GAS = 0x5A,
    // JUMPDEST = 0x5B,
    // TLOAD = 0x5C,
    // TSTORE = 0x5D,
    MCOPY = 0x5E,

    // // Push Operations
    PUSH0 = 0x5F,
    PUSH1 = 0x60,
    PUSH2 = 0x61,
    PUSH3 = 0x62,
    PUSH4 = 0x63,
    PUSH5 = 0x64,
    PUSH6 = 0x65,
    PUSH7 = 0x66,
    PUSH8 = 0x67,
    PUSH9 = 0x68,
    PUSH10 = 0x69,
    PUSH11 = 0x6A,
    PUSH12 = 0x6B,
    PUSH13 = 0x6C,
    PUSH14 = 0x6D,
    PUSH15 = 0x6E,
    PUSH16 = 0x6F,
    PUSH17 = 0x70,
    PUSH18 = 0x71,
    PUSH19 = 0x72,
    PUSH20 = 0x73,
    PUSH21 = 0x74,
    PUSH22 = 0x75,
    PUSH23 = 0x76,
    PUSH24 = 0x77,
    PUSH25 = 0x78,
    PUSH26 = 0x79,
    PUSH27 = 0x7A,
    PUSH28 = 0x7B,
    PUSH29 = 0x7C,
    PUSH30 = 0x7D,
    PUSH31 = 0x7E,
    PUSH32 = 0x7F,
    // // Duplication Operations
    DUP1 = 0x80,
    DUP2 = 0x81,
    DUP3 = 0x82,
    DUP4 = 0x83,
    DUP5 = 0x84,
    DUP6 = 0x85,
    DUP7 = 0x86,
    DUP8 = 0x87,
    DUP9 = 0x88,
    DUP10 = 0x89,
    DUP11 = 0x8A,
    DUP12 = 0x8B,
    DUP13 = 0x8C,
    DUP14 = 0x8D,
    DUP15 = 0x8E,
    DUP16 = 0x8F,
    // // Swap Operations
    SWAP1 = 0x90,
    SWAP2 = 0x91,
    SWAP3 = 0x92,
    SWAP4 = 0x93,
    SWAP5 = 0x94,
    SWAP6 = 0x95,
    SWAP7 = 0x96,
    SWAP8 = 0x97,
    SWAP9 = 0x98,
    SWAP10 = 0x99,
    SWAP11 = 0x9A,
    SWAP12 = 0x9B,
    SWAP13 = 0x9C,
    SWAP14 = 0x9D,
    SWAP15 = 0x9E,
    SWAP16 = 0x9F,
    // // Logging Operations
    // LOG0 = 0xA0,
    // LOG1 = 0xA1,
    // LOG2 = 0xA2,
    // LOG3 = 0xA3,
    // LOG4 = 0xA4,

    // // System Operations
    // CREATE = 0xF0,
    CALL = 0xF1,
    // CALLCODE = 0xF2,
    RETURN = 0xF3,
    // DELEGATECALL = 0xF4,
    // CREATE2 = 0xF5,
    // STATICCALL = 0xFA,
    // REVERT = 0xFD,
    // INVALID = 0xFE,
    // SELFDESTRUCT = 0xFF,
}

impl Copy for Opcode {}

impl From<u8> for Opcode {
    fn from(byte: u8) -> Self {
        match byte {
            0x00 => Opcode::STOP,
            0x01 => Opcode::ADD,
            0x16 => Opcode::AND,
            0x17 => Opcode::OR,
            0x18 => Opcode::XOR,
            0x19 => Opcode::NOT,
            0x1A => Opcode::BYTE,
            0x1B => Opcode::SHL,
            0x1C => Opcode::SHR,
            0x1D => Opcode::SAR,
            0x02 => Opcode::MUL,
            0x03 => Opcode::SUB,
            0x04 => Opcode::DIV,
            0x05 => Opcode::SDIV,
            0x06 => Opcode::MOD,
            0x07 => Opcode::SMOD,
            0x08 => Opcode::ADDMOD,
            0x09 => Opcode::MULMOD,
            0x0A => Opcode::EXP,
            0x0B => Opcode::SIGNEXTEND,
            0x10 => Opcode::LT,
            0x11 => Opcode::GT,
            0x12 => Opcode::SLT,
            0x13 => Opcode::SGT,
            0x14 => Opcode::EQ,
            0x15 => Opcode::ISZERO,
            0x20 => Opcode::KECCAK256,
            x if x == Opcode::PUSH0 as u8 => Opcode::PUSH0,
            x if x == Opcode::PUSH1 as u8 => Opcode::PUSH1,
            x if x == Opcode::PUSH2 as u8 => Opcode::PUSH2,
            x if x == Opcode::PUSH3 as u8 => Opcode::PUSH3,
            x if x == Opcode::PUSH4 as u8 => Opcode::PUSH4,
            x if x == Opcode::PUSH5 as u8 => Opcode::PUSH5,
            x if x == Opcode::PUSH6 as u8 => Opcode::PUSH6,
            x if x == Opcode::PUSH7 as u8 => Opcode::PUSH7,
            x if x == Opcode::PUSH8 as u8 => Opcode::PUSH8,
            x if x == Opcode::PUSH9 as u8 => Opcode::PUSH9,
            x if x == Opcode::PUSH10 as u8 => Opcode::PUSH10,
            x if x == Opcode::PUSH11 as u8 => Opcode::PUSH11,
            x if x == Opcode::PUSH12 as u8 => Opcode::PUSH12,
            x if x == Opcode::PUSH13 as u8 => Opcode::PUSH13,
            x if x == Opcode::PUSH14 as u8 => Opcode::PUSH14,
            x if x == Opcode::PUSH15 as u8 => Opcode::PUSH15,
            x if x == Opcode::PUSH16 as u8 => Opcode::PUSH16,
            x if x == Opcode::PUSH17 as u8 => Opcode::PUSH17,
            x if x == Opcode::PUSH18 as u8 => Opcode::PUSH18,
            x if x == Opcode::PUSH19 as u8 => Opcode::PUSH19,
            x if x == Opcode::PUSH20 as u8 => Opcode::PUSH20,
            x if x == Opcode::PUSH21 as u8 => Opcode::PUSH21,
            x if x == Opcode::PUSH22 as u8 => Opcode::PUSH22,
            x if x == Opcode::PUSH23 as u8 => Opcode::PUSH23,
            x if x == Opcode::PUSH24 as u8 => Opcode::PUSH24,
            x if x == Opcode::PUSH25 as u8 => Opcode::PUSH25,
            x if x == Opcode::PUSH26 as u8 => Opcode::PUSH26,
            x if x == Opcode::PUSH27 as u8 => Opcode::PUSH27,
            x if x == Opcode::PUSH28 as u8 => Opcode::PUSH28,
            x if x == Opcode::PUSH29 as u8 => Opcode::PUSH29,
            x if x == Opcode::PUSH30 as u8 => Opcode::PUSH30,
            x if x == Opcode::PUSH31 as u8 => Opcode::PUSH31,
            x if x == Opcode::DUP1 as u8 => Opcode::DUP1,
            x if x == Opcode::DUP2 as u8 => Opcode::DUP2,
            x if x == Opcode::DUP3 as u8 => Opcode::DUP3,
            x if x == Opcode::DUP4 as u8 => Opcode::DUP4,
            x if x == Opcode::DUP5 as u8 => Opcode::DUP5,
            x if x == Opcode::DUP6 as u8 => Opcode::DUP6,
            x if x == Opcode::DUP7 as u8 => Opcode::DUP7,
            x if x == Opcode::DUP8 as u8 => Opcode::DUP8,
            x if x == Opcode::DUP9 as u8 => Opcode::DUP9,
            x if x == Opcode::DUP10 as u8 => Opcode::DUP10,
            x if x == Opcode::DUP11 as u8 => Opcode::DUP11,
            x if x == Opcode::DUP12 as u8 => Opcode::DUP12,
            x if x == Opcode::DUP13 as u8 => Opcode::DUP13,
            x if x == Opcode::DUP14 as u8 => Opcode::DUP14,
            x if x == Opcode::DUP15 as u8 => Opcode::DUP15,
            x if x == Opcode::DUP16 as u8 => Opcode::DUP16,
            x if x == Opcode::SWAP1 as u8 => Opcode::SWAP1,
            x if x == Opcode::SWAP2 as u8 => Opcode::SWAP2,
            x if x == Opcode::SWAP3 as u8 => Opcode::SWAP3,
            x if x == Opcode::SWAP4 as u8 => Opcode::SWAP4,
            x if x == Opcode::SWAP5 as u8 => Opcode::SWAP5,
            x if x == Opcode::SWAP6 as u8 => Opcode::SWAP6,
            x if x == Opcode::SWAP7 as u8 => Opcode::SWAP7,
            x if x == Opcode::SWAP8 as u8 => Opcode::SWAP8,
            x if x == Opcode::SWAP9 as u8 => Opcode::SWAP9,
            x if x == Opcode::SWAP10 as u8 => Opcode::SWAP10,
            x if x == Opcode::SWAP11 as u8 => Opcode::SWAP11,
            x if x == Opcode::SWAP12 as u8 => Opcode::SWAP12,
            x if x == Opcode::SWAP13 as u8 => Opcode::SWAP13,
            x if x == Opcode::SWAP14 as u8 => Opcode::SWAP14,
            x if x == Opcode::SWAP15 as u8 => Opcode::SWAP15,
            x if x == Opcode::SWAP16 as u8 => Opcode::SWAP16,
            0x7F => Opcode::PUSH32,
            0x50 => Opcode::POP,
            0x51 => Opcode::MLOAD,
            0x52 => Opcode::MSTORE,
            0x53 => Opcode::MSTORE8,
            0x59 => Opcode::MSIZE,
            0x5E => Opcode::MCOPY,
            0xF1 => Opcode::CALL,
            0xF3 => Opcode::RETURN,
            _ => panic!("Unknown opcode: 0x{:02X}", byte),
        }
    }
}
