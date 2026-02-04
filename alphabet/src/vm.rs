//! Interface to Alphabet's virtual machine.

<<<<<<< Updated upstream:alphabet/src/vm.rs
use std::collections::HashMap;
=======
use std::{array, io::Read};
>>>>>>> Stashed changes:src/vm.rs

use crate::is::{Instruction, is_op_r_type, op::*};

/// How many bytes are in each memory block.
pub const BLOCK_SIZE: usize = 1 << 16;

/// How many blocks are in the machine's memory.
pub const BLOCK_COUNT: usize = 1 << 16;

/// How many general-purpose registers
/// are supported by the system.
pub const REGISTER_COUNT: usize = 32;

/// The maximum word-based address the
/// system supports.
pub const MAX_WORD_ADDRESS: u32 = (1 << 30) - 1;

/// Byte contents of a memory block.
<<<<<<< Updated upstream:alphabet/src/vm.rs
type MemoryBlockBytes = [u8; BLOCK_SIZE];

/// Flags marking dirty bits on byte
/// locations in a memory block.
type MemoryBlockFlags = [bool; BLOCK_SIZE];
=======
pub type BlockBytes = [u8; BLOCK_SIZE];
>>>>>>> Stashed changes:src/vm.rs

/// A minimal virtual I/O device controller.
///
/// A virtual I/O controller must handle the behavior of
/// all reads and writes to addresses within its block.
pub trait IoController {
    /// Read a byte of data from the controller.
    fn read_byte(&self, offset: u16) -> u8;

    /// Notify the I/O device that it may update state.
    fn tick(&mut self);

    /// Write a byte of data to the controller.
    fn write_byte(&mut self, offset: u16, byte: u8);
}

/// A block of byte-addressed virtual machine memory.
pub enum Block {
    /// Block is unassigned.
    Empty,

    /// Readable and writable memory.
    Memory(Box<BlockBytes>),

    /// A memory-mapped I/O device.
    Io(Box<dyn IoController>),
}

impl Block {
    /// Create a new memory block containing the specified data.
    pub fn with_data(data: BlockBytes) -> Self {
        Self::Memory(Box::new(data))
    }

    /// Create a new zeroed memory block.
    pub fn new_memory() -> Self {
        Self::with_data([0; BLOCK_SIZE])
    }

    /// Create a new block mapped to an I/O controller.
    pub fn new_controller(controller: impl IoController + 'static) -> Self {
        Self::Io(Box::new(controller))
    }

    /// Whether the block is empty.
    pub fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }

    /// Whether the block is readable/writeable memory.
    pub fn is_memory(&self) -> bool {
        matches!(self, Self::Memory(_))
    }

    /// Whether the block is I/O-mapped.
    pub fn is_io(&self) -> bool {
        matches!(self, Self::Io(_))
    }

    /// If this is an I/O block, notify
    /// the controller it may update state.
    pub fn tick(&mut self) -> bool {
        if let Self::Io(controller) = self {
            controller.tick();
            true
        } else {
            false
        }
    }

    /// Read a byte of data from the block.
    pub fn read_byte(&self, offset: u16) -> u8 {
        match self {
            Self::Empty => 0,
            Self::Memory(memory) => memory[offset as usize],
            Self::Io(controller) => controller.read_byte(offset),
        }
    }

    /// Read a half-word of data from the block.
    pub fn read_half_word(&self, offset: u16) -> u16 {
        if offset > u16::MAX - 1 {
            return 0;
        }
        let bytes = match self {
            Self::Empty => {
                return 0;
            }
            Self::Memory(memory) => {
                let offset = offset as usize;
                [memory[offset + 0], memory[offset + 1]]
            }
            Self::Io(controller) => [
                controller.read_byte(offset + 0),
                controller.read_byte(offset + 1),
            ],
        };
        u16::from_be_bytes(bytes)
    }

    /// Read a word of data from the block.
    pub fn read_word(&self, offset: u16) -> u32 {
        if offset > u16::MAX - 3 {
            return 0;
        }
        let bytes = match self {
            Self::Empty => {
                return 0;
            }
            Self::Memory(memory) => {
                let offset = offset as usize;
                [
                    memory[offset + 0],
                    memory[offset + 1],
                    memory[offset + 2],
                    memory[offset + 3],
                ]
            }
            Self::Io(controller) => [
                controller.read_byte(offset + 0),
                controller.read_byte(offset + 1),
                controller.read_byte(offset + 2),
                controller.read_byte(offset + 3),
            ],
        };
        u32::from_be_bytes(bytes)
    }

    /// Write a byte of data to the block.
    pub fn write_byte(&mut self, offset: u16, data: u8) {
        match self {
            Self::Empty => {
                if data == 0 {
                    return;
                }
                *self = Self::new_memory();
                self.write_byte(offset, data);
            }
            Self::Memory(memory) => {
                memory[offset as usize] = data;
            }
            Self::Io(controller) => {
                controller.write_byte(offset, data);
            }
        }
    }

    /// Write a half-word of data to the block.
    pub fn write_half_word(&mut self, offset: u16, data: u16) {
        if offset > u16::MAX - 2 {
            return;
        }
        let bytes = data.to_be_bytes();
        match self {
            Self::Empty => {
                if data == 0 {
                    return;
                }
                *self = Self::new_memory();
                self.write_half_word(offset, data);
            }
            Self::Memory(memory) => {
                let offset = offset as usize;
                memory[offset + 0] = bytes[0];
                memory[offset + 1] = bytes[1];
            }
            Self::Io(controller) => {
                controller.write_byte(offset + 0, bytes[0]);
                controller.write_byte(offset + 1, bytes[1]);
            }
        };
    }

    /// Write a word of data to the block.
    pub fn write_word(&mut self, offset: u16, data: u32) {
        if offset > u16::MAX - 4 {
            return;
        }
        let bytes = data.to_be_bytes();
        match self {
            Self::Empty => {
                if data == 0 {
                    return;
                }
                *self = Self::new_memory();
                self.write_word(offset, data);
            }
            Self::Memory(memory) => {
                let offset = offset as usize;
                memory[offset + 0] = bytes[0];
                memory[offset + 1] = bytes[1];
                memory[offset + 2] = bytes[2];
                memory[offset + 3] = bytes[3];
            }
            Self::Io(controller) => {
                controller.write_byte(offset + 0, bytes[0]);
                controller.write_byte(offset + 1, bytes[1]);
                controller.write_byte(offset + 2, bytes[2]);
                controller.write_byte(offset + 3, bytes[3]);
            }
        };
    }
}

/// The result of the VM executing a single instruction.
pub struct InstructionResult {
    // Whether the instruction could be decoded.
    // (Invalid is no-op).
    pub valid: bool,

    // Whether the program counter was overwritten.
    pub jumped: bool,
}

/// Instance of the Alphabet virtual machine.
pub struct Vm {
    program_counter: u32,
    registers: [u32; REGISTER_COUNT],
    blocks: [Block; BLOCK_COUNT],

    // I/O device index caching.
    io_indices: Vec<usize>,
    io_indices_valid: bool,
}

/// The status of whether a queried block
/// already existed.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BlockExistence {
    /// The block already existed.
    Existed,
    /// The block was created.
    Created,
}

impl Vm {
    /// Create a new virtual machine.
    pub fn new() -> Self {
        Self {
            program_counter: 0,
            registers: [0; REGISTER_COUNT],
            blocks: array::from_fn(|_| Block::Empty),
            io_indices: Vec::with_capacity(BLOCK_COUNT),
            io_indices_valid: true,
        }
    }

<<<<<<< Updated upstream:alphabet/src/vm.rs
    /// Reset the virtual machine's
    /// program counter, registers,
    /// and memory.
    pub fn reset(&mut self) {
        self.program_counter = 0;
        self.registers.fill(0);
        self.blocks.clear();
    }

=======
>>>>>>> Stashed changes:src/vm.rs
    /// Restart the program, resettings
    /// the virtual machine's program counter
    /// and registers.
    pub fn restart(&mut self) {
        self.program_counter = 0;
        self.registers.fill(0);
    }

    /// Reset the virtual machine's
    /// program counter, registers,
    /// and memory.
    pub fn reset(&mut self) {
        self.restart();
        self.blocks.fill_with(|| Block::Empty);
    }

    /// Get the word address of the next
    /// instruction to execute.
    pub fn program_counter(&self) -> u32 {
        self.program_counter
    }

    /// Set the program counter to the
    /// specified word address.
    pub fn set_program_counter(&mut self, address: u32) {
        self.program_counter = address & MAX_WORD_ADDRESS;
    }

    /// Get the value of the specified register.
    pub fn register(&self, index: usize) -> u32 {
        // r0 should always hold value 0.
        self.registers[index % REGISTER_COUNT]
    }

    /// Read the value of all regiters.
    pub fn registers(&self) -> &[u32; REGISTER_COUNT] {
        &self.registers
    }

    /// Set the value of the specified register.
    pub fn set_register(&mut self, index: usize, word: u32) {
        let index = index % REGISTER_COUNT;
        if index == 0 {
            return;
        }
        self.registers[index] = word;
    }

    fn refresh_io_index_cache(&mut self) {
        if self.io_indices_valid {
            return;
        }
        self.io_indices.clear();
        let io_indices = self
            .blocks
            .iter()
            .enumerate()
            .filter(|(_, block)| block.is_io())
            .map(|(i, _)| i);
        self.io_indices.extend(io_indices);
        self.io_indices_valid = true;
    }

    /// Get all VM blocks.
    pub fn blocks(&self) -> &[Block; BLOCK_COUNT] {
        &self.blocks
    }

    /// Mutably get all VM blocks.
    pub fn blocks_mut(&mut self) -> &mut [Block; BLOCK_COUNT] {
        &mut self.blocks
    }

    /// Get a block of memory with the given index.
    pub fn block(&self, block_index: u16) -> &Block {
        &self.blocks[block_index as usize]
    }

    /// Get a mutable block of memory with the given index.
    pub fn block_mut(&mut self, block_index: u16) -> &mut Block {
        // Because this block can be replaced with an I/O
        // block through the reference, we'll need to check
        // for that next time we tick all I/O devices.
        self.io_indices_valid = false;

        &mut self.blocks[block_index as usize]
    }

    /// Get a block from the requested byte address,
    /// returning the block and the offset.
    pub fn block_from_addr(&self, address: u32) -> (&Block, u16) {
        let block_index = (address >> 16) as u16;
        let offset = address as u16;
        (self.block(block_index), offset)
    }

    /// Get a block from the requested byte address,
    /// returning the block and the offset.
    pub fn block_from_addr_mut(&mut self, address: u32) -> (&mut Block, u16) {
        let block_index = (address >> 16) as u16;
        let offset = address as u16;
        (self.block_mut(block_index), offset)
    }

    /// Configure/reset a block of memory.
    pub fn set_block(&mut self, block_index: u16, block: Block) -> Block {
        let new_block_is_io = block.is_io();
        let old_block = std::mem::replace(self.block_mut(block_index), block);
        let old_block_is_io = old_block.is_io();
        let block_index = block_index as usize;
        if old_block_is_io && !new_block_is_io {
            self.io_indices.retain(|&index| index != block_index);
        } else if !old_block_is_io && new_block_is_io {
            self.io_indices.push(block_index);
        }
        old_block
    }

    /// Reset the specified block, detaching any
    /// associated I/O device.
    pub fn remove_block(&mut self, block_index: u16) -> Block {
        self.set_block(block_index, Block::Empty)
    }

    /// Get a block of memory. If it is empty,
    /// create a new read/write memory block.
    pub fn get_block_or_create(&mut self, block_index: u16) -> (&mut Block, BlockExistence) {
        let block = self.block_mut(block_index);
        if block.is_empty() {
            *block = Block::new_memory();
            (block, BlockExistence::Created)
        } else {
            (block, BlockExistence::Existed)
        }
    }

    /// Notify all I/O controllers they may update their state.
    pub fn tick_all(&mut self) {
        self.refresh_io_index_cache();
        for &block_index in &self.io_indices {
            self.blocks[block_index].tick();
        }
    }

    /// Read a byte of data from virtual memory.
    pub fn read_byte(&self, address: u32) -> u8 {
        let (block, offset) = self.block_from_addr(address);
        block.read_byte(offset)
    }

    /// Read a half-word of data from virtual memory.
    pub fn read_half_word(&self, address: u32) -> u16 {
        let (block, offset) = self.block_from_addr(address);
        block.read_half_word(offset)
    }

    /// Read a word of data from virtual memory.
    pub fn read_word(&self, address: u32) -> u32 {
        let (block, offset) = self.block_from_addr(address);
        block.read_word(offset)
    }

    /// Tick the I/O controller if necessary, then
    /// read a byte of data from virtual memory.
    pub fn tick_read_byte(&mut self, address: u32) -> u8 {
        let (block, offset) = self.block_from_addr_mut(address);
        block.tick();
        block.read_byte(offset)
    }

    /// Tick the I/O controller if necessary, then
    /// read a half-word of data from virtual memory.
    pub fn tick_read_half_word(&mut self, address: u32) -> u16 {
        let (block, offset) = self.block_from_addr_mut(address);
        block.tick();
        block.read_half_word(offset)
    }

    /// Tick the I/O controller if necessary, then
    /// read a word of data from virtual memory.
    pub fn tick_read_word(&mut self, address: u32) -> u32 {
        let (block, offset) = self.block_from_addr_mut(address);
        block.tick();
        block.read_word(offset)
    }

    /// Write a byte of data to virtual memory.
    pub fn write_byte(&mut self, address: u32, data: u8) {
        let (block, offset) = self.block_from_addr_mut(address);
        block.write_byte(offset, data);
    }

    /// Write a half-word of data to virtual memory.
    pub fn write_half_word(&mut self, address: u32, data: u16) {
        let (block, offset) = self.block_from_addr_mut(address);
        block.write_half_word(offset, data);
    }

    /// Write a word of data to virtual memory.
    pub fn write_word(&mut self, address: u32, data: u32) {
        let (block, offset) = self.block_from_addr_mut(address);
        block.write_word(offset, data);
    }

    /// Write an instruction to virtual memory. Fails and
    /// returns false if the block is I/O-mapped.
    pub fn write_instruction(&mut self, address: u32, instruction: &Instruction) -> bool {
        let (block, offset) = self.block_from_addr_mut(address);
        if block.is_io() {
            return false;
        }
        let data = instruction.encode();
        block.write_word(offset, data);
        true
    }

<<<<<<< Updated upstream:alphabet/src/vm.rs
    fn exec_r_type(&mut self, instruction: Instruction) -> InstructionResult {
        let payload = unsafe { instruction.payload.r_type };
        let r_op_1 = self.read_register(payload.r_op_1 & 0x1F);
        let r_op_2 = self.read_register(payload.r_op_2 & 0x1F);
        let result = match instruction.op {
            ADD => r_op_1.wrapping_add(r_op_2),
            SUB => r_op_1.wrapping_sub(r_op_2),
            SHL => r_op_1 << (r_op_2 & 0x1F),
            SHR => r_op_1 >> (r_op_2 & 0x1F),
            SAR => (r_op_1 as i32 >> (r_op_2 & 0x1F)) as u32,
            AND => r_op_1 & r_op_2,
            OR => r_op_1 | r_op_2,
            XOR => r_op_1 ^ r_op_2,
            SLT => {
                if (r_op_1 as i32) < (r_op_2 as i32) {
=======
    const SHIFT_MASK: u32 = 0x1F;

    fn exec_r_type(&mut self, operation: Operation, payload: &RTypePayload) -> InstructionResult {
        let r_a = self.register(payload.register_a_index());
        let r_b = self.register(payload.register_b_index());

        let result = match operation.opcode() {
            Operation::ADD_CODE => r_a.wrapping_add(r_b),
            Operation::SUB_CODE => r_a.wrapping_sub(r_b),
            Operation::SHL_CODE => r_a << (r_b & Self::SHIFT_MASK),
            Operation::SHR_CODE => r_a >> (r_b & Self::SHIFT_MASK),
            Operation::SAR_CODE => (r_a as i32 >> (r_b & Self::SHIFT_MASK)) as u32,
            Operation::AND_CODE => r_a & r_b,
            Operation::OR_CODE => r_a | r_b,
            Operation::XOR_CODE => r_a ^ r_b,
            Operation::SLT_CODE => {
                if (r_a as i32) < (r_b as i32) {
>>>>>>> Stashed changes:src/vm.rs
                    1
                } else {
                    0
                }
            }
            SLTU => {
                if r_op_1 < r_op_2 {
                    1
                } else {
                    0
                }
            }
            _ => {
                return InstructionResult {
                    valid: false,
                    jumped: false,
                };
            }
        };
<<<<<<< Updated upstream:alphabet/src/vm.rs
        self.write_register(payload.r_result, result);
        InstructionResult {
            valid: true,
            jumped: false,
        }
    }

    fn exec_i_type(&mut self, instruction: Instruction) -> InstructionResult {
        let payload = unsafe { instruction.payload.i_type };
        let r_op = self.read_register(payload.r_op & 0x1F);
        let r_src = self.read_register(payload.r_result & 0x1F);
        let imm = payload.imm;
=======
        self.set_register(payload.register_r_index(), result);
        InstructionResult { jumped: false }
    }

    fn exec_i_type(&mut self, operation: Operation, payload: &ITypePayload) -> InstructionResult {
        let r_r = self.register(payload.register_r_index());
        let r_a = self.register(payload.register_a_index());
        let imm = payload.immediate_value();
>>>>>>> Stashed changes:src/vm.rs
        let mut jumped = false;
        let result = match instruction.op {
            ADDI => Some(r_op.wrapping_add(imm as u32)),
            SUBI => Some(r_op.wrapping_sub(imm as u32)),
            SHLI => Some(r_op << (imm & 0x1F)),
            SHRI => Some(r_op >> (imm & 0x1F)),
            SARI => Some((r_op as i32 >> (imm & 0x1F)) as u32),
            ANDI => Some(r_op & (imm as u32)),
            ANDUI => Some(r_op & ((imm as u32) << 16)),
            ORI => Some(r_op | (imm as u32)),
            ORUI => Some(r_op | ((imm as u32) << 16)),
            XORI => Some(r_op ^ (imm as u32)),
            XORUI => Some(r_op ^ ((imm as u32) << 16)),
            SLTI => Some(if (r_op as i32) < (imm as i16 as i32) {
                1
            } else {
                0
            }),
<<<<<<< Updated upstream:alphabet/src/vm.rs
            SLTUI => Some(if r_op < imm as u32 { 1 } else { 0 }),
            LDW => {
                let addr = r_op.wrapping_add_signed(imm as i16 as i32);
                let word = self.read_word(addr);
                Some(word)
            }
            LDHW => {
                let addr = r_op.wrapping_add_signed(imm as i16 as i32);
                let word = self.read_half_word(addr) as i16 as i32 as u32;
                Some(word)
            }
            LDHWU => {
                let addr = r_op.wrapping_add_signed(imm as i16 as i32);
                let word = self.read_half_word(addr) as u32;
                Some(word)
            }
            LDB => {
                let addr = r_op.wrapping_add_signed(imm as i16 as i32);
                let word = self.read_byte(addr) as i8 as i32 as u32;
                Some(word)
            }
            LDBU => {
                let addr = r_op.wrapping_add_signed(imm as i16 as i32);
                let word = self.read_byte(addr) as u32;
=======
            Operation::SLTUI_CODE => Some(if r_a < imm as u32 { 1 } else { 0 }),
            Operation::LDW_CODE => {
                let addr = r_a.wrapping_add_signed(imm as i16 as i32);
                let word = self.tick_read_word(addr);
                Some(word)
            }
            Operation::LDHW_CODE => {
                let addr = r_a.wrapping_add_signed(imm as i16 as i32);
                let word = self.tick_read_half_word(addr) as i16 as i32 as u32;
                Some(word)
            }
            Operation::LDHWU_CODE => {
                let addr = r_a.wrapping_add_signed(imm as i16 as i32);
                let word = self.tick_read_half_word(addr) as u32;
                Some(word)
            }
            Operation::LDB_CODE => {
                let addr = r_a.wrapping_add_signed(imm as i16 as i32);
                let word = self.tick_read_byte(addr) as i8 as i32 as u32;
                Some(word)
            }
            Operation::LDBU_CODE => {
                let addr = r_a.wrapping_add_signed(imm as i16 as i32);
                let word = self.tick_read_byte(addr) as u32;
>>>>>>> Stashed changes:src/vm.rs
                Some(word)
            }
            STW => {
                let addr = r_op.wrapping_add_signed(imm as i16 as i32);
                self.write_word(addr, r_src);
                None
            }
            STHW => {
                let addr = r_op.wrapping_add_signed(imm as i16 as i32);
                self.write_half_word(addr, r_src as u16);
                None
            }
            STB => {
                let addr = r_op.wrapping_add_signed(imm as i16 as i32);
                self.write_byte(addr, r_src as u8);
                None
            }
            JMP => {
                let ret = (self.program_counter + 1) & MAX_WORD_ADDRESS;
<<<<<<< Updated upstream:alphabet/src/vm.rs
                self.program_counter =
                    self.program_counter.wrapping_add_signed(imm as i16 as i32) & MAX_WORD_ADDRESS;
=======
                self.set_program_counter(
                    self.program_counter.wrapping_add_signed(imm as i16 as i32),
                );
>>>>>>> Stashed changes:src/vm.rs
                jumped = true;
                Some(ret)
            }
            JMPR => {
                let ret = (self.program_counter + 1) & MAX_WORD_ADDRESS;
<<<<<<< Updated upstream:alphabet/src/vm.rs
                self.program_counter =
                    r_op.wrapping_add_signed(imm as i16 as i32) & MAX_WORD_ADDRESS;
                jumped = true;
                Some(ret)
            }
            BEQ => {
                if r_src == r_op {
                    self.program_counter =
                        self.program_counter.wrapping_add_signed(imm as i16 as i32)
                            & MAX_WORD_ADDRESS;
=======
                self.set_program_counter(r_a.wrapping_add_signed(imm as i16 as i32));
                jumped = true;
                Some(ret)
            }
            Operation::BEQ_CODE => {
                if r_r == r_a {
                    self.set_program_counter(
                        self.program_counter.wrapping_add_signed(imm as i16 as i32),
                    );
>>>>>>> Stashed changes:src/vm.rs
                    jumped = true;
                }
                None
            }
<<<<<<< Updated upstream:alphabet/src/vm.rs
            BNE => {
                if r_src != r_op {
                    self.program_counter =
                        self.program_counter.wrapping_add_signed(imm as i16 as i32)
                            & MAX_WORD_ADDRESS;
=======
            Operation::BNE_CODE => {
                if r_r != r_a {
                    self.set_program_counter(
                        self.program_counter.wrapping_add_signed(imm as i16 as i32),
                    );
>>>>>>> Stashed changes:src/vm.rs
                    jumped = true;
                }
                None
            }
            _ => {
                return InstructionResult {
                    valid: false,
                    jumped: false,
                };
            }
        };
        if let Some(result) = result {
<<<<<<< Updated upstream:alphabet/src/vm.rs
            self.write_register(payload.r_result, result);
        }
        InstructionResult {
            valid: true,
            jumped,
=======
            self.set_register(payload.register_r_index(), result);
>>>>>>> Stashed changes:src/vm.rs
        }
    }

    /// Write a word of data to virtual memory.
    pub fn execute(&mut self, instruction: Instruction) -> InstructionResult {
        if instruction.op == NOOP {
            InstructionResult {
                valid: true,
                jumped: false,
            }
        } else if is_op_r_type(instruction.op) {
            self.exec_r_type(instruction)
        } else {
            self.exec_i_type(instruction)
        }
    }

<<<<<<< Updated upstream:alphabet/src/vm.rs
    /// Tick I/O devices, run the next instruction,
    /// and advance the program counter.
    pub fn step_forward(&mut self) {
=======
    /// Move the program counter forward one instruction.
    pub fn advance(&mut self) {
        self.set_program_counter(self.program_counter + 1);
    }

    /// Run the next instruction, and advance the program counter.
    ///
    /// If the instruction is invalid, the program counter is still advanced.
    pub fn execute_and_advance(
        &mut self,
    ) -> Result<(Instruction, InstructionResult), InstructionError> {
>>>>>>> Stashed changes:src/vm.rs
        let instruction = self.read_word(self.program_counter * 4);
        let instruction = Instruction::decode(instruction);
        let result = self.execute(instruction);
        if !result.jumped {
<<<<<<< Updated upstream:alphabet/src/vm.rs
            self.program_counter = (self.program_counter + 1) & MAX_WORD_ADDRESS;
=======
            self.advance();
        }
        Ok((instruction, result))
    }

    /// Execute instructions sequentially as long as a condition is met.
    pub fn run_while(&mut self, mut predicate: impl FnMut(&Vm) -> bool) {
        while predicate(self) {
            // Ignore instruction errors.
            let _ = self.execute_and_advance();
        }
    }

    /// Run until the program counter is at a certain word address.
    /// The instruction at that address is not run.
    pub fn run_to_address(&mut self, stop_address: u32) {
        let stop_address = stop_address & MAX_WORD_ADDRESS;
        self.run_while(|vm| vm.program_counter() != stop_address);
    }

    /// Create a virtual machine with state initialized
    /// from an image.
    pub fn from_image(image: Image) -> Self {
        let mut vm = Self::new();
        for entry in image.entries {
            // Ignore invalid entries.
            if entry.start_offset > entry.end_offset {
                continue;
            }
            let mut data = [0; BLOCK_SIZE];
            let length = (entry.end_offset - entry.start_offset + 1) as usize;
            let offset = entry.start_offset as usize;
            for i in 0..length {
                data[i + offset] = entry.data[i];
            }
            let block = Block::with_data(data);
            vm.blocks[entry.block_index as usize] = block;
        }
        vm
    }

    /// Generate an image, a snapshot of the VM's
    /// state.
    pub fn image(&self) -> Image {
        let mut image = Image {
            entries: Vec::new(),
        };
        for (index, block) in self.blocks.iter().enumerate() {
            let Block::Memory(mem) = block else {
                continue;
            };
            let start_offset = mem.iter().position(|&b| b != 0);
            let Some(start_offset) = start_offset else {
                continue;
            };

            // Unwrap: guaranteed to be at least offset.
            let end_offset = BLOCK_SIZE - 1 - mem.iter().rev().position(|&b| b != 0).unwrap();

            let data = mem[start_offset..=end_offset].to_vec();
            let image_entry = ImageEntry {
                block_index: index as u16,
                start_offset: start_offset as u16,
                end_offset: end_offset as u16,
                data,
            };
            image.entries.push(image_entry);
        }
        image
    }
}

/// The contents of a block of
/// memory in an image.
pub struct ImageEntry {
    pub block_index: u16,
    pub start_offset: u16,
    pub end_offset: u16,
    pub data: Vec<u8>,
}

/// A representation of VM state.
pub struct Image {
    pub entries: Vec<ImageEntry>,
}

macro_rules! nextbyte {
    ($bytes: expr, $byte:ident, $image:expr) => {
        let Some(Ok($byte)) = $bytes.next() else {
            return $image;
        };
    };
}

impl<R: Read> From<R> for Image {
    fn from(reader: R) -> Self {
        let mut image = Image {
            entries: Vec::new(),
        };
        let mut bytes = reader.bytes();
        loop {
            // Extract block index bytes.
            nextbyte!(bytes, bi_u, image);
            nextbyte!(bytes, bi_l, image);

            // Extract start offset bytes.
            nextbyte!(bytes, so_u, image);
            nextbyte!(bytes, so_l, image);

            // Extract end offset bytes.
            nextbyte!(bytes, eo_u, image);
            nextbyte!(bytes, eo_l, image);

            let block_index = u16::from_be_bytes([bi_u, bi_l]);
            let start_offset = u16::from_be_bytes([so_u, so_l]);
            let end_offset = u16::from_be_bytes([eo_u, eo_l]);

            // Bad offset values are considered to be length 0.
            if start_offset > end_offset {
                continue;
            }

            let length = (end_offset - start_offset + 1) as usize;
            let mut data = Vec::with_capacity(length);
            for _ in 0..length {
                let Some(Ok(byte)) = bytes.next() else {
                    break;
                };
                data.push(byte);
            }
            let image_entry = ImageEntry {
                block_index,
                start_offset,
                end_offset,
                data,
            };
            image.entries.push(image_entry);
>>>>>>> Stashed changes:src/vm.rs
        }
    }
}
