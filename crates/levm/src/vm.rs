use std::collections::HashMap;

use crate::{opcodes::Opcode, operations::Operation};
use bytes::Bytes;
use ethereum_types::{Address, U256, U512};

#[derive(Debug, Clone, Default)]
pub struct VM {
    pub stack: Vec<U256>, // max 1024 in the future
    pub memory: Memory,
    pub pc: usize,
    pub transient_storage: TransientStorage, // TODO: this should actually go inside an execution environment!
    pub caller: Address,
}

/// Shifts the value to the right by 255 bits and checks the most significant bit is a 1
fn is_negative(value: U256) -> bool {
    value.bit(255)
}
/// negates a number in two's complement
fn negate(value: U256) -> U256 {
    !value + U256::one()
}

impl VM {
    pub fn execute(&mut self, mut bytecode: Bytes) {
        loop {
            match self.next_opcode(&mut bytecode).unwrap() {
                Opcode::STOP => break,
                Opcode::ADD => {
                    let augend = self.stack.pop().unwrap();
                    let addend = self.stack.pop().unwrap();
                    let sum = augend.overflowing_add(addend).0;
                    self.stack.push(sum);
                }
                Opcode::MUL => {
                    let multiplicand = self.stack.pop().unwrap();
                    let multiplier = self.stack.pop().unwrap();
                    let product = multiplicand.overflowing_mul(multiplier).0;
                    self.stack.push(product);
                }
                Opcode::SUB => {
                    let minuend = self.stack.pop().unwrap();
                    let subtrahend = self.stack.pop().unwrap();
                    let difference = minuend.overflowing_sub(subtrahend).0;
                    self.stack.push(difference);
                }
                Opcode::DIV => {
                    let dividend = self.stack.pop().unwrap();
                    let divisor = self.stack.pop().unwrap();
                    if divisor.is_zero() {
                        self.stack.push(U256::zero());
                        continue;
                    }
                    let quotient = dividend / divisor;
                    self.stack.push(quotient);
                }
                Opcode::SDIV => {
                    let dividend = self.stack.pop().unwrap();
                    let divisor = self.stack.pop().unwrap();
                    if divisor.is_zero() {
                        self.stack.push(U256::zero());
                        continue;
                    }

                    let dividend_is_negative = is_negative(dividend);
                    let divisor_is_negative = is_negative(divisor);
                    let dividend = if dividend_is_negative {
                        negate(dividend)
                    } else {
                        dividend
                    };
                    let divisor = if divisor_is_negative {
                        negate(divisor)
                    } else {
                        divisor
                    };
                    let quotient = dividend / divisor;
                    let quotient_is_negative = dividend_is_negative ^ divisor_is_negative;
                    let quotient = if quotient_is_negative {
                        negate(quotient)
                    } else {
                        quotient
                    };

                    self.stack.push(quotient);
                }
                Opcode::MOD => {
                    let dividend = self.stack.pop().unwrap();
                    let divisor = self.stack.pop().unwrap();
                    if divisor.is_zero() {
                        self.stack.push(U256::zero());
                        continue;
                    }
                    let remainder = dividend % divisor;
                    self.stack.push(remainder);
                }
                Opcode::SMOD => {
                    let dividend = self.stack.pop().unwrap();
                    let divisor = self.stack.pop().unwrap();
                    if divisor.is_zero() {
                        self.stack.push(U256::zero());
                        continue;
                    }

                    let dividend_is_negative = is_negative(dividend);
                    let divisor_is_negative = is_negative(divisor);
                    let dividend = if dividend_is_negative {
                        negate(dividend)
                    } else {
                        dividend
                    };
                    let divisor = if divisor_is_negative {
                        negate(divisor)
                    } else {
                        divisor
                    };
                    let remainder = dividend % divisor;
                    let remainder_is_negative = dividend_is_negative ^ divisor_is_negative;
                    let remainder = if remainder_is_negative {
                        negate(remainder)
                    } else {
                        remainder
                    };

                    self.stack.push(remainder);
                }
                Opcode::ADDMOD => {
                    let augend = self.stack.pop().unwrap();
                    let addend = self.stack.pop().unwrap();
                    let divisor = self.stack.pop().unwrap();
                    if divisor.is_zero() {
                        self.stack.push(U256::zero());
                        continue;
                    }
                    let (sum, overflow) = augend.overflowing_add(addend);
                    let mut remainder = sum % divisor;
                    if overflow || remainder > divisor {
                        remainder = remainder.overflowing_sub(divisor).0;
                    }

                    self.stack.push(remainder);
                }
                Opcode::MULMOD => {
                    let multiplicand = U512::from(self.stack.pop().unwrap());

                    let multiplier = U512::from(self.stack.pop().unwrap());
                    let divisor = U512::from(self.stack.pop().unwrap());
                    if divisor.is_zero() {
                        self.stack.push(U256::zero());
                        continue;
                    }

                    let (product, overflow) = multiplicand.overflowing_mul(multiplier);
                    let mut remainder = product % divisor;
                    if overflow || remainder > divisor {
                        remainder = remainder.overflowing_sub(divisor).0;
                    }
                    let mut result = Vec::new();
                    for byte in remainder.0.iter().take(4) {
                        let bytes = byte.to_le_bytes();
                        result.extend_from_slice(&bytes);
                    }
                    // before reverse we have something like [120, 255, 0, 0....]
                    // after reverse we get the [0, 0, ...., 255, 120] which is the correct order for the little endian u256
                    result.reverse();
                    let remainder = U256::from(result.as_slice());
                    self.stack.push(remainder);
                }
                Opcode::EXP => {
                    let base = self.stack.pop().unwrap();
                    let exponent = self.stack.pop().unwrap();
                    let power = base.overflowing_pow(exponent).0;
                    self.stack.push(power);
                }
                Opcode::SIGNEXTEND => {
                    let byte_size = self.stack.pop().unwrap();
                    let value_to_extend = self.stack.pop().unwrap();

                    let bits_per_byte = U256::from(8);
                    let sign_bit_position_on_byte = 7;
                    let max_byte_size = 31;

                    let byte_size = byte_size.min(U256::from(max_byte_size));
                    let sign_bit_index = bits_per_byte * byte_size + sign_bit_position_on_byte;
                    let is_negative = value_to_extend.bit(sign_bit_index.as_usize());
                    let sign_bit_mask = (U256::one() << sign_bit_index) - U256::one();
                    let result = if is_negative {
                        value_to_extend | !sign_bit_mask
                    } else {
                        value_to_extend & sign_bit_mask
                    };
                    self.stack.push(result);
                }
                Opcode::LT => {
                    let lho = self.stack.pop().unwrap();
                    let rho = self.stack.pop().unwrap();
                    let result = if lho < rho { U256::one() } else { U256::zero() };
                    self.stack.push(result);
                }
                Opcode::GT => {
                    let lho = self.stack.pop().unwrap();
                    let rho = self.stack.pop().unwrap();
                    let result = if lho > rho { U256::one() } else { U256::zero() };
                    self.stack.push(result);
                }
                Opcode::SLT => {
                    let lho = self.stack.pop().unwrap();
                    let rho = self.stack.pop().unwrap();
                    let lho_is_negative = lho.bit(255);
                    let rho_is_negative = rho.bit(255);
                    let result = if lho_is_negative == rho_is_negative {
                        // if both have the same sign, compare their magnitudes
                        if lho < rho {
                            U256::one()
                        } else {
                            U256::zero()
                        }
                    } else {
                        // if they have different signs, the negative number is smaller
                        if lho_is_negative {
                            U256::one()
                        } else {
                            U256::zero()
                        }
                    };
                    self.stack.push(result);
                }
                Opcode::SGT => {
                    let lho = self.stack.pop().unwrap();
                    let rho = self.stack.pop().unwrap();
                    let lho_is_negative = lho.bit(255);
                    let rho_is_negative = rho.bit(255);
                    let result = if lho_is_negative == rho_is_negative {
                        // if both have the same sign, compare their magnitudes
                        if lho > rho {
                            U256::one()
                        } else {
                            U256::zero()
                        }
                    } else {
                        // if they have different signs, the positive number is bigger
                        if rho_is_negative {
                            U256::one()
                        } else {
                            U256::zero()
                        }
                    };
                    self.stack.push(result);
                }
                Opcode::EQ => {
                    let lho = self.stack.pop().unwrap();
                    let rho = self.stack.pop().unwrap();
                    let result = if lho == rho {
                        U256::one()
                    } else {
                        U256::zero()
                    };
                    self.stack.push(result);
                }
                Opcode::ISZERO => {
                    let operand = self.stack.pop().unwrap();
                    let result = if operand == U256::zero() {
                        U256::one()
                    } else {
                        U256::zero()
                    };
                    self.stack.push(result);
                }
                Opcode::PUSH0 => {
                    self.stack.push(U256::zero());
                }
                // PUSHn
                op if (Opcode::PUSH1..Opcode::PUSH32).contains(&op) => {
                    let n_bytes = (op as u8) - (Opcode::PUSH1 as u8) + 1;
                    let next_n_bytes = bytecode
                        .get(self.pc..self.pc + n_bytes as usize)
                        .expect("invalid bytecode");
                    let value_to_push = U256::from(next_n_bytes);
                    self.stack.push(value_to_push);
                    self.increment_pc_by(n_bytes as usize);
                }
                Opcode::PUSH32 => {
                    let next_32_bytes = bytecode.get(self.pc..self.pc + 32).unwrap();
                    let value_to_push = U256::from(next_32_bytes);
                    self.stack.push(value_to_push);
                    self.increment_pc_by(32);
                }
                // DUPn
                op if (Opcode::DUP1..=Opcode::DUP16).contains(&op) => {
                    let depth = (op as u8) - (Opcode::DUP1 as u8) + 1;
                    assert!(
                        self.stack.len().ge(&(depth as usize)),
                        "stack underflow: not enough values on the stack"
                    );
                    let value_at_depth = self.stack.get(self.stack.len() - depth as usize).unwrap();
                    self.stack.push(*value_at_depth);
                }
                // SWAPn
                op if (Opcode::SWAP1..=Opcode::SWAP16).contains(&op) => {
                    let depth = (op as u8) - (Opcode::SWAP1 as u8) + 1;
                    assert!(
                        self.stack.len().ge(&(depth as usize)),
                        "stack underflow: not enough values on the stack"
                    );
                    let stack_top_index = self.stack.len();
                    let to_swap_index = stack_top_index.checked_sub(depth as usize).unwrap();
                    self.stack.swap(stack_top_index - 1, to_swap_index - 1);
                }
                Opcode::TLOAD => {
                    let key = self.stack.pop().unwrap();
                    let value = self.transient_storage.get(self.caller, key);

                    self.stack.push(value);
                }
                Opcode::TSTORE => {
                    let key = self.stack.pop().unwrap();
                    let value = self.stack.pop().unwrap();

                    self.transient_storage.set(self.caller, key, value);
                }
                Opcode::MLOAD => {
                    // spend_gas(3);
                    let offset = self.stack.pop().unwrap().try_into().unwrap();
                    let value = self.memory.load(offset);
                    self.stack.push(value);
                }
                Opcode::MSTORE => {
                    // spend_gas(3);
                    let offset = self.stack.pop().unwrap().try_into().unwrap();
                    let value = self.stack.pop().unwrap();
                    let mut value_bytes = [0u8; 32];
                    value.to_big_endian(&mut value_bytes);

                    self.memory.store_bytes(offset, &value_bytes);
                }
                Opcode::MSTORE8 => {
                    // spend_gas(3);
                    let offset = self.stack.pop().unwrap().try_into().unwrap();
                    let value = self.stack.pop().unwrap();
                    let mut value_bytes = [0u8; 32];
                    value.to_big_endian(&mut value_bytes);

                    self.memory
                        .store_bytes(offset, value_bytes[31..32].as_ref());
                }
                Opcode::MSIZE => {
                    // spend_gas(2);
                    self.stack.push(self.memory.size());
                }
                Opcode::MCOPY => {
                    // spend_gas(3) + dynamic gas
                    let dest_offset = self.stack.pop().unwrap().try_into().unwrap();
                    let src_offset = self.stack.pop().unwrap().try_into().unwrap();
                    let size = self.stack.pop().unwrap().try_into().unwrap();
                    if size == 0 {
                        continue;
                    }

                    self.memory.copy(src_offset, dest_offset, size);
                }
                _ => unimplemented!(),
            }
        }
    }

    fn next_opcode(&mut self, opcodes: &mut Bytes) -> Option<Opcode> {
        let opcode = opcodes.get(self.pc).copied().map(Opcode::from);
        self.increment_pc();
        opcode
    }

    fn increment_pc_by(&mut self, count: usize) {
        self.pc += count;
    }

    fn increment_pc(&mut self) {
        self.increment_pc_by(1);
    }

    pub fn pc(&self) -> usize {
        self.pc
    }
}

#[derive(Debug, Clone, Default)]
pub struct TransientStorage(HashMap<(Address, U256), U256>);

impl TransientStorage {
    /// Returns a new empty transient storage.
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    /// Get a value at a storage key for an account.
    ///
    /// Returns `U256::zero()` if there is no such value. See [implementation reference]
    /// or [EIP-1153].
    ///
    /// [implementation reference]: https://github.com/ethereum/execution-specs/blob/51fac24740e662844446439ceeb96a460aae0ba0/src/ethereum/cancun/state.py#L641
    /// [EIP-1153]: https://eips.ethereum.org/EIPS/eip-1153#reference-implementation
    pub fn get(&self, address: Address, key: U256) -> U256 {
        if let Some(value) = self.0.get(&(address, key)) {
            *value
        } else {
            U256::zero()
        }
    }

    /// Set a value at a storage key for an account.
    pub fn set(&mut self, address: Address, key: U256, value: U256) {
        self.0.insert((address, key), value);
    }
}

#[derive(Debug, Clone, Default)]
pub struct Memory {
    data: Vec<u8>,
}

impl Memory {
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }

    fn resize(&mut self, offset: usize) {
        if offset.next_multiple_of(32) > self.data.len() {
            self.data.resize(offset.next_multiple_of(32), 0);
        }
    }

    pub fn load(&mut self, offset: usize) -> U256 {
        self.resize(offset + 32);
        let value_bytes: [u8; 32] = self
            .data
            .get(offset..offset + 32)
            .unwrap()
            .try_into()
            .unwrap();
        U256::from(value_bytes)
    }

    pub fn store_bytes(&mut self, offset: usize, value: &[u8]) {
        let len = value.len();
        self.resize(offset + len);
        self.data
            .splice(offset..offset + len, value.iter().copied());
    }

    pub fn size(&self) -> U256 {
        U256::from(self.data.len())
    }

    pub fn copy(&mut self, src_offset: usize, dest_offset: usize, size: usize) {
        let max_size = std::cmp::max(src_offset + size, dest_offset + size);
        self.resize(max_size);
        let mut temp = vec![0u8; size];

        temp.copy_from_slice(&self.data[src_offset..src_offset + size]);

        self.data[dest_offset..dest_offset + size].copy_from_slice(&temp);
    }
}

#[test]
fn transient_store() {
    let mut vm = VM::default();

    assert!(vm.transient_storage.0.is_empty());

    let value = U256::from_big_endian(&[0xaa; 3]);
    let key = U256::from_big_endian(&[0xff; 2]);

    let operations = [
        Operation::Push32(value),
        Operation::Push32(key),
        Operation::Tstore,
        Operation::Stop,
    ];

    let bytecode = operations.iter().flat_map(Operation::to_bytecode).collect();

    vm.execute(bytecode);

    assert_eq!(vm.transient_storage.get(vm.caller, key), value)
}

#[test]
#[should_panic]
fn transient_store_no_values_panics() {
    let mut vm = VM::default();

    assert!(vm.transient_storage.0.is_empty());

    let operations = [Operation::Tstore, Operation::Stop];

    let bytecode = operations.iter().flat_map(Operation::to_bytecode).collect();

    vm.execute(bytecode);
}

#[test]
fn transient_load() {
    let value = U256::from_big_endian(&[0xaa; 3]);
    let key = U256::from_big_endian(&[0xff; 2]);

    let mut vm = VM::default();

    vm.transient_storage.set(vm.caller, key, value);

    let operations = [Operation::Push32(key), Operation::Tload, Operation::Stop];

    let bytecode = operations.iter().flat_map(Operation::to_bytecode).collect();

    vm.execute(bytecode);

    assert_eq!(vm.stack.pop().unwrap(), value)
}

#[test]
fn get_ok() {
    let mut tstorage = TransientStorage::new();

    tstorage
        .0
        .insert((Address::default(), U256::one()), U256::from("0xffff"));

    assert_eq!(
        tstorage.get(Address::default(), U256::one()),
        U256::from("0xffff")
    )
}

#[test]
fn unexistant_returns_zero() {
    let tsstorage = TransientStorage::new();

    assert_eq!(tsstorage.get(Address::default(), U256::one()), U256::zero())
}
