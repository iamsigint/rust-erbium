// src/core/vm.rs

use crate::core::{Address, Hash, State};
use crate::utils::error::{BlockchainError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// VM Configuration
#[derive(Debug, Clone)]
pub struct VMConfig {
    pub max_stack_depth: usize,
    pub max_memory_size: usize,
    pub max_execution_steps: usize,
    pub gas_limit: u64,
    pub base_gas_cost: u64,
}

impl Default for VMConfig {
    fn default() -> Self {
        Self {
            max_stack_depth: 1024,
            max_memory_size: 64 * 1024, // 64KB
            max_execution_steps: 1_000_000,
            gas_limit: 10_000_000,
            base_gas_cost: 1,
        }
    }
}

/// VM Opcodes (Instruction Set)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Opcode {
    // Stack operations
    PUSH1 = 0x60,
    PUSH2,
    PUSH3,
    PUSH4,
    PUSH5,
    PUSH6,
    PUSH7,
    PUSH8,
    PUSH9,
    PUSH10,
    PUSH11,
    PUSH12,
    PUSH13,
    PUSH14,
    PUSH15,
    PUSH16,
    PUSH17,
    PUSH18,
    PUSH19,
    PUSH20,
    PUSH21,
    PUSH22,
    PUSH23,
    PUSH24,
    PUSH25,
    PUSH26,
    PUSH27,
    PUSH28,
    PUSH29,
    PUSH30,
    PUSH31,
    PUSH32,
    POP = 0x50,
    DUP1 = 0x80,
    DUP2,
    DUP3,
    DUP4,
    DUP5,
    DUP6,
    DUP7,
    DUP8,
    DUP9,
    DUP10,
    DUP11,
    DUP12,
    DUP13,
    DUP14,
    DUP15,
    DUP16,
    SWAP1 = 0x90,
    SWAP2,
    SWAP3,
    SWAP4,
    SWAP5,
    SWAP6,
    SWAP7,
    SWAP8,
    SWAP9,
    SWAP10,
    SWAP11,
    SWAP12,
    SWAP13,
    SWAP14,
    SWAP15,
    SWAP16,

    // Arithmetic operations
    ADD = 0x01,
    MUL,
    SUB,
    DIV,
    SDIV,
    MOD,
    SMOD,
    ADDMOD,
    MULMOD,
    EXP,

    // Comparison operations
    LT = 0x10,
    GT,
    SLT,
    SGT,
    EQ,
    ISZERO,

    // Bitwise operations
    AND = 0x16,
    OR,
    XOR,
    NOT,
    BYTE,

    // Cryptographic operations
    SHA3 = 0x20,

    // Environmental operations
    ADDRESS = 0x30,
    BALANCE,
    ORIGIN,
    CALLER,
    CALLVALUE,
    CALLDATALOAD,
    CALLDATASIZE,
    CALLDATACOPY,
    CODESIZE,
    CODECOPY,
    GASPRICE,
    EXTCODESIZE,
    EXTCODECOPY,
    RETURNDATASIZE,
    RETURNDATACOPY,
    EXTCODEHASH,

    // Block operations
    BLOCKHASH = 0x40,
    COINBASE,
    TIMESTAMP,
    NUMBER,
    DIFFICULTY,
    GASLIMIT,
    CHAINID,

    // Memory operations
    MLOAD = 0x51,
    MSTORE,
    MSTORE8,

    // Storage operations
    SLOAD = 0x54,
    SSTORE,

    // Jump operations
    JUMP = 0x56,
    JUMPI,
    PC,
    JUMPDEST,

    // Function operations
    CALL = 0xF1,
    CALLCODE,
    DELEGATECALL,
    STATICCALL,
    RETURN,
    REVERT,
    INVALID,
    SELFDESTRUCT,

    // Logging operations
    LOG0 = 0xA0,
    LOG1,
    LOG2,
    LOG3,
    LOG4,

    // Stop and invalid
    STOP = 0x00,
}

impl Opcode {
    /// Get gas cost for opcode
    pub fn gas_cost(&self) -> u64 {
        match self {
            // Very low cost operations
            Opcode::STOP
            | Opcode::ADD
            | Opcode::SUB
            | Opcode::LT
            | Opcode::GT
            | Opcode::EQ
            | Opcode::ISZERO
            | Opcode::AND
            | Opcode::OR
            | Opcode::XOR
            | Opcode::NOT
            | Opcode::POP
            | Opcode::PC
            | Opcode::JUMPDEST => 1,

            // Low cost operations
            Opcode::MUL
            | Opcode::DIV
            | Opcode::SDIV
            | Opcode::MOD
            | Opcode::SMOD
            | Opcode::BYTE => 3,

            // Mid cost operations
            Opcode::ADDMOD | Opcode::MULMOD => 5,

            // High cost operations
            Opcode::EXP => 10,

            // Memory operations
            Opcode::MLOAD | Opcode::MSTORE => 3,
            Opcode::MSTORE8 => 3,

            // Storage operations
            Opcode::SLOAD => 100,
            Opcode::SSTORE => 5000, // High cost to discourage spam

            // Calls
            Opcode::CALL | Opcode::CALLCODE | Opcode::DELEGATECALL | Opcode::STATICCALL => 100,

            // Cryptographic operations
            Opcode::SHA3 => 30,

            // Environmental operations
            Opcode::ADDRESS
            | Opcode::BALANCE
            | Opcode::ORIGIN
            | Opcode::CALLER
            | Opcode::CALLVALUE
            | Opcode::CALLDATALOAD
            | Opcode::CALLDATASIZE
            | Opcode::CODESIZE
            | Opcode::GASPRICE
            | Opcode::EXTCODESIZE
            | Opcode::RETURNDATASIZE => 2,

            // Block operations
            Opcode::BLOCKHASH => 20,
            Opcode::COINBASE
            | Opcode::TIMESTAMP
            | Opcode::NUMBER
            | Opcode::DIFFICULTY
            | Opcode::GASLIMIT
            | Opcode::CHAINID => 2,

            // Jumps
            Opcode::JUMP | Opcode::JUMPI => 8,

            // Stack operations
            Opcode::PUSH1
            | Opcode::PUSH2
            | Opcode::PUSH3
            | Opcode::PUSH4
            | Opcode::PUSH5
            | Opcode::PUSH6
            | Opcode::PUSH7
            | Opcode::PUSH8
            | Opcode::PUSH9
            | Opcode::PUSH10
            | Opcode::PUSH11
            | Opcode::PUSH12
            | Opcode::PUSH13
            | Opcode::PUSH14
            | Opcode::PUSH15
            | Opcode::PUSH16
            | Opcode::PUSH17
            | Opcode::PUSH18
            | Opcode::PUSH19
            | Opcode::PUSH20
            | Opcode::PUSH21
            | Opcode::PUSH22
            | Opcode::PUSH23
            | Opcode::PUSH24
            | Opcode::PUSH25
            | Opcode::PUSH26
            | Opcode::PUSH27
            | Opcode::PUSH28
            | Opcode::PUSH29
            | Opcode::PUSH30
            | Opcode::PUSH31
            | Opcode::PUSH32 => 3,

            Opcode::DUP1
            | Opcode::DUP2
            | Opcode::DUP3
            | Opcode::DUP4
            | Opcode::DUP5
            | Opcode::DUP6
            | Opcode::DUP7
            | Opcode::DUP8
            | Opcode::DUP9
            | Opcode::DUP10
            | Opcode::DUP11
            | Opcode::DUP12
            | Opcode::DUP13
            | Opcode::DUP14
            | Opcode::DUP15
            | Opcode::DUP16 => 3,

            Opcode::SWAP1
            | Opcode::SWAP2
            | Opcode::SWAP3
            | Opcode::SWAP4
            | Opcode::SWAP5
            | Opcode::SWAP6
            | Opcode::SWAP7
            | Opcode::SWAP8
            | Opcode::SWAP9
            | Opcode::SWAP10
            | Opcode::SWAP11
            | Opcode::SWAP12
            | Opcode::SWAP13
            | Opcode::SWAP14
            | Opcode::SWAP15
            | Opcode::SWAP16 => 3,

            // Logging
            Opcode::LOG0 | Opcode::LOG1 | Opcode::LOG2 | Opcode::LOG3 | Opcode::LOG4 => 375,

            // Others
            _ => 1,
        }
    }
}

/// VM Stack Value
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct StackValue(pub [u8; 32]);

impl StackValue {
    /// Create from u64
    pub fn from_u64(value: u64) -> Self {
        let mut bytes = [0u8; 32];
        bytes[24..].copy_from_slice(&value.to_be_bytes());
        StackValue(bytes)
    }

    /// Convert to u64
    pub fn to_u64(&self) -> u64 {
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&self.0[24..]);
        u64::from_be_bytes(bytes)
    }

    /// Create from bytes
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let mut value = [0u8; 32];
        let len = bytes.len().min(32);
        value[32 - len..].copy_from_slice(&bytes[..len]);
        StackValue(value)
    }

    /// Get as bytes
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Zero value
    pub fn zero() -> Self {
        StackValue([0u8; 32])
    }
}

impl Default for StackValue {
    fn default() -> Self {
        StackValue::zero()
    }
}

/// VM Memory
#[derive(Debug, Clone)]
pub struct Memory {
    data: Vec<u8>,
    max_size: usize,
}

impl Memory {
    /// Create new memory
    pub fn new(max_size: usize) -> Self {
        Self {
            data: Vec::new(),
            max_size,
        }
    }

    /// Read 32 bytes from memory
    pub fn read(&mut self, offset: usize) -> Result<StackValue> {
        if offset + 32 > self.max_size {
            return Err(BlockchainError::VM(
                "Memory access out of bounds".to_string(),
            ));
        }

        if offset >= self.data.len() {
            return Ok(StackValue::zero());
        }

        let end = (offset + 32).min(self.data.len());
        let mut bytes = [0u8; 32];
        bytes[..end - offset].copy_from_slice(&self.data[offset..end]);
        Ok(StackValue(bytes))
    }

    /// Write 32 bytes to memory
    pub fn write(&mut self, offset: usize, value: StackValue) -> Result<()> {
        if offset + 32 > self.max_size {
            return Err(BlockchainError::VM(
                "Memory access out of bounds".to_string(),
            ));
        }

        if offset + 32 > self.data.len() {
            self.data.resize(offset + 32, 0);
        }

        self.data[offset..offset + 32].copy_from_slice(&value.0);
        Ok(())
    }

    /// Copy data to memory
    pub fn copy(&mut self, dest_offset: usize, src: &[u8]) -> Result<()> {
        if dest_offset + src.len() > self.max_size {
            return Err(BlockchainError::VM(
                "Memory access out of bounds".to_string(),
            ));
        }

        if dest_offset + src.len() > self.data.len() {
            self.data.resize(dest_offset + src.len(), 0);
        }

        self.data[dest_offset..dest_offset + src.len()].copy_from_slice(src);
        Ok(())
    }

    /// Get memory size
    pub fn size(&self) -> usize {
        self.data.len()
    }
}

/// Contract Storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractStorage {
    data: HashMap<StackValue, StackValue>,
}

impl ContractStorage {
    /// Create new storage
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    /// Load from storage
    pub fn load(&self, key: StackValue) -> StackValue {
        self.data.get(&key).copied().unwrap_or(StackValue::zero())
    }

    /// Store to storage
    pub fn store(&mut self, key: StackValue, value: StackValue) {
        if value == StackValue::zero() {
            self.data.remove(&key);
        } else {
            self.data.insert(key, value);
        }
    }

    /// Get all storage keys
    pub fn keys(&self) -> Vec<StackValue> {
        self.data.keys().copied().collect()
    }
}

/// VM Execution Context
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    pub address: Address,     // Contract address
    pub caller: Address,      // Caller address
    pub origin: Address,      // Original transaction sender
    pub value: u64,           // Value sent with call
    pub data: Vec<u8>,        // Call data
    pub gas_price: u64,       // Gas price
    pub gas_limit: u64,       // Gas limit
    pub block_number: u64,    // Current block number
    pub block_timestamp: u64, // Current block timestamp
    pub chain_id: u64,        // Chain ID
}

impl Default for ExecutionContext {
    fn default() -> Self {
        Self {
            address: Address::new_unchecked(
                "0x0000000000000000000000000000000000000000".to_string(),
            ),
            caller: Address::new_unchecked(
                "0x0000000000000000000000000000000000000000".to_string(),
            ),
            origin: Address::new_unchecked(
                "0x0000000000000000000000000000000000000000".to_string(),
            ),
            value: 0,
            data: Vec::new(),
            gas_price: 1,
            gas_limit: 10_000_000,
            block_number: 0,
            block_timestamp: 0,
            chain_id: 137,
        }
    }
}

/// VM Execution Result
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub success: bool,
    pub return_data: Vec<u8>,
    pub gas_used: u64,
    pub logs: Vec<Log>,
    pub gas_refunded: u64,
}

impl ExecutionResult {
    /// Successful execution
    pub fn success(return_data: Vec<u8>, gas_used: u64, logs: Vec<Log>) -> Self {
        Self {
            success: true,
            return_data,
            gas_used,
            logs,
            gas_refunded: 0,
        }
    }

    /// Failed execution
    pub fn failure(gas_used: u64) -> Self {
        Self {
            success: false,
            return_data: Vec::new(),
            gas_used,
            logs: Vec::new(),
            gas_refunded: 0,
        }
    }
}

/// Event Log
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Log {
    pub address: Address,
    pub topics: Vec<Hash>,
    pub data: Vec<u8>,
}

impl Log {
    /// Create new log
    pub fn new(address: Address, topics: Vec<Hash>, data: Vec<u8>) -> Self {
        Self {
            address,
            topics,
            data,
        }
    }
}

/// Smart Contract
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contract {
    pub address: Address,
    pub bytecode: Vec<u8>,
    pub storage: ContractStorage,
    pub balance: u64,
    pub nonce: u64,
}

impl Contract {
    /// Create new contract
    pub fn new(address: Address, bytecode: Vec<u8>) -> Self {
        Self {
            address,
            bytecode,
            storage: ContractStorage::new(),
            balance: 0,
            nonce: 0,
        }
    }

    /// Get contract code hash
    pub fn code_hash(&self) -> Hash {
        Hash::new(&self.bytecode)
    }
}

/// Virtual Machine
pub struct VirtualMachine {
    config: VMConfig,
    state: Arc<RwLock<State>>,
}

impl VirtualMachine {
    /// Create new VM
    pub fn new(config: VMConfig, state: Arc<RwLock<State>>) -> Self {
        Self { config, state }
    }

    /// Execute contract bytecode
    pub async fn execute(
        &self,
        contract: &mut Contract,
        context: &ExecutionContext,
    ) -> Result<ExecutionResult> {
        let mut stack = Vec::new();
        let mut memory = Memory::new(self.config.max_memory_size);
        let mut pc = 0; // Program counter
        let mut gas_used = 0u64;
        let mut logs = Vec::new();

        while pc < contract.bytecode.len() && gas_used < context.gas_limit {
            let opcode = contract.bytecode[pc];
            pc += 1;

            // Check gas
            let opcode_enum = self.byte_to_opcode(opcode)?;
            let gas_cost = opcode_enum.gas_cost();
            if gas_used + gas_cost > context.gas_limit {
                return Ok(ExecutionResult::failure(gas_used));
            }
            gas_used += gas_cost;

            // Execute instruction
            match opcode_enum {
                Opcode::STOP => break,
                Opcode::ADD => self.execute_add(&mut stack)?,
                Opcode::MUL => self.execute_mul(&mut stack)?,
                Opcode::SUB => self.execute_sub(&mut stack)?,
                Opcode::DIV => self.execute_div(&mut stack)?,
                Opcode::MOD => self.execute_mod(&mut stack)?,
                Opcode::POP => self.execute_pop(&mut stack)?,
                // PUSH operations
                Opcode::PUSH1
                | Opcode::PUSH2
                | Opcode::PUSH3
                | Opcode::PUSH4
                | Opcode::PUSH5
                | Opcode::PUSH6
                | Opcode::PUSH7
                | Opcode::PUSH8
                | Opcode::PUSH9
                | Opcode::PUSH10
                | Opcode::PUSH11
                | Opcode::PUSH12
                | Opcode::PUSH13
                | Opcode::PUSH14
                | Opcode::PUSH15
                | Opcode::PUSH16
                | Opcode::PUSH17
                | Opcode::PUSH18
                | Opcode::PUSH19
                | Opcode::PUSH20
                | Opcode::PUSH21
                | Opcode::PUSH22
                | Opcode::PUSH23
                | Opcode::PUSH24
                | Opcode::PUSH25
                | Opcode::PUSH26
                | Opcode::PUSH27
                | Opcode::PUSH28
                | Opcode::PUSH29
                | Opcode::PUSH30
                | Opcode::PUSH31
                | Opcode::PUSH32 => {
                    let size = (opcode - Opcode::PUSH1 as u8 + 1) as usize;
                    self.execute_push(&mut stack, &contract.bytecode, &mut pc, size)?;
                }
                // DUP operations
                Opcode::DUP1
                | Opcode::DUP2
                | Opcode::DUP3
                | Opcode::DUP4
                | Opcode::DUP5
                | Opcode::DUP6
                | Opcode::DUP7
                | Opcode::DUP8
                | Opcode::DUP9
                | Opcode::DUP10
                | Opcode::DUP11
                | Opcode::DUP12
                | Opcode::DUP13
                | Opcode::DUP14
                | Opcode::DUP15
                | Opcode::DUP16 => {
                    let depth = (opcode - Opcode::DUP1 as u8 + 1) as usize;
                    self.execute_dup(&mut stack, depth)?;
                }
                // SWAP operations
                Opcode::SWAP1
                | Opcode::SWAP2
                | Opcode::SWAP3
                | Opcode::SWAP4
                | Opcode::SWAP5
                | Opcode::SWAP6
                | Opcode::SWAP7
                | Opcode::SWAP8
                | Opcode::SWAP9
                | Opcode::SWAP10
                | Opcode::SWAP11
                | Opcode::SWAP12
                | Opcode::SWAP13
                | Opcode::SWAP14
                | Opcode::SWAP15
                | Opcode::SWAP16 => {
                    let depth = (opcode - Opcode::SWAP1 as u8 + 1) as usize;
                    self.execute_swap(&mut stack, depth)?;
                }
                Opcode::MLOAD => self.execute_mload(&mut stack, &mut memory)?,
                Opcode::MSTORE => self.execute_mstore(&mut stack, &mut memory)?,
                Opcode::SLOAD => self.execute_sload(&mut stack, &contract.storage)?,
                Opcode::SSTORE => self.execute_sstore(&mut stack, &mut contract.storage)?,
                Opcode::JUMP => self.execute_jump(&mut stack, &mut pc, &contract.bytecode)?,
                Opcode::JUMPI => self.execute_jumpi(&mut stack, &mut pc, &contract.bytecode)?,
                Opcode::PC => self.execute_pc(&mut stack, pc)?,
                Opcode::JUMPDEST => {} // No operation
                Opcode::ADDRESS => self.execute_address(&mut stack, &context.address)?,
                Opcode::BALANCE => {
                    self.execute_balance(&mut stack, &context.address, &self.state)
                        .await?
                }
                Opcode::CALLER => self.execute_caller(&mut stack, &context.caller)?,
                Opcode::CALLVALUE => self.execute_callvalue(&mut stack, context.value)?,
                Opcode::CALLDATALOAD => self.execute_calldataload(&mut stack, &context.data)?,
                Opcode::CALLDATASIZE => self.execute_calldatasize(&mut stack, &context.data)?,
                Opcode::CODESIZE => self.execute_codesize(&mut stack, &contract.bytecode)?,
                Opcode::GASPRICE => self.execute_gasprice(&mut stack, context.gas_price)?,
                Opcode::TIMESTAMP => self.execute_timestamp(&mut stack, context.block_timestamp)?,
                Opcode::NUMBER => self.execute_number(&mut stack, context.block_number)?,
                Opcode::CHAINID => self.execute_chainid(&mut stack, context.chain_id)?,
                // LOG operations
                Opcode::LOG0 | Opcode::LOG1 | Opcode::LOG2 | Opcode::LOG3 | Opcode::LOG4 => {
                    let topics_count = (opcode - Opcode::LOG0 as u8) as usize;
                    self.execute_log(
                        &mut stack,
                        &mut memory,
                        &context.address,
                        topics_count,
                        &mut logs,
                    )?;
                }
                Opcode::RETURN => {
                    let result = self.execute_return(&mut stack, &memory)?;
                    return Ok(ExecutionResult::success(result, gas_used, logs));
                }
                Opcode::REVERT => {
                    return Ok(ExecutionResult::failure(gas_used));
                }
                _ => {
                    log::warn!("Unsupported opcode: 0x{:02x}", opcode);
                    return Ok(ExecutionResult::failure(gas_used));
                }
            }

            // Check stack limits
            if stack.len() > self.config.max_stack_depth {
                return Ok(ExecutionResult::failure(gas_used));
            }
        }

        Ok(ExecutionResult::success(Vec::new(), gas_used, logs))
    }

    /// Convert byte to opcode
    fn byte_to_opcode(&self, byte: u8) -> Result<Opcode> {
        match byte {
            0x00 => Ok(Opcode::STOP),
            0x01 => Ok(Opcode::ADD),
            0x02 => Ok(Opcode::MUL),
            0x03 => Ok(Opcode::SUB),
            0x04 => Ok(Opcode::DIV),
            0x06 => Ok(Opcode::MOD),
            0x10 => Ok(Opcode::LT),
            0x11 => Ok(Opcode::GT),
            0x14 => Ok(Opcode::EQ),
            0x15 => Ok(Opcode::ISZERO),
            0x16 => Ok(Opcode::AND),
            0x17 => Ok(Opcode::OR),
            0x18 => Ok(Opcode::XOR),
            0x19 => Ok(Opcode::NOT),
            0x20 => Ok(Opcode::SHA3),
            0x30 => Ok(Opcode::ADDRESS),
            0x31 => Ok(Opcode::BALANCE),
            0x32 => Ok(Opcode::ORIGIN),
            0x33 => Ok(Opcode::CALLER),
            0x34 => Ok(Opcode::CALLVALUE),
            0x35 => Ok(Opcode::CALLDATALOAD),
            0x36 => Ok(Opcode::CALLDATASIZE),
            0x38 => Ok(Opcode::CODESIZE),
            0x3A => Ok(Opcode::GASPRICE),
            0x40 => Ok(Opcode::BLOCKHASH),
            0x41 => Ok(Opcode::COINBASE),
            0x42 => Ok(Opcode::TIMESTAMP),
            0x43 => Ok(Opcode::NUMBER),
            0x44 => Ok(Opcode::DIFFICULTY),
            0x45 => Ok(Opcode::GASLIMIT),
            0x46 => Ok(Opcode::CHAINID),
            0x50 => Ok(Opcode::POP),
            0x51 => Ok(Opcode::MLOAD),
            0x52 => Ok(Opcode::MSTORE),
            0x54 => Ok(Opcode::SLOAD),
            0x55 => Ok(Opcode::SSTORE),
            0x56 => Ok(Opcode::JUMP),
            0x57 => Ok(Opcode::JUMPI),
            0x58 => Ok(Opcode::PC),
            0x5B => Ok(Opcode::JUMPDEST),
            0x60..=0x7F => Ok(Opcode::PUSH1), // PUSH1-PUSH32
            0x80..=0x8F => Ok(Opcode::DUP1),  // DUP1-DUP16
            0x90..=0x9F => Ok(Opcode::SWAP1), // SWAP1-SWAP16
            0xA0..=0xA4 => Ok(Opcode::LOG0),  // LOG0-LOG4
            0xF1 => Ok(Opcode::CALL),
            0xF3 => Ok(Opcode::RETURN),
            0xFD => Ok(Opcode::REVERT),
            0xFF => Ok(Opcode::SELFDESTRUCT),
            _ => Err(BlockchainError::VM(format!(
                "Unknown opcode: 0x{:02x}",
                byte
            ))),
        }
    }

    // Stack operations
    fn execute_add(&self, stack: &mut Vec<StackValue>) -> Result<()> {
        let a = stack
            .pop()
            .ok_or_else(|| BlockchainError::VM("Stack underflow".to_string()))?;
        let b = stack
            .pop()
            .ok_or_else(|| BlockchainError::VM("Stack underflow".to_string()))?;
        let result = StackValue::from_u64(a.to_u64().wrapping_add(b.to_u64()));
        stack.push(result);
        Ok(())
    }

    fn execute_mul(&self, stack: &mut Vec<StackValue>) -> Result<()> {
        let a = stack
            .pop()
            .ok_or_else(|| BlockchainError::VM("Stack underflow".to_string()))?;
        let b = stack
            .pop()
            .ok_or_else(|| BlockchainError::VM("Stack underflow".to_string()))?;
        let result = StackValue::from_u64(a.to_u64().wrapping_mul(b.to_u64()));
        stack.push(result);
        Ok(())
    }

    fn execute_sub(&self, stack: &mut Vec<StackValue>) -> Result<()> {
        let a = stack
            .pop()
            .ok_or_else(|| BlockchainError::VM("Stack underflow".to_string()))?;
        let b = stack
            .pop()
            .ok_or_else(|| BlockchainError::VM("Stack underflow".to_string()))?;
        let result = StackValue::from_u64(a.to_u64().wrapping_sub(b.to_u64()));
        stack.push(result);
        Ok(())
    }

    fn execute_div(&self, stack: &mut Vec<StackValue>) -> Result<()> {
        let a = stack
            .pop()
            .ok_or_else(|| BlockchainError::VM("Stack underflow".to_string()))?;
        let b = stack
            .pop()
            .ok_or_else(|| BlockchainError::VM("Stack underflow".to_string()))?;
        let divisor = b.to_u64();
        let result = if divisor == 0 {
            StackValue::zero()
        } else {
            StackValue::from_u64(a.to_u64() / divisor)
        };
        stack.push(result);
        Ok(())
    }

    fn execute_mod(&self, stack: &mut Vec<StackValue>) -> Result<()> {
        let a = stack
            .pop()
            .ok_or_else(|| BlockchainError::VM("Stack underflow".to_string()))?;
        let b = stack
            .pop()
            .ok_or_else(|| BlockchainError::VM("Stack underflow".to_string()))?;
        let divisor = b.to_u64();
        let result = if divisor == 0 {
            StackValue::zero()
        } else {
            StackValue::from_u64(a.to_u64() % divisor)
        };
        stack.push(result);
        Ok(())
    }

    fn execute_pop(&self, stack: &mut Vec<StackValue>) -> Result<()> {
        stack
            .pop()
            .ok_or_else(|| BlockchainError::VM("Stack underflow".to_string()))?;
        Ok(())
    }

    fn execute_push(
        &self,
        stack: &mut Vec<StackValue>,
        bytecode: &[u8],
        pc: &mut usize,
        size: usize,
    ) -> Result<()> {
        if *pc + size > bytecode.len() {
            return Err(BlockchainError::VM("Push data out of bounds".to_string()));
        }
        let data = &bytecode[*pc..*pc + size];
        *pc += size;
        stack.push(StackValue::from_bytes(data));
        Ok(())
    }

    fn execute_dup(&self, stack: &mut Vec<StackValue>, depth: usize) -> Result<()> {
        if stack.len() < depth {
            return Err(BlockchainError::VM("Stack underflow".to_string()));
        }
        let value = stack[stack.len() - depth];
        stack.push(value);
        Ok(())
    }

    fn execute_swap(&self, stack: &mut Vec<StackValue>, depth: usize) -> Result<()> {
        if stack.len() < depth + 1 {
            return Err(BlockchainError::VM("Stack underflow".to_string()));
        }
        let len = stack.len();
        stack.swap(len - 1, len - 1 - depth);
        Ok(())
    }

    // Memory operations
    fn execute_mload(&self, stack: &mut Vec<StackValue>, memory: &mut Memory) -> Result<()> {
        let offset = stack
            .pop()
            .ok_or_else(|| BlockchainError::VM("Stack underflow".to_string()))?;
        let value = memory.read(offset.to_u64() as usize)?;
        stack.push(value);
        Ok(())
    }

    fn execute_mstore(&self, stack: &mut Vec<StackValue>, memory: &mut Memory) -> Result<()> {
        let offset = stack
            .pop()
            .ok_or_else(|| BlockchainError::VM("Stack underflow".to_string()))?;
        let value = stack
            .pop()
            .ok_or_else(|| BlockchainError::VM("Stack underflow".to_string()))?;
        memory.write(offset.to_u64() as usize, value)?;
        Ok(())
    }

    // Storage operations
    fn execute_sload(&self, stack: &mut Vec<StackValue>, storage: &ContractStorage) -> Result<()> {
        let key = stack
            .pop()
            .ok_or_else(|| BlockchainError::VM("Stack underflow".to_string()))?;
        let value = storage.load(key);
        stack.push(value);
        Ok(())
    }

    fn execute_sstore(
        &self,
        stack: &mut Vec<StackValue>,
        storage: &mut ContractStorage,
    ) -> Result<()> {
        let key = stack
            .pop()
            .ok_or_else(|| BlockchainError::VM("Stack underflow".to_string()))?;
        let value = stack
            .pop()
            .ok_or_else(|| BlockchainError::VM("Stack underflow".to_string()))?;
        storage.store(key, value);
        Ok(())
    }

    // Control flow
    fn execute_jump(
        &self,
        stack: &mut Vec<StackValue>,
        pc: &mut usize,
        bytecode: &[u8],
    ) -> Result<()> {
        let destination = stack
            .pop()
            .ok_or_else(|| BlockchainError::VM("Stack underflow".to_string()))?;
        let dest_pc = destination.to_u64() as usize;

        if dest_pc >= bytecode.len() || bytecode[dest_pc] != Opcode::JUMPDEST as u8 {
            return Err(BlockchainError::VM("Invalid jump destination".to_string()));
        }

        *pc = dest_pc;
        Ok(())
    }

    fn execute_jumpi(
        &self,
        stack: &mut Vec<StackValue>,
        pc: &mut usize,
        bytecode: &[u8],
    ) -> Result<()> {
        let destination = stack
            .pop()
            .ok_or_else(|| BlockchainError::VM("Stack underflow".to_string()))?;
        let condition = stack
            .pop()
            .ok_or_else(|| BlockchainError::VM("Stack underflow".to_string()))?;

        if condition.to_u64() != 0 {
            let dest_pc = destination.to_u64() as usize;
            if dest_pc >= bytecode.len() || bytecode[dest_pc] != Opcode::JUMPDEST as u8 {
                return Err(BlockchainError::VM("Invalid jump destination".to_string()));
            }
            *pc = dest_pc;
        }
        Ok(())
    }

    fn execute_pc(&self, stack: &mut Vec<StackValue>, pc: usize) -> Result<()> {
        stack.push(StackValue::from_u64(pc as u64));
        Ok(())
    }

    // Environmental operations
    fn execute_address(&self, stack: &mut Vec<StackValue>, address: &Address) -> Result<()> {
        let addr_bytes = address.to_bytes().unwrap_or_default();
        stack.push(StackValue::from_bytes(&addr_bytes));
        Ok(())
    }

    async fn execute_balance(
        &self,
        stack: &mut Vec<StackValue>,
        address: &Address,
        state: &Arc<RwLock<State>>,
    ) -> Result<()> {
        let state_read = state.read().await;
        let balance = state_read.get_balance(address).unwrap_or(0);
        stack.push(StackValue::from_u64(balance));
        Ok(())
    }

    fn execute_caller(&self, stack: &mut Vec<StackValue>, caller: &Address) -> Result<()> {
        let caller_bytes = caller.to_bytes().unwrap_or_default();
        stack.push(StackValue::from_bytes(&caller_bytes));
        Ok(())
    }

    fn execute_callvalue(&self, stack: &mut Vec<StackValue>, value: u64) -> Result<()> {
        stack.push(StackValue::from_u64(value));
        Ok(())
    }

    fn execute_calldataload(&self, stack: &mut Vec<StackValue>, data: &[u8]) -> Result<()> {
        let offset = stack
            .pop()
            .ok_or_else(|| BlockchainError::VM("Stack underflow".to_string()))?;
        let offset_usize = offset.to_u64() as usize;

        let mut bytes = [0u8; 32];
        if offset_usize < data.len() {
            let end = (offset_usize + 32).min(data.len());
            bytes[..end - offset_usize].copy_from_slice(&data[offset_usize..end]);
        }
        stack.push(StackValue(bytes));
        Ok(())
    }

    fn execute_calldatasize(&self, stack: &mut Vec<StackValue>, data: &[u8]) -> Result<()> {
        stack.push(StackValue::from_u64(data.len() as u64));
        Ok(())
    }

    fn execute_codesize(&self, stack: &mut Vec<StackValue>, bytecode: &[u8]) -> Result<()> {
        stack.push(StackValue::from_u64(bytecode.len() as u64));
        Ok(())
    }

    fn execute_gasprice(&self, stack: &mut Vec<StackValue>, gas_price: u64) -> Result<()> {
        stack.push(StackValue::from_u64(gas_price));
        Ok(())
    }

    fn execute_timestamp(&self, stack: &mut Vec<StackValue>, timestamp: u64) -> Result<()> {
        stack.push(StackValue::from_u64(timestamp));
        Ok(())
    }

    fn execute_number(&self, stack: &mut Vec<StackValue>, block_number: u64) -> Result<()> {
        stack.push(StackValue::from_u64(block_number));
        Ok(())
    }

    fn execute_chainid(&self, stack: &mut Vec<StackValue>, chain_id: u64) -> Result<()> {
        stack.push(StackValue::from_u64(chain_id));
        Ok(())
    }

    // Logging
    fn execute_log(
        &self,
        stack: &mut Vec<StackValue>,
        memory: &mut Memory,
        address: &Address,
        topics_count: usize,
        logs: &mut Vec<Log>,
    ) -> Result<()> {
        let offset = stack
            .pop()
            .ok_or_else(|| BlockchainError::VM("Stack underflow".to_string()))?;
        let size = stack
            .pop()
            .ok_or_else(|| BlockchainError::VM("Stack underflow".to_string()))?;

        let offset_usize = offset.to_u64() as usize;
        let size_usize = size.to_u64() as usize;

        if offset_usize + size_usize > memory.size() {
            return Err(BlockchainError::VM("Log data out of bounds".to_string()));
        }

        let mut topics = Vec::new();
        for _ in 0..topics_count {
            let topic = stack
                .pop()
                .ok_or_else(|| BlockchainError::VM("Stack underflow".to_string()))?;
            topics.push(Hash::from_bytes(*topic.as_bytes()));
        }

        let data = memory.data[offset_usize..offset_usize + size_usize].to_vec();

        logs.push(Log::new(address.clone(), topics, data));
        Ok(())
    }

    // Return
    fn execute_return(&self, stack: &mut Vec<StackValue>, memory: &Memory) -> Result<Vec<u8>> {
        let offset = stack
            .pop()
            .ok_or_else(|| BlockchainError::VM("Stack underflow".to_string()))?;
        let size = stack
            .pop()
            .ok_or_else(|| BlockchainError::VM("Stack underflow".to_string()))?;

        let offset_usize = offset.to_u64() as usize;
        let size_usize = size.to_u64() as usize;

        if offset_usize + size_usize > memory.size() {
            return Err(BlockchainError::VM("Return data out of bounds".to_string()));
        }

        Ok(memory.data[offset_usize..offset_usize + size_usize].to_vec())
    }
}

/// Contract Manager
pub struct ContractManager {
    vm: VirtualMachine,
    contracts: HashMap<Address, Contract>,
}

impl ContractManager {
    /// Create new contract manager
    pub fn new(vm: VirtualMachine) -> Self {
        Self {
            vm,
            contracts: HashMap::new(),
        }
    }

    /// Deploy contract
    pub async fn deploy_contract(
        &mut self,
        bytecode: Vec<u8>,
        deployer: Address,
        context: &ExecutionContext,
    ) -> Result<Address> {
        // Generate contract address (simple scheme for now)
        let nonce = self.contracts.len() as u64;
        let deployer_bytes = deployer.to_bytes().unwrap_or_default();
        let mut address_data = deployer_bytes.to_vec();
        address_data.extend_from_slice(&nonce.to_be_bytes());
        let contract_address = Address::from_bytes(&Hash::new(&address_data).as_bytes()[..20]);

        let mut contract = Contract::new(contract_address.clone(), bytecode);

        // Execute constructor if present
        let result = self.vm.execute(&mut contract, context).await?;

        if result.success {
            self.contracts.insert(contract_address.clone(), contract);
            log::info!("Deployed contract at {}", contract_address.as_str());
            Ok(contract_address)
        } else {
            Err(BlockchainError::VM(
                "Contract deployment failed".to_string(),
            ))
        }
    }

    /// Call contract
    pub async fn call_contract(
        &mut self,
        address: &Address,
        context: &ExecutionContext,
    ) -> Result<ExecutionResult> {
        let contract = self
            .contracts
            .get_mut(address)
            .ok_or_else(|| BlockchainError::VM("Contract not found".to_string()))?;

        self.vm.execute(contract, context).await
    }

    /// Get contract
    pub fn get_contract(&self, address: &Address) -> Option<&Contract> {
        self.contracts.get(address)
    }

    /// Get all contracts
    pub fn get_all_contracts(&self) -> Vec<&Contract> {
        self.contracts.values().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    #[tokio::test]
    async fn test_contract_deployment() {
        let state = Arc::new(RwLock::new(State::new()));
        let vm_config = VMConfig::default();
        let vm = VirtualMachine::new(vm_config, state);
        let mut manager = ContractManager::new(vm);

        // Simple contract bytecode: PUSH1 0x42, PUSH1 0x00, MSTORE, PUSH1 0x20, PUSH1 0x00, RETURN
        let bytecode = vec![
            0x60, 0x42, // PUSH1 0x42
            0x60, 0x00, // PUSH1 0x00
            0x52, // MSTORE
            0x60, 0x20, // PUSH1 0x20
            0x60, 0x00, // PUSH1 0x00
            0xF3, // RETURN
        ];

        let deployer =
            Address::new_unchecked("0x0000000000000000000000000000000000000001".to_string());
        let context = ExecutionContext {
            caller: deployer.clone(),
            origin: deployer.clone(),
            ..Default::default()
        };

        let contract_address = manager
            .deploy_contract(bytecode, deployer, &context)
            .await
            .unwrap();

        let contract = manager.get_contract(&contract_address).unwrap();
        assert_eq!(contract.bytecode.len(), 10);
    }

    #[tokio::test]
    async fn test_simple_arithmetic() {
        let state = Arc::new(RwLock::new(State::new()));
        let vm_config = VMConfig::default();
        let vm = VirtualMachine::new(vm_config, state);
        let mut manager = ContractManager::new(vm);

        // Contract: PUSH1 0x05, PUSH1 0x03, ADD, PUSH1 0x00, MSTORE, PUSH1 0x20, PUSH1 0x00, RETURN
        let bytecode = vec![
            0x60, 0x05, // PUSH1 5
            0x60, 0x03, // PUSH1 3
            0x01, // ADD
            0x60, 0x00, // PUSH1 0
            0x52, // MSTORE
            0x60, 0x20, // PUSH1 32
            0x60, 0x00, // PUSH1 0
            0xF3, // RETURN
        ];

        let deployer =
            Address::new_unchecked("0x0000000000000000000000000000000000000001".to_string());
        let context = ExecutionContext {
            caller: deployer.clone(),
            origin: deployer.clone(),
            ..Default::default()
        };

        let contract_address = manager
            .deploy_contract(bytecode, deployer.clone(), &context)
            .await
            .unwrap();

        // Call contract
        let call_context = ExecutionContext {
            address: contract_address.clone(),
            caller: deployer.clone(),
            origin: deployer.clone(),
            ..Default::default()
        };

        let result = manager
            .call_contract(&contract_address, &call_context)
            .await
            .unwrap();
        assert!(result.success);

        // Result should be 8 (5 + 3)
        assert_eq!(result.return_data.len(), 32);
        let result_value = u64::from_be_bytes(result.return_data[24..].try_into().unwrap());
        assert_eq!(result_value, 8);
    }

    #[tokio::test]
    async fn test_stack_operations() {
        let state = Arc::new(RwLock::new(State::new()));
        let vm_config = VMConfig::default();
        let vm = VirtualMachine::new(vm_config, state);

        let mut contract = Contract::new(
            Address::new_unchecked("0x0000000000000000000000000000000000000001".to_string()),
            vec![0x60, 0x42, 0x60, 0x00, 0x52, 0x60, 0x20, 0x60, 0x00, 0xF3], // PUSH1 0x42, PUSH1 0x00, MSTORE, PUSH1 0x20, PUSH1 0x00, RETURN
        );

        let context = ExecutionContext::default();
        let result = vm.execute(&mut contract, &context).await.unwrap();

        assert!(result.success);
        assert_eq!(result.return_data.len(), 32);
    }
}
