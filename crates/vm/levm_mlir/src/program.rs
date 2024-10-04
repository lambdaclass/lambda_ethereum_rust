use num_bigint::BigUint;
use std::{cmp::min, fmt};
use thiserror::Error;

#[derive(Debug)]
pub enum Opcode {
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
    // unused 0x0C-0x0F
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
    // unused 0x1E-0x1F
    KECCAK256 = 0x20,
    // unused 0x21-0x2F
    ADDRESS = 0x30,
    BALANCE = 0x31,
    ORIGIN = 0x32,
    CALLER = 0x33,
    CALLVALUE = 0x34,
    CALLDATALOAD = 0x35,
    CALLDATASIZE = 0x36,
    CALLDATACOPY = 0x37,
    CODESIZE = 0x38,
    CODECOPY = 0x39,
    GASPRICE = 0x3A,
    EXTCODESIZE = 0x3B,
    EXTCODECOPY = 0x3C,
    RETURNDATASIZE = 0x3D,
    RETURNDATACOPY = 0x3E,
    EXTCODEHASH = 0x3F,
    BLOCKHASH = 0x40,
    COINBASE = 0x41,
    TIMESTAMP = 0x42,
    NUMBER = 0x43,
    PREVRANDAO = 0x44,
    GASLIMIT = 0x45,
    CHAINID = 0x46,
    SELFBALANCE = 0x47,
    BASEFEE = 0x48,
    BLOBHASH = 0x49,
    BLOBBASEFEE = 0x4A,
    // unused 0x4B-0x4F
    POP = 0x50,
    MLOAD = 0x51,
    MSTORE = 0x52,
    MSTORE8 = 0x53,
    SLOAD = 0x54,
    SSTORE = 0x55,
    JUMP = 0x56,
    JUMPI = 0x57,
    PC = 0x58,
    MSIZE = 0x59,
    GAS = 0x5A,
    JUMPDEST = 0x5B,
    TLOAD = 0x5C,
    TSTORE = 0x5D,
    MCOPY = 0x5E,
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
    LOG0 = 0xA0,
    LOG1 = 0xA1,
    LOG2 = 0xA2,
    LOG3 = 0xA3,
    LOG4 = 0xA4,
    // unused 0xA5-0xEF
    CREATE = 0xF0,
    CALL = 0xF1,
    CALLCODE = 0xF2,
    RETURN = 0xF3,
    DELEGATECALL = 0xF4,
    CREATE2 = 0xF5,
    // unused 0xF6-0xF9
    STATICCALL = 0xFA,
    // unused 0xFB-0xFC
    REVERT = 0xFD,
    INVALID = 0xFE,
    SELFDESTRUCT = 0xFF,
}

#[derive(Error, Debug)]
#[error("The opcode `{:02X}` is not valid", self.0)]
pub struct OpcodeParseError(u8);

#[derive(Error, Debug)]
pub struct ParseError(Vec<OpcodeParseError>);

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let opcodes: Vec<_> = self.0.iter().map(|x| format!("{:02X}", x.0)).collect();
        writeln!(f, "The following opcodes could not be parsed: ")?;
        writeln!(f, "{:#?}", opcodes)?;
        Ok(())
    }
}

impl TryFrom<u8> for Opcode {
    type Error = OpcodeParseError;
    fn try_from(opcode: u8) -> Result<Opcode, Self::Error> {
        let op = match opcode {
            x if x == Opcode::STOP as u8 => Opcode::STOP,
            x if x == Opcode::ADD as u8 => Opcode::ADD,
            x if x == Opcode::MUL as u8 => Opcode::MUL,
            x if x == Opcode::SUB as u8 => Opcode::SUB,
            x if x == Opcode::DIV as u8 => Opcode::DIV,
            x if x == Opcode::SDIV as u8 => Opcode::SDIV,
            x if x == Opcode::MOD as u8 => Opcode::MOD,
            x if x == Opcode::SMOD as u8 => Opcode::SMOD,
            x if x == Opcode::ADDMOD as u8 => Opcode::ADDMOD,
            x if x == Opcode::MULMOD as u8 => Opcode::MULMOD,
            x if x == Opcode::EXP as u8 => Opcode::EXP,
            x if x == Opcode::SIGNEXTEND as u8 => Opcode::SIGNEXTEND,
            x if x == Opcode::LT as u8 => Opcode::LT,
            x if x == Opcode::GT as u8 => Opcode::GT,
            x if x == Opcode::SLT as u8 => Opcode::SLT,
            x if x == Opcode::SGT as u8 => Opcode::SGT,
            x if x == Opcode::EQ as u8 => Opcode::EQ,
            x if x == Opcode::ISZERO as u8 => Opcode::ISZERO,
            x if x == Opcode::AND as u8 => Opcode::AND,
            x if x == Opcode::OR as u8 => Opcode::OR,
            x if x == Opcode::XOR as u8 => Opcode::XOR,
            x if x == Opcode::NOT as u8 => Opcode::NOT,
            x if x == Opcode::BYTE as u8 => Opcode::BYTE,
            x if x == Opcode::SHL as u8 => Opcode::SHL,
            x if x == Opcode::SHR as u8 => Opcode::SHR,
            x if x == Opcode::SAR as u8 => Opcode::SAR,
            x if x == Opcode::KECCAK256 as u8 => Opcode::KECCAK256,
            x if x == Opcode::ADDRESS as u8 => Opcode::ADDRESS,
            x if x == Opcode::BALANCE as u8 => Opcode::BALANCE,
            x if x == Opcode::ORIGIN as u8 => Opcode::ORIGIN,
            x if x == Opcode::CALLER as u8 => Opcode::CALLER,
            x if x == Opcode::CALLVALUE as u8 => Opcode::CALLVALUE,
            x if x == Opcode::CALLDATALOAD as u8 => Opcode::CALLDATALOAD,
            x if x == Opcode::CALLDATASIZE as u8 => Opcode::CALLDATASIZE,
            x if x == Opcode::CALLDATACOPY as u8 => Opcode::CALLDATACOPY,
            x if x == Opcode::CODESIZE as u8 => Opcode::CODESIZE,
            x if x == Opcode::CODECOPY as u8 => Opcode::CODECOPY,
            x if x == Opcode::GASPRICE as u8 => Opcode::GASPRICE,
            x if x == Opcode::EXTCODESIZE as u8 => Opcode::EXTCODESIZE,
            x if x == Opcode::EXTCODECOPY as u8 => Opcode::EXTCODECOPY,
            x if x == Opcode::RETURNDATASIZE as u8 => Opcode::RETURNDATASIZE,
            x if x == Opcode::RETURNDATACOPY as u8 => Opcode::RETURNDATACOPY,
            x if x == Opcode::EXTCODEHASH as u8 => Opcode::EXTCODEHASH,
            x if x == Opcode::BLOCKHASH as u8 => Opcode::BLOCKHASH,
            x if x == Opcode::COINBASE as u8 => Opcode::COINBASE,
            x if x == Opcode::TIMESTAMP as u8 => Opcode::TIMESTAMP,
            x if x == Opcode::NUMBER as u8 => Opcode::NUMBER,
            x if x == Opcode::PREVRANDAO as u8 => Opcode::PREVRANDAO,
            x if x == Opcode::GASLIMIT as u8 => Opcode::GASLIMIT,
            x if x == Opcode::CHAINID as u8 => Opcode::CHAINID,
            x if x == Opcode::SELFBALANCE as u8 => Opcode::SELFBALANCE,
            x if x == Opcode::BASEFEE as u8 => Opcode::BASEFEE,
            x if x == Opcode::BLOBHASH as u8 => Opcode::BLOBHASH,
            x if x == Opcode::BLOBBASEFEE as u8 => Opcode::BLOBBASEFEE,
            x if x == Opcode::POP as u8 => Opcode::POP,
            x if x == Opcode::MLOAD as u8 => Opcode::MLOAD,
            x if x == Opcode::MSTORE as u8 => Opcode::MSTORE,
            x if x == Opcode::MSTORE8 as u8 => Opcode::MSTORE8,
            x if x == Opcode::SLOAD as u8 => Opcode::SLOAD,
            x if x == Opcode::SSTORE as u8 => Opcode::SSTORE,
            x if x == Opcode::JUMP as u8 => Opcode::JUMP,
            x if x == Opcode::JUMPI as u8 => Opcode::JUMPI,
            x if x == Opcode::PC as u8 => Opcode::PC,
            x if x == Opcode::MSIZE as u8 => Opcode::MSIZE,
            x if x == Opcode::GAS as u8 => Opcode::GAS,
            x if x == Opcode::JUMPDEST as u8 => Opcode::JUMPDEST,
            x if x == Opcode::TLOAD as u8 => Opcode::TLOAD,
            x if x == Opcode::TSTORE as u8 => Opcode::TSTORE,
            x if x == Opcode::MCOPY as u8 => Opcode::MCOPY,
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
            x if x == Opcode::PUSH32 as u8 => Opcode::PUSH32,
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
            x if x == Opcode::LOG0 as u8 => Opcode::LOG0,
            x if x == Opcode::LOG1 as u8 => Opcode::LOG1,
            x if x == Opcode::LOG2 as u8 => Opcode::LOG2,
            x if x == Opcode::LOG3 as u8 => Opcode::LOG3,
            x if x == Opcode::LOG4 as u8 => Opcode::LOG4,
            x if x == Opcode::CREATE as u8 => Opcode::CREATE,
            x if x == Opcode::CALL as u8 => Opcode::CALL,
            x if x == Opcode::CALLCODE as u8 => Opcode::CALLCODE,
            x if x == Opcode::RETURN as u8 => Opcode::RETURN,
            x if x == Opcode::DELEGATECALL as u8 => Opcode::DELEGATECALL,
            x if x == Opcode::CREATE2 as u8 => Opcode::CREATE2,
            x if x == Opcode::STATICCALL as u8 => Opcode::STATICCALL,
            x if x == Opcode::REVERT as u8 => Opcode::REVERT,
            x if x == Opcode::SELFDESTRUCT as u8 => Opcode::SELFDESTRUCT,
            x => return Err(OpcodeParseError(x)),
        };

        Ok(op)
    }
}

#[derive(Debug, Clone)]
pub enum Operation {
    Stop,
    Add,
    Mul,
    Sub,
    Div,
    Sdiv,
    Mod,
    SMod,
    Addmod,
    Mulmod,
    Exp,
    SignExtend,
    Lt,
    Gt,
    Slt,
    Sgt,
    Eq,
    IsZero,
    And,
    Or,
    Xor,
    Not,
    Byte,
    Shl,
    Shr,
    Sar,
    Keccak256,
    Address,
    Balance,
    Origin,
    Caller,
    Callvalue,
    CalldataLoad,
    CallDataSize,
    CallDataCopy,
    Codesize,
    Codecopy,
    Gasprice,
    ExtcodeSize,
    ExtcodeCopy,
    ReturnDataSize,
    ReturnDataCopy,
    ExtcodeHash,
    BlockHash,
    Coinbase,
    Timestamp,
    Number,
    Prevrandao,
    Gaslimit,
    Chainid,
    SelfBalance,
    Basefee,
    BlobHash,
    BlobBaseFee,
    Pop,
    Mload,
    Mstore,
    Mstore8,
    Sload,
    Sstore,
    Jump,
    Jumpi,
    PC { pc: usize },
    Msize,
    Gas,
    Jumpdest { pc: usize },
    Tload,
    Tstore,
    Mcopy,
    Push0,
    Push((u8, BigUint)),
    Dup(u8),
    Swap(u8),
    Log(u8),
    Create,
    Call,
    CallCode,
    Return,
    DelegateCall,
    Create2,
    StaticCall,
    Revert,
    Invalid,
    SelfDestruct,
}

impl Operation {
    pub fn to_bytecode(&self) -> Vec<u8> {
        match self {
            Operation::Stop => vec![Opcode::STOP as u8],
            Operation::Add => vec![Opcode::ADD as u8],
            Operation::Mul => vec![Opcode::MUL as u8],
            Operation::Sub => vec![Opcode::SUB as u8],
            Operation::Div => vec![Opcode::DIV as u8],
            Operation::Sdiv => vec![Opcode::SDIV as u8],
            Operation::Mod => vec![Opcode::MOD as u8],
            Operation::SMod => vec![Opcode::SMOD as u8],
            Operation::Addmod => vec![Opcode::ADDMOD as u8],
            Operation::Mulmod => vec![Opcode::MULMOD as u8],
            Operation::Exp => vec![Opcode::EXP as u8],
            Operation::SignExtend => vec![Opcode::SIGNEXTEND as u8],
            Operation::Lt => vec![Opcode::LT as u8],
            Operation::Gt => vec![Opcode::GT as u8],
            Operation::Slt => vec![Opcode::SLT as u8],
            Operation::Sgt => vec![Opcode::SGT as u8],
            Operation::Eq => vec![Opcode::EQ as u8],
            Operation::IsZero => vec![Opcode::ISZERO as u8],
            Operation::And => vec![Opcode::AND as u8],
            Operation::Or => vec![Opcode::OR as u8],
            Operation::Xor => vec![Opcode::XOR as u8],
            Operation::Not => vec![Opcode::NOT as u8],
            Operation::Byte => vec![Opcode::BYTE as u8],
            Operation::Shl => vec![Opcode::SHL as u8],
            Operation::Shr => vec![Opcode::SHR as u8],
            Operation::Sar => vec![Opcode::SAR as u8],
            Operation::Keccak256 => vec![Opcode::KECCAK256 as u8],
            Operation::Address => vec![Opcode::ADDRESS as u8],
            Operation::Balance => vec![Opcode::BALANCE as u8],
            Operation::Origin => vec![Opcode::ORIGIN as u8],
            Operation::Caller => vec![Opcode::CALLER as u8],
            Operation::Callvalue => vec![Opcode::CALLVALUE as u8],
            Operation::CalldataLoad => vec![Opcode::CALLDATALOAD as u8],
            Operation::CallDataSize => vec![Opcode::CALLDATASIZE as u8],
            Operation::CallDataCopy => vec![Opcode::CALLDATACOPY as u8],
            Operation::Codesize => vec![Opcode::CODESIZE as u8],
            Operation::Codecopy => vec![Opcode::CODECOPY as u8],
            Operation::Gasprice => vec![Opcode::GASPRICE as u8],
            Operation::ExtcodeCopy => vec![Opcode::EXTCODECOPY as u8],
            Operation::ReturnDataSize => vec![Opcode::RETURNDATASIZE as u8],
            Operation::ReturnDataCopy => vec![Opcode::RETURNDATACOPY as u8],
            Operation::ExtcodeHash => vec![Opcode::EXTCODEHASH as u8],
            Operation::BlockHash => vec![Opcode::BLOCKHASH as u8],
            Operation::ExtcodeSize => vec![Opcode::EXTCODESIZE as u8],
            Operation::Coinbase => vec![Opcode::COINBASE as u8],
            Operation::Timestamp => vec![Opcode::TIMESTAMP as u8],
            Operation::Number => vec![Opcode::NUMBER as u8],
            Operation::Prevrandao => vec![Opcode::PREVRANDAO as u8],
            Operation::Gaslimit => vec![Opcode::GASLIMIT as u8],
            Operation::Chainid => vec![Opcode::CHAINID as u8],
            Operation::SelfBalance => vec![Opcode::SELFBALANCE as u8],
            Operation::Basefee => vec![Opcode::BASEFEE as u8],
            Operation::BlobHash => vec![Opcode::BLOBHASH as u8],
            Operation::BlobBaseFee => vec![Opcode::BLOBBASEFEE as u8],
            Operation::Pop => vec![Opcode::POP as u8],
            Operation::Mload => vec![Opcode::MLOAD as u8],
            Operation::Mstore => vec![Opcode::MSTORE as u8],
            Operation::Mstore8 => vec![Opcode::MSTORE8 as u8],
            Operation::Sload => vec![Opcode::SLOAD as u8],
            Operation::Sstore => vec![Opcode::SSTORE as u8],
            Operation::Jump => vec![Opcode::JUMP as u8],
            Operation::Jumpi => vec![Opcode::JUMPI as u8],
            Operation::PC { pc: _ } => vec![Opcode::PC as u8],
            Operation::Msize => vec![Opcode::MSIZE as u8],
            Operation::Gas => vec![Opcode::GAS as u8],
            Operation::Jumpdest { pc: _ } => vec![Opcode::JUMPDEST as u8],
            Operation::Tload => vec![Opcode::TLOAD as u8],
            Operation::Tstore => vec![Opcode::TSTORE as u8],
            Operation::Mcopy => vec![Opcode::MCOPY as u8],
            Operation::Push0 => vec![Opcode::PUSH0 as u8],
            Operation::Push((n, x)) => {
                let len = 1 + *n as usize;
                let mut opcode_bytes = vec![0; len];
                opcode_bytes[0] = Opcode::PUSH0 as u8 + n;
                let bytes = x.to_bytes_be();
                opcode_bytes[len - bytes.len()..].copy_from_slice(&bytes);
                opcode_bytes
            }
            Operation::Dup(n) => vec![Opcode::DUP1 as u8 + n - 1],
            Operation::Swap(n) => vec![Opcode::SWAP1 as u8 + n - 1],
            Operation::Log(n) => vec![Opcode::LOG0 as u8 + n],
            Operation::Create => vec![Opcode::CREATE as u8],
            Operation::Call => vec![Opcode::CALL as u8],
            Operation::CallCode => vec![Opcode::CALLCODE as u8],
            Operation::Return => vec![Opcode::RETURN as u8],
            Operation::DelegateCall => vec![Opcode::DELEGATECALL as u8],
            Operation::Create2 => vec![Opcode::CREATE2 as u8],
            Operation::StaticCall => vec![Opcode::STATICCALL as u8],
            Operation::Revert => vec![Opcode::REVERT as u8],
            Operation::Invalid => vec![Opcode::INVALID as u8],
            Operation::SelfDestruct => vec![Opcode::SELFDESTRUCT as u8],
        }
    }
}

#[derive(Debug, Clone)]
pub struct Program {
    pub(crate) operations: Vec<Operation>,
    pub(crate) code_size: u32,
}

impl Program {
    pub fn from_bytecode_checked(bytecode: &[u8]) -> Result<Self, ParseError> {
        let mut operations = vec![];
        let mut pc = 0;
        let mut failed_opcodes = vec![];

        while pc < bytecode.len() {
            match Self::parse_operation(bytecode, pc) {
                Ok((op, new_pc)) => {
                    operations.push(op);
                    pc = new_pc;
                }
                Err(e) => {
                    failed_opcodes.push(e);
                    pc += 1;
                }
            }
        }

        let code_size = Self::get_codesize(&operations);

        if failed_opcodes.is_empty() {
            Ok(Program {
                operations,
                code_size,
            })
        } else {
            Err(ParseError(failed_opcodes))
        }
    }

    pub fn from_bytecode(bytecode: &[u8]) -> Self {
        let mut operations = vec![];
        let mut pc = 0;

        while pc < bytecode.len() {
            match Self::parse_operation(bytecode, pc) {
                Ok((op, new_pc)) => {
                    operations.push(op);
                    pc = new_pc;
                }
                Err(_) => {
                    operations.push(Operation::Invalid);
                    pc += 1;
                }
            }
        }

        let code_size = Self::get_codesize(&operations);

        Program {
            operations,
            code_size,
        }
    }

    pub fn to_bytecode(self) -> Vec<u8> {
        self.operations
            .iter()
            .flat_map(Operation::to_bytecode)
            .collect::<Vec<u8>>()
    }

    fn parse_operation(
        bytecode: &[u8],
        mut pc: usize,
    ) -> Result<(Operation, usize), OpcodeParseError> {
        let opcode = Opcode::try_from(bytecode[pc])?;

        let op = match opcode {
            Opcode::STOP => Operation::Stop,
            Opcode::ADD => Operation::Add,
            Opcode::MUL => Operation::Mul,
            Opcode::SUB => Operation::Sub,
            Opcode::DIV => Operation::Div,
            Opcode::SDIV => Operation::Sdiv,
            Opcode::MOD => Operation::Mod,
            Opcode::SMOD => Operation::SMod,
            Opcode::ADDMOD => Operation::Addmod,
            Opcode::MULMOD => Operation::Mulmod,
            Opcode::EXP => Operation::Exp,
            Opcode::SIGNEXTEND => Operation::SignExtend,
            Opcode::LT => Operation::Lt,
            Opcode::GT => Operation::Gt,
            Opcode::SLT => Operation::Slt,
            Opcode::SGT => Operation::Sgt,
            Opcode::EQ => Operation::Eq,
            Opcode::ISZERO => Operation::IsZero,
            Opcode::AND => Operation::And,
            Opcode::OR => Operation::Or,
            Opcode::XOR => Operation::Xor,
            Opcode::NOT => Operation::Not,
            Opcode::BYTE => Operation::Byte,
            Opcode::SHL => Operation::Shl,
            Opcode::SHR => Operation::Shr,
            Opcode::SAR => Operation::Sar,
            Opcode::KECCAK256 => Operation::Keccak256,
            Opcode::ADDRESS => Operation::Address,
            Opcode::BALANCE => Operation::Balance,
            Opcode::ORIGIN => Operation::Origin,
            Opcode::CALLER => Operation::Caller,
            Opcode::CALLVALUE => Operation::Callvalue,
            Opcode::CALLDATALOAD => Operation::CalldataLoad,
            Opcode::CALLDATASIZE => Operation::CallDataSize,
            Opcode::CALLDATACOPY => Operation::CallDataCopy,
            Opcode::CODESIZE => Operation::Codesize,
            Opcode::CODECOPY => Operation::Codecopy,
            Opcode::GASPRICE => Operation::Gasprice,
            Opcode::EXTCODESIZE => Operation::ExtcodeSize,
            Opcode::EXTCODECOPY => Operation::ExtcodeCopy,
            Opcode::RETURNDATASIZE => Operation::ReturnDataSize,
            Opcode::RETURNDATACOPY => Operation::ReturnDataCopy,
            Opcode::EXTCODEHASH => Operation::ExtcodeHash,
            Opcode::BLOCKHASH => Operation::BlockHash,
            Opcode::COINBASE => Operation::Coinbase,
            Opcode::TIMESTAMP => Operation::Timestamp,
            Opcode::NUMBER => Operation::Number,
            Opcode::PREVRANDAO => Operation::Prevrandao,
            Opcode::GASLIMIT => Operation::Gaslimit,
            Opcode::CHAINID => Operation::Chainid,
            Opcode::SELFBALANCE => Operation::SelfBalance,
            Opcode::BASEFEE => Operation::Basefee,
            Opcode::BLOBHASH => Operation::BlobHash,
            Opcode::BLOBBASEFEE => Operation::BlobBaseFee,
            Opcode::POP => Operation::Pop,
            Opcode::MLOAD => Operation::Mload,
            Opcode::MSTORE => Operation::Mstore,
            Opcode::MSTORE8 => Operation::Mstore8,
            Opcode::SLOAD => Operation::Sload,
            Opcode::SSTORE => Operation::Sstore,
            Opcode::JUMP => Operation::Jump,
            Opcode::JUMPI => Operation::Jumpi,
            Opcode::PC => Operation::PC { pc },
            Opcode::MSIZE => Operation::Msize,
            Opcode::GAS => Operation::Gas,
            Opcode::JUMPDEST => Operation::Jumpdest { pc },
            Opcode::TLOAD => Operation::Tload,
            Opcode::TSTORE => Operation::Tstore,
            Opcode::MCOPY => Operation::Mcopy,
            Opcode::PUSH0 => Operation::Push0,
            Opcode::PUSH1 => {
                // TODO: return error if not enough bytes (same for PUSHN)
                pc += 1;
                let x = bytecode[pc..min(bytecode.len(), pc + 1)]
                    .try_into()
                    .unwrap();
                Operation::Push((1, (BigUint::from_bytes_be(x))))
            }
            Opcode::PUSH2 => {
                pc += 1;
                let x = bytecode[pc..min(bytecode.len(), pc + 2)]
                    .try_into()
                    .unwrap();
                pc += 1;
                Operation::Push((2, (BigUint::from_bytes_be(x))))
            }
            Opcode::PUSH3 => {
                pc += 1;
                let x = bytecode[pc..min(bytecode.len(), pc + 3)]
                    .try_into()
                    .unwrap();
                pc += 2;
                Operation::Push((3, (BigUint::from_bytes_be(x))))
            }
            Opcode::PUSH4 => {
                pc += 1;
                let x = bytecode[pc..min(bytecode.len(), pc + 4)]
                    .try_into()
                    .unwrap();
                pc += 3;
                Operation::Push((4, (BigUint::from_bytes_be(x))))
            }
            Opcode::PUSH5 => {
                pc += 1;
                let x = bytecode[pc..min(bytecode.len(), pc + 5)]
                    .try_into()
                    .unwrap();
                pc += 4;
                Operation::Push((5, (BigUint::from_bytes_be(x))))
            }
            Opcode::PUSH6 => {
                pc += 1;
                let x = bytecode[pc..min(bytecode.len(), pc + 6)]
                    .try_into()
                    .unwrap();
                pc += 5;
                Operation::Push((6, (BigUint::from_bytes_be(x))))
            }
            Opcode::PUSH7 => {
                pc += 1;
                let x = bytecode[pc..min(bytecode.len(), pc + 7)]
                    .try_into()
                    .unwrap();
                pc += 6;
                Operation::Push((7, (BigUint::from_bytes_be(x))))
            }
            Opcode::PUSH8 => {
                pc += 1;
                let x = bytecode[pc..min(bytecode.len(), pc + 8)]
                    .try_into()
                    .unwrap();
                pc += 7;
                Operation::Push((8, (BigUint::from_bytes_be(x))))
            }
            Opcode::PUSH9 => {
                pc += 1;
                let x = bytecode[pc..min(bytecode.len(), pc + 9)]
                    .try_into()
                    .unwrap();
                pc += 8;
                Operation::Push((9, (BigUint::from_bytes_be(x))))
            }
            Opcode::PUSH10 => {
                pc += 1;
                let x = bytecode[pc..min(bytecode.len(), pc + 10)]
                    .try_into()
                    .unwrap();
                pc += 9;
                Operation::Push((10, (BigUint::from_bytes_be(x))))
            }
            Opcode::PUSH11 => {
                pc += 1;
                let x = bytecode[pc..min(bytecode.len(), pc + 11)]
                    .try_into()
                    .unwrap();
                pc += 10;
                Operation::Push((11, (BigUint::from_bytes_be(x))))
            }
            Opcode::PUSH12 => {
                pc += 1;
                let x = bytecode[pc..min(bytecode.len(), pc + 12)]
                    .try_into()
                    .unwrap();
                pc += 11;
                Operation::Push((12, (BigUint::from_bytes_be(x))))
            }
            Opcode::PUSH13 => {
                pc += 1;
                let x = bytecode[pc..min(bytecode.len(), pc + 13)]
                    .try_into()
                    .unwrap();
                pc += 12;
                Operation::Push((13, (BigUint::from_bytes_be(x))))
            }
            Opcode::PUSH14 => {
                pc += 1;
                let x = bytecode[pc..min(bytecode.len(), pc + 14)]
                    .try_into()
                    .unwrap();
                pc += 13;
                Operation::Push((14, (BigUint::from_bytes_be(x))))
            }
            Opcode::PUSH15 => {
                pc += 1;
                let x = bytecode[pc..min(bytecode.len(), pc + 15)]
                    .try_into()
                    .unwrap();
                pc += 14;
                Operation::Push((15, (BigUint::from_bytes_be(x))))
            }
            Opcode::PUSH16 => {
                pc += 1;
                let x = bytecode[pc..min(bytecode.len(), pc + 16)]
                    .try_into()
                    .unwrap();
                pc += 15;
                Operation::Push((16, (BigUint::from_bytes_be(x))))
            }
            Opcode::PUSH17 => {
                pc += 1;
                let x = bytecode[pc..min(bytecode.len(), pc + 17)]
                    .try_into()
                    .unwrap();
                pc += 16;
                Operation::Push((17, (BigUint::from_bytes_be(x))))
            }
            Opcode::PUSH18 => {
                pc += 1;
                let x = bytecode[pc..min(bytecode.len(), pc + 18)]
                    .try_into()
                    .unwrap();
                pc += 17;
                Operation::Push((18, (BigUint::from_bytes_be(x))))
            }
            Opcode::PUSH19 => {
                pc += 1;
                let x = bytecode[pc..min(bytecode.len(), pc + 19)]
                    .try_into()
                    .unwrap();
                pc += 18;
                Operation::Push((19, (BigUint::from_bytes_be(x))))
            }
            Opcode::PUSH20 => {
                pc += 1;
                let x = bytecode[pc..min(bytecode.len(), pc + 20)]
                    .try_into()
                    .unwrap();
                pc += 19;
                Operation::Push((20, (BigUint::from_bytes_be(x))))
            }
            Opcode::PUSH21 => {
                pc += 1;
                let x = bytecode[pc..min(bytecode.len(), pc + 21)]
                    .try_into()
                    .unwrap();
                pc += 20;
                Operation::Push((21, (BigUint::from_bytes_be(x))))
            }
            Opcode::PUSH22 => {
                pc += 1;
                let x = bytecode[pc..min(bytecode.len(), pc + 22)]
                    .try_into()
                    .unwrap();
                pc += 21;
                Operation::Push((22, (BigUint::from_bytes_be(x))))
            }
            Opcode::PUSH23 => {
                pc += 1;
                let x = bytecode[pc..min(bytecode.len(), pc + 23)]
                    .try_into()
                    .unwrap();
                pc += 22;
                Operation::Push((23, (BigUint::from_bytes_be(x))))
            }
            Opcode::PUSH24 => {
                pc += 1;
                let x = bytecode[pc..min(bytecode.len(), pc + 24)]
                    .try_into()
                    .unwrap();
                pc += 23;
                Operation::Push((24, (BigUint::from_bytes_be(x))))
            }
            Opcode::PUSH25 => {
                pc += 1;
                let x = bytecode[pc..min(bytecode.len(), pc + 25)]
                    .try_into()
                    .unwrap();
                pc += 24;
                Operation::Push((25, (BigUint::from_bytes_be(x))))
            }
            Opcode::PUSH26 => {
                pc += 1;
                let x = bytecode[pc..min(bytecode.len(), pc + 26)]
                    .try_into()
                    .unwrap();
                pc += 25;
                Operation::Push((26, (BigUint::from_bytes_be(x))))
            }
            Opcode::PUSH27 => {
                pc += 1;
                let x = bytecode[pc..min(bytecode.len(), pc + 27)]
                    .try_into()
                    .unwrap();
                pc += 26;
                Operation::Push((27, (BigUint::from_bytes_be(x))))
            }
            Opcode::PUSH28 => {
                pc += 1;
                let x = bytecode[pc..min(bytecode.len(), pc + 28)]
                    .try_into()
                    .unwrap();
                pc += 27;
                Operation::Push((28, (BigUint::from_bytes_be(x))))
            }
            Opcode::PUSH29 => {
                pc += 1;
                let x = bytecode[pc..min(bytecode.len(), pc + 29)]
                    .try_into()
                    .unwrap();
                pc += 28;
                Operation::Push((29, (BigUint::from_bytes_be(x))))
            }
            Opcode::PUSH30 => {
                pc += 1;
                let x = bytecode[pc..min(bytecode.len(), pc + 30)]
                    .try_into()
                    .unwrap();
                pc += 29;
                Operation::Push((30, (BigUint::from_bytes_be(x))))
            }
            Opcode::PUSH31 => {
                pc += 1;
                let x = bytecode[pc..min(bytecode.len(), pc + 31)]
                    .try_into()
                    .unwrap();
                pc += 30;
                Operation::Push((31, (BigUint::from_bytes_be(x))))
            }
            Opcode::PUSH32 => {
                pc += 1;
                let x = bytecode[pc..min(bytecode.len(), pc + 32)]
                    .try_into()
                    .unwrap();
                pc += 31;
                Operation::Push((32, (BigUint::from_bytes_be(x))))
            }
            Opcode::DUP1 => Operation::Dup(1),
            Opcode::DUP2 => Operation::Dup(2),
            Opcode::DUP3 => Operation::Dup(3),
            Opcode::DUP4 => Operation::Dup(4),
            Opcode::DUP5 => Operation::Dup(5),
            Opcode::DUP6 => Operation::Dup(6),
            Opcode::DUP7 => Operation::Dup(7),
            Opcode::DUP8 => Operation::Dup(8),
            Opcode::DUP9 => Operation::Dup(9),
            Opcode::DUP10 => Operation::Dup(10),
            Opcode::DUP11 => Operation::Dup(11),
            Opcode::DUP12 => Operation::Dup(12),
            Opcode::DUP13 => Operation::Dup(13),
            Opcode::DUP14 => Operation::Dup(14),
            Opcode::DUP15 => Operation::Dup(15),
            Opcode::DUP16 => Operation::Dup(16),
            Opcode::SWAP1 => Operation::Swap(1),
            Opcode::SWAP2 => Operation::Swap(2),
            Opcode::SWAP3 => Operation::Swap(3),
            Opcode::SWAP4 => Operation::Swap(4),
            Opcode::SWAP5 => Operation::Swap(5),
            Opcode::SWAP6 => Operation::Swap(6),
            Opcode::SWAP7 => Operation::Swap(7),
            Opcode::SWAP8 => Operation::Swap(8),
            Opcode::SWAP9 => Operation::Swap(9),
            Opcode::SWAP10 => Operation::Swap(10),
            Opcode::SWAP11 => Operation::Swap(11),
            Opcode::SWAP12 => Operation::Swap(12),
            Opcode::SWAP13 => Operation::Swap(13),
            Opcode::SWAP14 => Operation::Swap(14),
            Opcode::SWAP15 => Operation::Swap(15),
            Opcode::SWAP16 => Operation::Swap(16),
            Opcode::LOG0 => Operation::Log(0),
            Opcode::LOG1 => Operation::Log(1),
            Opcode::LOG2 => Operation::Log(2),
            Opcode::LOG3 => Operation::Log(3),
            Opcode::LOG4 => Operation::Log(4),
            Opcode::CREATE => Operation::Create,
            Opcode::CALL => Operation::Call,
            Opcode::CALLCODE => Operation::CallCode,
            Opcode::RETURN => Operation::Return,
            Opcode::DELEGATECALL => Operation::DelegateCall,
            Opcode::CREATE2 => Operation::Create2,
            Opcode::STATICCALL => Operation::StaticCall,
            Opcode::REVERT => Operation::Revert,
            Opcode::INVALID => Operation::Invalid,
            Opcode::SELFDESTRUCT => Operation::SelfDestruct,
        };
        pc += 1;

        Ok((op, pc))
    }

    fn get_codesize(operations: &[Operation]) -> u32 {
        operations
            .iter()
            .map(|op| match op {
                // the size in bytes to push + 1 from the PUSHN opcode
                Operation::Push((size, _)) => (size + 1) as u32,
                _ => 1,
            })
            .sum()
    }
}

impl From<Vec<Operation>> for Program {
    fn from(operations: Vec<Operation>) -> Self {
        let code_size = Self::get_codesize(&operations);

        Program {
            operations,
            code_size,
        }
    }
}
