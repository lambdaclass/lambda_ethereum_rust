use crate::{errors::VMError, opcodes::Opcode};
use bytes::Bytes;
use ethereum_rust_core::U256;

#[derive(Debug, PartialEq, Eq, Clone)]
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
    CallDataLoad,
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
    PC,
    Msize,
    Gas,
    Jumpdest,
    Tload,
    Tstore,
    Mcopy,
    Push0,
    Push((u8, U256)),
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
    pub fn to_bytecode(&self) -> Result<Bytes, VMError> {
        let bytecode = match self {
            Operation::Stop => Bytes::copy_from_slice(&[Opcode::STOP.to_u8()]),
            Operation::Add => Bytes::copy_from_slice(&[Opcode::ADD.to_u8()]),
            Operation::Mul => Bytes::copy_from_slice(&[Opcode::MUL.to_u8()]),
            Operation::Sub => Bytes::copy_from_slice(&[Opcode::SUB.to_u8()]),
            Operation::Div => Bytes::copy_from_slice(&[Opcode::DIV.to_u8()]),
            Operation::Sdiv => Bytes::copy_from_slice(&[Opcode::SDIV.to_u8()]),
            Operation::Mod => Bytes::copy_from_slice(&[Opcode::MOD.to_u8()]),
            Operation::SMod => Bytes::copy_from_slice(&[Opcode::SMOD.to_u8()]),
            Operation::Addmod => Bytes::copy_from_slice(&[Opcode::ADDMOD.to_u8()]),
            Operation::Mulmod => Bytes::copy_from_slice(&[Opcode::MULMOD.to_u8()]),
            Operation::Exp => Bytes::copy_from_slice(&[Opcode::EXP.to_u8()]),
            Operation::SignExtend => Bytes::copy_from_slice(&[Opcode::SIGNEXTEND.to_u8()]),
            Operation::Lt => Bytes::copy_from_slice(&[Opcode::LT.to_u8()]),
            Operation::Gt => Bytes::copy_from_slice(&[Opcode::GT.to_u8()]),
            Operation::Slt => Bytes::copy_from_slice(&[Opcode::SLT.to_u8()]),
            Operation::Sgt => Bytes::copy_from_slice(&[Opcode::SGT.to_u8()]),
            Operation::Eq => Bytes::copy_from_slice(&[Opcode::EQ.to_u8()]),
            Operation::IsZero => Bytes::copy_from_slice(&[Opcode::ISZERO.to_u8()]),
            Operation::And => Bytes::copy_from_slice(&[Opcode::AND.to_u8()]),
            Operation::Or => Bytes::copy_from_slice(&[Opcode::OR.to_u8()]),
            Operation::Xor => Bytes::copy_from_slice(&[Opcode::XOR.to_u8()]),
            Operation::Not => Bytes::copy_from_slice(&[Opcode::NOT.to_u8()]),
            Operation::Byte => Bytes::copy_from_slice(&[Opcode::BYTE.to_u8()]),
            Operation::Shl => Bytes::copy_from_slice(&[Opcode::SHL.to_u8()]),
            Operation::Shr => Bytes::copy_from_slice(&[Opcode::SHR.to_u8()]),
            Operation::Sar => Bytes::copy_from_slice(&[Opcode::SAR.to_u8()]),
            Operation::Keccak256 => Bytes::copy_from_slice(&[Opcode::KECCAK256.to_u8()]),
            Operation::Address => Bytes::copy_from_slice(&[Opcode::ADDRESS.to_u8()]),
            Operation::Balance => Bytes::copy_from_slice(&[Opcode::BALANCE.to_u8()]),
            Operation::Origin => Bytes::copy_from_slice(&[Opcode::ORIGIN.to_u8()]),
            Operation::Caller => Bytes::copy_from_slice(&[Opcode::CALLER.to_u8()]),
            Operation::Callvalue => Bytes::copy_from_slice(&[Opcode::CALLVALUE.to_u8()]),
            Operation::CallDataLoad => Bytes::copy_from_slice(&[Opcode::CALLDATALOAD.to_u8()]),
            Operation::CallDataSize => Bytes::copy_from_slice(&[Opcode::CALLDATASIZE.to_u8()]),
            Operation::CallDataCopy => Bytes::copy_from_slice(&[Opcode::CALLDATACOPY.to_u8()]),
            Operation::Codesize => Bytes::copy_from_slice(&[Opcode::CODESIZE.to_u8()]),
            Operation::Codecopy => Bytes::copy_from_slice(&[Opcode::CODECOPY.to_u8()]),
            Operation::Gasprice => Bytes::copy_from_slice(&[Opcode::GASPRICE.to_u8()]),
            Operation::ExtcodeSize => Bytes::copy_from_slice(&[Opcode::EXTCODESIZE.to_u8()]),
            Operation::ExtcodeCopy => Bytes::copy_from_slice(&[Opcode::EXTCODECOPY.to_u8()]),
            Operation::ReturnDataSize => Bytes::copy_from_slice(&[Opcode::RETURNDATASIZE.to_u8()]),
            Operation::ReturnDataCopy => Bytes::copy_from_slice(&[Opcode::RETURNDATACOPY.to_u8()]),
            Operation::ExtcodeHash => Bytes::copy_from_slice(&[Opcode::EXTCODEHASH.to_u8()]),
            Operation::BlockHash => Bytes::copy_from_slice(&[Opcode::BLOCKHASH.to_u8()]),
            Operation::Coinbase => Bytes::copy_from_slice(&[Opcode::COINBASE.to_u8()]),
            Operation::Timestamp => Bytes::copy_from_slice(&[Opcode::TIMESTAMP.to_u8()]),
            Operation::Number => Bytes::copy_from_slice(&[Opcode::NUMBER.to_u8()]),
            Operation::Prevrandao => Bytes::copy_from_slice(&[Opcode::PREVRANDAO.to_u8()]),
            Operation::Gaslimit => Bytes::copy_from_slice(&[Opcode::GASLIMIT.to_u8()]),
            Operation::Chainid => Bytes::copy_from_slice(&[Opcode::CHAINID.to_u8()]),
            Operation::SelfBalance => Bytes::copy_from_slice(&[Opcode::SELFBALANCE.to_u8()]),
            Operation::Basefee => Bytes::copy_from_slice(&[Opcode::BASEFEE.to_u8()]),
            Operation::BlobHash => Bytes::copy_from_slice(&[Opcode::BLOBHASH.to_u8()]),
            Operation::BlobBaseFee => Bytes::copy_from_slice(&[Opcode::BLOBBASEFEE.to_u8()]),
            Operation::Pop => Bytes::copy_from_slice(&[Opcode::POP.to_u8()]),
            Operation::Mload => Bytes::copy_from_slice(&[Opcode::MLOAD.to_u8()]),
            Operation::Mstore => Bytes::copy_from_slice(&[Opcode::MSTORE.to_u8()]),
            Operation::Mstore8 => Bytes::copy_from_slice(&[Opcode::MSTORE8.to_u8()]),
            Operation::Sload => Bytes::copy_from_slice(&[Opcode::SLOAD.to_u8()]),
            Operation::Sstore => Bytes::copy_from_slice(&[Opcode::SSTORE.to_u8()]),
            Operation::Jump => Bytes::copy_from_slice(&[Opcode::JUMP.to_u8()]),
            Operation::Jumpi => Bytes::copy_from_slice(&[Opcode::JUMPI.to_u8()]),
            Operation::PC => Bytes::copy_from_slice(&[Opcode::PC.to_u8()]),
            Operation::Msize => Bytes::copy_from_slice(&[Opcode::MSIZE.to_u8()]),
            Operation::Gas => Bytes::copy_from_slice(&[Opcode::GAS.to_u8()]),
            Operation::Jumpdest => Bytes::copy_from_slice(&[Opcode::JUMPDEST.to_u8()]),
            Operation::Tload => Bytes::copy_from_slice(&[Opcode::TLOAD.to_u8()]),
            Operation::Tstore => Bytes::copy_from_slice(&[Opcode::TSTORE.to_u8()]),
            Operation::Mcopy => Bytes::copy_from_slice(&[Opcode::MCOPY.to_u8()]),
            Operation::Push0 => Bytes::copy_from_slice(&[Opcode::PUSH0.to_u8()]),
            Operation::Push((n, value)) => {
                let n_usize: usize = (*n).into();
                assert!(*n <= 32, "PUSH32 is the max");
                // the amount of bytes needed to represent the value must
                // be less than the n in PUSHn
                assert!(
                    value.bits().div_ceil(8) <= n_usize,
                    "value doesn't fit in n bytes"
                );
                let mut word_buffer = [0; 32];
                value.to_big_endian(&mut word_buffer);
                // extract the last n bytes to push
                let value_to_push = &word_buffer.get((32 - n_usize)..).ok_or(VMError::SlicingError)?;
                assert_eq!(value_to_push.len(), n_usize);
                let opcode = Opcode::try_from(Opcode::PUSH0.to_u8() + *n)?;
                let mut bytes = vec![opcode.to_u8()];
                bytes.extend_from_slice(value_to_push);

                Bytes::copy_from_slice(&bytes)
            }
            Operation::Dup(n) => {
                assert!(*n <= 16, "DUP16 is the max");
                Bytes::copy_from_slice(&[Opcode::DUP1.to_u8() + n - 1])
            }
            Operation::Swap(n) => {
                assert!(*n <= 16, "SWAP16 is the max");
                Bytes::copy_from_slice(&[Opcode::SWAP1.to_u8() + n - 1])
            }
            Operation::Log(n) => {
                assert!(*n <= 4, "LOG4 is the max");
                Bytes::copy_from_slice(&[Opcode::LOG0.to_u8() + n])
            }
            Operation::Create => Bytes::copy_from_slice(&[Opcode::CREATE.to_u8()]),
            Operation::Call => Bytes::copy_from_slice(&[Opcode::CALL.to_u8()]),
            Operation::CallCode => Bytes::copy_from_slice(&[Opcode::CALLCODE.to_u8()]),
            Operation::Return => Bytes::copy_from_slice(&[Opcode::RETURN.to_u8()]),
            Operation::DelegateCall => Bytes::copy_from_slice(&[Opcode::DELEGATECALL.to_u8()]),
            Operation::Create2 => Bytes::copy_from_slice(&[Opcode::CREATE2.to_u8()]),
            Operation::StaticCall => Bytes::copy_from_slice(&[Opcode::STATICCALL.to_u8()]),
            Operation::Revert => Bytes::copy_from_slice(&[Opcode::REVERT.to_u8()]),
            Operation::Invalid => Bytes::copy_from_slice(&[Opcode::INVALID.to_u8()]),
            Operation::SelfDestruct => Bytes::copy_from_slice(&[Opcode::SELFDESTRUCT.to_u8()]),
        };
        Ok(bytecode)
    }
}
