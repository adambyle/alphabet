//! Virtual machine API.
//!
//! The Alphabet virtual machine has thirty-one general-purpose 32-bit registers.
//! Register 0 always reads 0 and ignores writes. The memory is byte-addressable
//! using 32-bit addresses.
//!
//! The memory is split into 2^16 _blocks_ of 2^16 bytes each. Implementation-wise,
//! each full block is heap-allocated as needed; this is done automatically the first
//! time an unused block is written with non-zero data.
//!
//! Although alignment is not enforced, reads and writes of half-words or words
//! that cross block boundaries are not allowed and will silently fail (reads
//! return 0). Alignment is enforced for instructions--the program counter
//! is a word address. Simlarly, instructions that move the program counter
//! use word addresses and word offsets instead of byte addresses and byte offsets.
//!
//! The program counter begins at 0 and wraps if advanced past the last valid
//! address or moved before 0.
//!
//! # I/O controllers
//!
//! I/O controllers may be mapped to any block of memory. There are no formal restrictions
//! on the behavior of these virtual I/O devices, only that they control all reads and writes
//! to addresses within their associated block. I/O controllers may be created to interface
//! with a console, graphical display, features of the host environment (such as random number
//! generation or time APIs), input devices like the mouse and keyboard, and even the network.
//!
//! See the [`IoController`] trait for more recommendations on implementation.
//!
//! **Note:** The program counter may enter a memory-mapped block. Executing an instruction
//! entails reading a word of memory from the I/O controller and interpreting it as an
//! instruction; in other words, the VM does not care whether the instruction comes
//! from a memory block or an I/O block.

use std::{
    collections::BTreeSet,
    io::{self, Read, Write},
    iter,
};

use crate::{
    Image, Instruction, Operation,
    image::{ImageEntries, ImageEntryRef},
    is::{ITypePayload, InstructionError, Payload, RTypePayload, inst},
};

/// The number of byte addresses in each block of VM memory.
pub const BLOCK_SIZE: usize = 1 << 16;

/// The number of blocks in VM memory.
pub const BLOCK_COUNT: usize = 1 << 16;

/// The number of registers supported by the system.
pub const REGISTER_COUNT: usize = 32;

/// The maximum valid word address.
pub const MAX_WORD_ADDRESS: u32 = (1 << 30) - 1;

/// The byte contents of a readable/writable memory block.
///
/// This byte array will usually be [boxed](Box).
pub type BlockBytes = [u8; BLOCK_SIZE];

fn write_memory_bytes(block_bytes: &mut BlockBytes, new_bytes: &[u8], offset: u16) {
    let offset = offset as usize;
    let end = offset + new_bytes.len();
    let target = &mut block_bytes[offset..end];
    target.copy_from_slice(new_bytes);
}

/// A virtual I/O device controller.
///
/// An I/O controller determines the behavior of all
/// memory reads and writes to addresses within its block.
/// The controller may not know the original address of the
/// operation, only the offset within its block.
///
/// A well-implemented I/O controller will have nearly
/// instant implementations for [`read_byte`](Self::read_byte) and
/// [`write_byte`](Self::write_byte).
/// It should leverage asynchronous operations if possible, and
/// provide a status field for long-running operations.
///
/// The [`tick`](Self::tick) method allows the controller to update its state
/// if necessary, usually before a read. (See [`Vm::set_tick_on_read`].)
///
/// # Example
///
/// This example is contrived because it implements a feature that may
/// as well be implemented in machine instructions, but it demonstrates
/// implementation conventions that apply to more appropriate use cases.
///
/// ```rust
/// # use alphabet::vm::IoController;
///
/// // Accumulates byte inputs into a 32-bit total.
/// struct Accumulator {
///     total: u32,
///     total_bytes: [u8; 4],
/// }
///
/// impl IoController for Accumulator {
///     fn read_byte(&self, offset: u16) -> u8 {
///         if offset >= 4 {
///             return 0;
///         }
///         self.total_bytes[offset as usize]
///     }
///
///     fn tick(&mut self) {
///         // Split total into bytes before read.
///         self.total_bytes = self.total.to_be_bytes();
///     }
///
///     fn write_byte(&mut self, _offset: u16, byte: u8) {
///         // Offset doesn't matter, add written byte to total.
///         self.total = self.total.wrapping_add(byte as u32);
///     }
/// }
/// ```
pub trait IoController {
    /// Read a byte of data from the controller.
    ///
    /// The byte read is `offset` away from the first
    /// address in the associated block.
    fn read_byte(&self, offset: u16) -> u8;

    /// Notify the I/O device that it may update state.
    fn tick(&mut self);

    /// Write a byte of data to the controller.
    ///
    /// The byte written is `offset` away from the first
    /// address in the associated block.
    fn write_byte(&mut self, offset: u16, byte: u8);
}

/// The status of whether a queried block
/// already existed or whether memory
/// was allocated.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BlockExistence {
    /// The block already existed.
    Existed,
    /// The block was created.
    Created,
    /// The block did not exist and was not created.
    Ignored,
}

/// A block of byte-addressed virtual machine memory.
///
/// Memory for a block is not allocated unless a non-zero
/// value is written within the block.
pub enum Block {
    /// The block has not been written and has not
    /// been mapped to an I/O controller. No space
    /// for the block's memory is allocated.
    Empty,

    /// Arbitrarily readable and writable memory.
    /// A full block of space is allocated (see [`BLOCK_SIZE`]).
    Memory(Box<BlockBytes>),

    /// A memory-mapped I/O device.
    Io(Box<dyn IoController>),
}

impl Block {
    /// Create a new memory block containing the specified data.
    pub fn with_data(data: Box<BlockBytes>) -> Self {
        Self::Memory(data)
    }

    /// Create a new block mapped to an I/O controller.
    pub fn with_controller(controller: impl IoController + 'static) -> Self {
        Self::Io(Box::new(controller))
    }

    /// Create a new zeroed memory block.
    ///
    /// Note that writes to [empty blocks](Block::Empty)
    /// automatically allocated memory, so creating a
    /// zeroed memory block is only necessary if you need
    /// to proactively allocate memory.
    pub fn new_memory() -> Self {
        Self::with_data(Box::new([0; BLOCK_SIZE]))
    }

    /// Whether the block is empty and no
    /// memory is allocated.
    ///
    /// A zeroed block of memory will return `false`.
    pub fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }

    /// Whether the block is allocated readable/writable memory.
    pub fn is_memory(&self) -> bool {
        matches!(self, Self::Memory(_))
    }

    /// Whether the block is I/O-mapped.
    pub fn is_io(&self) -> bool {
        matches!(self, Self::Io(_))
    }

    /// If this is an I/O block, notify
    /// the controller it may update state.
    ///
    /// See [`IoController::tick`].
    pub fn tick(&mut self) -> bool {
        if let Self::Io(controller) = self {
            controller.tick();
            true
        } else {
            false
        }
    }

    /// Read a byte of data from the block.
    ///
    /// If the block is empty, returns 0. If the block
    /// is mapped to an I/O controller, the controller
    /// determines the byte that is read.
    pub fn read_byte(&self, offset: u16) -> u8 {
        match self {
            Self::Empty => 0,
            Self::Memory(memory) => memory[offset as usize],
            Self::Io(controller) => controller.read_byte(offset),
        }
    }

    /// Read a half-word of data from the block.
    ///
    /// If the block is empty, returns 0. If the block
    /// is mapped to an I/O controller, the controller
    /// determines the half-word that is read.
    ///
    /// Reads of fewer than 2 bytes at the end of the block
    /// will return 0 (reads across block boundaries
    /// silently fail.)
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
    ///
    /// If the block is empty, returns 0. If the block
    /// is mapped to an I/O controller, the controller
    /// determines the word that is read.
    ///
    /// Reads of fewer than 4 bytes at the end of the block
    /// will return 0 (reads across block boundaries
    /// silently fail.)
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
    ///
    /// If the block is empty, readable/writable memory
    /// is assigned to the block. If the block is mapped
    /// to an I/O controller, the controller determines
    /// the behavior of the write.
    pub fn write_byte(&mut self, offset: u16, data: u8) -> BlockExistence {
        match self {
            Self::Empty => {
                if data == 0 {
                    return BlockExistence::Ignored;
                }
                *self = Self::new_memory();
                self.write_byte(offset, data);
                BlockExistence::Created
            }
            Self::Memory(memory) => {
                memory[offset as usize] = data;
                BlockExistence::Existed
            }
            Self::Io(controller) => {
                controller.write_byte(offset, data);
                BlockExistence::Existed
            }
        }
    }

    /// Write a half-word of data to the block.
    ///
    /// If the block is empty, readable/writable memory
    /// is assigned to the block. If the block is mapped
    /// to an I/O controller, the controller determines
    /// the behavior of the write.
    ///
    /// Writes of fewer than 2 bytes at the end of the block
    /// will fail.
    pub fn write_half_word(&mut self, offset: u16, data: u16) -> BlockExistence {
        if offset > u16::MAX - 2 {
            return if self.is_empty() {
                BlockExistence::Ignored
            } else {
                BlockExistence::Existed
            };
        }
        let bytes = data.to_be_bytes();
        match self {
            Self::Empty => {
                if data == 0 {
                    return BlockExistence::Ignored;
                }
                *self = Self::new_memory();
                self.write_half_word(offset, data);
                BlockExistence::Created
            }
            Self::Memory(memory) => {
                let offset = offset as usize;
                memory[offset + 0] = bytes[0];
                memory[offset + 1] = bytes[1];
                BlockExistence::Existed
            }
            Self::Io(controller) => {
                controller.write_byte(offset + 0, bytes[0]);
                controller.write_byte(offset + 1, bytes[1]);
                BlockExistence::Existed
            }
        }
    }

    /// Write a word of data to the block.
    ///
    /// If the block is empty, readable/writable memory
    /// is assigned to the block. If the block is mapped
    /// to an I/O controller, the controller determines
    /// the behavior of the write.
    ///
    /// Writes of fewer than 4 bytes at the end of the block
    /// will fail.
    pub fn write_word(&mut self, offset: u16, data: u32) -> BlockExistence {
        if offset > u16::MAX - 4 {
            return if self.is_empty() {
                BlockExistence::Ignored
            } else {
                BlockExistence::Existed
            };
        }
        let bytes = data.to_be_bytes();
        match self {
            Self::Empty => {
                if data == 0 {
                    return BlockExistence::Ignored;
                }
                *self = Self::new_memory();
                self.write_word(offset, data);
                BlockExistence::Created
            }
            Self::Memory(memory) => {
                let offset = offset as usize;
                memory[offset + 0] = bytes[0];
                memory[offset + 1] = bytes[1];
                memory[offset + 2] = bytes[2];
                memory[offset + 3] = bytes[3];
                BlockExistence::Existed
            }
            Self::Io(controller) => {
                controller.write_byte(offset + 0, bytes[0]);
                controller.write_byte(offset + 1, bytes[1]);
                controller.write_byte(offset + 2, bytes[2]);
                controller.write_byte(offset + 3, bytes[3]);
                BlockExistence::Existed
            }
        }
    }

    /// Write an arbitrary number of bytes into the block.
    ///
    /// If the block is empty, readable/writable memory
    /// is assigned to the block. If the block is mapped
    /// to an I/O controller, the controller determines
    /// the behavior of the write.
    ///
    /// Returns the bytes that were unwritten because they could not fit.
    pub fn write_bytes<'a>(&mut self, offset: u16, bytes: &'a [u8]) -> (&'a [u8], BlockExistence) {
        if bytes.is_empty() {
            return (
                bytes,
                if self.is_empty() {
                    BlockExistence::Ignored
                } else {
                    BlockExistence::Existed
                },
            );
        }

        // Bytes must be cutoff if they cannot fit.
        let end = usize::min(BLOCK_SIZE - offset as usize, bytes.len());
        let bytes_left = &bytes[end..];
        let new_bytes = &bytes[..end];

        let existence = match self {
            Self::Empty => {
                let non_zero = new_bytes.iter().any(|&b| b != 0);
                if !non_zero {
                    return (bytes_left, BlockExistence::Ignored);
                }
                let mut block_bytes = Box::new([0; BLOCK_SIZE]);
                write_memory_bytes(&mut block_bytes, new_bytes, offset);
                *self = Block::with_data(block_bytes);
                BlockExistence::Created
            }
            Self::Memory(memory) => {
                write_memory_bytes(memory, new_bytes, offset);
                BlockExistence::Existed
            }
            Self::Io(controller) => {
                let mut offset = offset;
                for &byte in new_bytes {
                    controller.write_byte(offset, byte);
                    offset += 1;
                }
                BlockExistence::Existed
            }
        };

        (bytes_left, existence)
    }
}

/// The outcome of the VM executing a single instruction.
#[derive(Debug)]
pub struct InstructionOutcome {
    /// Whether the program counter was overwritten (jumped
    /// to a location instead of advancing by 1).
    pub jumped: bool,
}

/// When a mutable reference to a block is taken, the
/// user of the reference can change the type of the
/// block through the reference. This status value
/// tracks which blocks need re-checks.
enum IoStatus {
    /// The type of all blocks are known.
    Known,
    /// The wrapped indices need to be rechecked.
    Check(Vec<usize>),
    /// All indices need to be rechecked.
    CheckAll,
}

/// The outcome of an operation writing an arbitrary number of bytes
/// to memory (see [`Vm::write_bytes`]).
pub struct VmWrite<'a> {
    /// Unwritten bytes, as the last address in memory was reached.
    pub leftover: &'a [u8],
    /// The indices of previously unallocated blocks of memory
    /// that were created from this operation.
    pub created_blocks: Vec<u16>,
}

/// The result of the VM executing an instruction.
type ExecuteResult = Result<(Instruction, InstructionOutcome), InstructionError>;

/// Instance of the Alphabet virtual machine.
pub struct Vm {
    program_counter: u32,
    registers: [u32; REGISTER_COUNT],
    blocks: Box<[Block; BLOCK_COUNT]>,
    tick_on_read: bool,

    // I/O device index caching.
    io_indices: BTreeSet<usize>,
    io_status: IoStatus,
}

impl Vm {
    /// Create a new virtual machine.
    pub fn new() -> Self {
        let blocks = iter::repeat_with(|| Block::Empty)
            .take(BLOCK_COUNT)
            .collect::<Vec<_>>()
            .try_into()
            .map_err(|_| ())
            .expect("failed to initialize blocks");
        Self {
            program_counter: 0,
            registers: [0; REGISTER_COUNT],
            blocks,
            tick_on_read: true,
            io_indices: BTreeSet::new(),
            io_status: IoStatus::Known,
        }
    }

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
        self.io_indices.clear();
        self.io_status = IoStatus::Known;
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

    /// Get all VM blocks.
    pub fn blocks(&self) -> &[Block; BLOCK_COUNT] {
        &self.blocks
    }

    /// Mutably get all VM blocks.
    ///
    /// Using this method in combination with [`Vm::tick_all`] is
    /// unperformant as the VM must recheck all blocks in case any
    /// were changed to/from I/O-mapped blocks.
    pub fn blocks_mut(&mut self) -> &mut [Block; BLOCK_COUNT] {
        // Because I/O blocks can be added or removed
        // through the reference, we need to recheck
        // for them before ticking.
        self.io_status = IoStatus::CheckAll;

        &mut self.blocks
    }

    /// Get a block of memory with the given index.
    pub fn block(&self, block_index: u16) -> &Block {
        &self.blocks[block_index as usize]
    }

    /// Get a mutable block of memory with the given index.
    pub fn block_mut(&mut self, block_index: u16) -> &mut Block {
        let block_index = block_index as usize;

        // Because an I/O block can be added or removed
        // through the reference, we need to recheck
        // for them before ticking.
        if let IoStatus::Known = self.io_status {
            self.io_status = IoStatus::Check(vec![block_index]);
        } else if let IoStatus::Check(ref mut indices) = self.io_status
            && !indices.contains(&block_index)
        {
            indices.push(block_index);
        }

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
            self.io_indices.remove(&block_index);
        } else if !old_block_is_io && new_block_is_io {
            self.io_indices.insert(block_index);
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

    /// Whether memory-read instructions automatically
    /// tick an I/O device when reading memory from
    /// its block.
    pub fn tick_on_read(&self) -> bool {
        self.tick_on_read
    }

    /// Set whether memory-read instructions automatically
    /// tick an I/O device when reading memory from its
    /// block.
    pub fn set_tick_on_read(&mut self, tick_on_read: bool) {
        self.tick_on_read = tick_on_read;
    }

    /// Notify all I/O controllers they may update their state.
    ///
    /// This method uses an internal cache of I/O mapped blocks.
    /// The [`Vm::blocks_mut`] method resets this cache, leading
    /// to poorer performance for the next call to `tick_all`.
    pub fn tick_all(&mut self) {
        // Refresh index cache.
        if let IoStatus::CheckAll = self.io_status {
            // If all need to be checked,
            // tick while updating cache.
            self.io_indices = self
                .blocks
                .iter_mut()
                .enumerate()
                .filter_map(|(i, b)| {
                    b.is_io().then(|| {
                        b.tick();
                        i
                    })
                })
                .collect();
            self.io_status = IoStatus::Known;
            return;
        }

        if let IoStatus::Check(ref indices) = self.io_status {
            for &index in indices {
                let is_io = self.blocks[index].is_io();
                if is_io {
                    self.io_indices.insert(index);
                } else {
                    self.io_indices.remove(&index);
                }
            }
            self.io_status = IoStatus::Known;
        }
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
    ///
    /// See [`Block::read_half_word`] for block-boundary behavior.
    pub fn read_half_word(&self, address: u32) -> u16 {
        let (block, offset) = self.block_from_addr(address);
        block.read_half_word(offset)
    }

    /// Read a word of data from virtual memory.
    ///
    /// See [`Block::read_word`] for block-boundary behavior.
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
    ///
    /// See [`Block::read_half_word`] for block-boundary behavior.
    pub fn tick_read_half_word(&mut self, address: u32) -> u16 {
        let (block, offset) = self.block_from_addr_mut(address);
        block.tick();
        block.read_half_word(offset)
    }

    /// Tick the I/O controller if necessary, then
    /// read a word of data from virtual memory.
    ///
    /// See [`Block::read_word`] for block-boundary behavior.
    pub fn tick_read_word(&mut self, address: u32) -> u32 {
        let (block, offset) = self.block_from_addr_mut(address);
        block.tick();
        block.read_word(offset)
    }

    /// Write a byte of data to virtual memory.
    ///
    /// See [`Block::write_byte`] for allocation behavior.
    pub fn write_byte(&mut self, address: u32, data: u8) -> BlockExistence {
        let (block, offset) = self.block_from_addr_mut(address);
        block.write_byte(offset, data)
    }

    /// Write a half-word of data to virtual memory.
    ///
    /// See [`Block::write_half_word`] for allocation and block-boundary behavior.
    pub fn write_half_word(&mut self, address: u32, data: u16) -> BlockExistence {
        let (block, offset) = self.block_from_addr_mut(address);
        block.write_half_word(offset, data)
    }

    /// Write a word of data to virtual memory.
    ///
    /// See [`Block::write_word`] for allocation and block-boundary behavior.
    pub fn write_word(&mut self, address: u32, data: u32) -> BlockExistence {
        let (block, offset) = self.block_from_addr_mut(address);
        block.write_word(offset, data)
    }

    /// Write an instruction to a **word address** of virtual memory.
    pub fn write_instruction(
        &mut self,
        word_address: u32,
        instruction: &Instruction,
    ) -> BlockExistence {
        let byte_address = 4 * (word_address % MAX_WORD_ADDRESS);
        let (block, offset) = self.block_from_addr_mut(byte_address);
        let data = instruction.encode();
        block.write_word(offset, data)
    }

    /// Write an arbitrary number of bytes to memory, starting
    /// at the specified address. The unwritten bytes are returned,
    /// as writing does not wrap around if the last address is reached.
    pub fn write_bytes<'a>(&mut self, address: u32, mut bytes: &'a [u8]) -> VmWrite<'a> {
        let mut block_index = (address >> 16) as u16;
        let mut offset = (address & 0xFFFF) as u16;
        let mut created_blocks = Vec::new();
        loop {
            let existence;
            (bytes, existence) = self.blocks[block_index as usize].write_bytes(offset, bytes);
            if existence == BlockExistence::Created {
                created_blocks.push(block_index);
            }
            if bytes.is_empty() || block_index == u16::MAX {
                return VmWrite {
                    leftover: bytes,
                    created_blocks,
                };
            }
            block_index += 1;
            offset = 0;
        }
    }

    const SHIFT_MASK: u32 = 0x1F;

    fn exec_r_type(&mut self, operation: Operation, payload: &RTypePayload) -> InstructionOutcome {
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
                    1
                } else {
                    0
                }
            }
            Operation::SLTU_CODE => {
                if r_a < r_b {
                    1
                } else {
                    0
                }
            }
            _ => panic!("invalid R-type opcode"),
        };
        self.set_register(payload.register_r_index(), result);
        InstructionOutcome { jumped: false }
    }

    fn exec_i_type(&mut self, operation: Operation, payload: &ITypePayload) -> InstructionOutcome {
        let r_r = self.register(payload.register_r_index());
        let r_a = self.register(payload.register_a_index());
        let imm = payload.immediate_value();
        let mut jumped = false;

        let result = match operation.opcode() {
            Operation::ADDI_CODE => Some(r_a.wrapping_add(imm as u32)),
            Operation::SUBI_CODE => Some(r_a.wrapping_sub(imm as u32)),
            Operation::SHLI_CODE => Some(r_a << (imm & Self::SHIFT_MASK as u16)),
            Operation::SHRI_CODE => Some(r_a >> (imm & Self::SHIFT_MASK as u16)),
            Operation::SARI_CODE => Some((r_a as i32 >> (imm & Self::SHIFT_MASK as u16)) as u32),
            Operation::ANDI_CODE => Some(r_a & (imm as u32)),
            Operation::ANDUI_CODE => Some(r_a & ((imm as u32) << 16)),
            Operation::ORI_CODE => Some(r_a | (imm as u32)),
            Operation::ORUI_CODE => Some(r_a | ((imm as u32) << 16)),
            Operation::XORI_CODE => Some(r_a ^ (imm as u32)),
            Operation::XORUI_CODE => Some(r_a ^ ((imm as u32) << 16)),
            Operation::SLTI_CODE => Some(if (r_a as i32) < (imm as i16 as i32) {
                1
            } else {
                0
            }),
            Operation::SLTUI_CODE => Some(if r_a < imm as u32 { 1 } else { 0 }),
            Operation::LDW_CODE => {
                let addr = r_a.wrapping_add_signed(imm as i16 as i32);
                let word = if self.tick_on_read {
                    self.tick_read_word(addr)
                } else {
                    self.read_word(addr)
                };
                Some(word)
            }
            Operation::LDHW_CODE => {
                let addr = r_a.wrapping_add_signed(imm as i16 as i32);
                let half_word = if self.tick_on_read {
                    self.tick_read_half_word(addr)
                } else {
                    self.read_half_word(addr)
                } as i16 as i32 as u32;
                Some(half_word)
            }
            Operation::LDHWU_CODE => {
                let addr = r_a.wrapping_add_signed(imm as i16 as i32);
                let half_word = if self.tick_on_read {
                    self.tick_read_half_word(addr)
                } else {
                    self.read_half_word(addr)
                } as u32;
                Some(half_word)
            }
            Operation::LDB_CODE => {
                let addr = r_a.wrapping_add_signed(imm as i16 as i32);
                let byte = if self.tick_on_read {
                    self.tick_read_byte(addr)
                } else {
                    self.read_byte(addr)
                } as i8 as i32 as u32;
                Some(byte)
            }
            Operation::LDBU_CODE => {
                let addr = r_a.wrapping_add_signed(imm as i16 as i32);
                let byte = if self.tick_on_read {
                    self.tick_read_byte(addr)
                } else {
                    self.read_byte(addr)
                } as u32;
                Some(byte)
            }
            Operation::STW_CODE => {
                let addr = r_a.wrapping_add_signed(imm as i16 as i32);
                self.write_word(addr, r_r);
                None
            }
            Operation::STHW_CODE => {
                let addr = r_a.wrapping_add_signed(imm as i16 as i32);
                self.write_half_word(addr, r_r as u16);
                None
            }
            Operation::STB_CODE => {
                let addr = r_a.wrapping_add_signed(imm as i16 as i32);
                self.write_byte(addr, r_r as u8);
                None
            }
            Operation::JMP_CODE => {
                let ret = (self.program_counter + 1) & MAX_WORD_ADDRESS;
                self.set_program_counter(
                    self.program_counter.wrapping_add_signed(imm as i16 as i32),
                );
                jumped = true;
                Some(ret)
            }
            Operation::JMPR_CODE => {
                let ret = (self.program_counter + 1) & MAX_WORD_ADDRESS;
                self.set_program_counter(r_a.wrapping_add_signed(imm as i16 as i32));
                jumped = true;
                Some(ret)
            }
            Operation::BEQ_CODE => {
                if r_r == r_a {
                    self.set_program_counter(
                        self.program_counter.wrapping_add_signed(imm as i16 as i32),
                    );
                    jumped = true;
                }
                None
            }
            Operation::BNE_CODE => {
                if r_r != r_a {
                    self.set_program_counter(
                        self.program_counter.wrapping_add_signed(imm as i16 as i32),
                    );
                    jumped = true;
                }
                None
            }
            _ => panic!("invalid I-type opcode"),
        };
        if let Some(result) = result {
            self.set_register(payload.register_r_index(), result);
        }
        InstructionOutcome { jumped }
    }

    /// Execute an instruction on the VM.
    pub fn execute(&mut self, instruction: &Instruction) -> InstructionOutcome {
        let operation = instruction.operation();
        let payload = instruction.payload();
        match payload {
            Payload::Noop(_) => InstructionOutcome { jumped: false },
            Payload::RType(payload) => self.exec_r_type(operation, payload),
            Payload::IType(payload) => self.exec_i_type(operation, payload),
        }
    }

    /// Move the program counter forward one instruction, wrapping
    /// if appropriate.
    pub fn advance(&mut self) {
        self.set_program_counter(self.program_counter + 1);
    }

    /// Decode the instruction at the specified **word address**.
    ///
    /// # Errors
    ///
    /// This method fails if the word at the specified address
    /// fails to decode to a valid instruction (see [`Instruction::decode`]).
    pub fn instruction_at(&self, word_address: u32) -> Result<Instruction, InstructionError> {
        let instruction = self.read_word(word_address * 4);
        Instruction::decode(instruction)
    }

    /// Decode the instruction at the program counter.
    ///
    /// # Errors
    ///
    /// This method fails if the word at the program counter
    /// fails to decode to a valid instruction (see [`Instruction::decode`]).
    pub fn current_instruction(&self) -> Result<Instruction, InstructionError> {
        self.instruction_at(self.program_counter)
    }

    /// Run the current instruction and advance the program counter.
    ///
    /// # Errors
    ///
    /// This method fails if the current instruction is invalid
    /// (see [`Vm::current_instruction`]). The instruct counter
    /// is still advanced.
    pub fn execute_and_advance(&mut self) -> ExecuteResult {
        let instruction = match self.current_instruction() {
            Ok(instruction) => instruction,
            Err(err) => {
                self.advance();
                return Err(err);
            }
        };
        let result = self.execute(&instruction);
        if !result.jumped {
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

    /// Execute instructions sequentially until a condition on the last
    /// completed instruction is met.
    pub fn run_until_instruction(
        &mut self,
        mut predicate: impl FnMut(&ExecuteResult) -> bool,
    ) -> ExecuteResult {
        loop {
            let result = self.execute_and_advance();
            if predicate(&result) {
                return result;
            }
        }
    }

    /// Execute instructions sequentially until the program counter
    /// is at a certain word address.
    ///
    /// The instruction at that address is not run.
    pub fn run_to_pc(&mut self, stop_address: u32) {
        let stop_address = stop_address & MAX_WORD_ADDRESS;
        self.run_while(|vm| vm.program_counter() != stop_address);
    }

    /// Execute instructions sequentially until the same
    /// instruction is executing infinitely (jump with offset 0).
    ///
    /// Returns the result of the last instruction run.
    pub fn run_until_loop(&mut self) -> ExecuteResult {
        self.run_until_instruction(|result| {
            result.as_ref().is_ok_and(|(instruction, _)| {
                if instruction.operation() != Operation::JMP {
                    return false;
                }
                let payload = instruction
                    .i_type_payload()
                    .expect("jpm instruction should have r-type payload");
                let imm = payload.immediate_value();
                // Jump to current statement infinitely.
                imm == 0
            })
        })
    }

    /// Execute instructions sequentially until the program counter
    /// jumps/branches instead of advancing by 1.
    ///
    /// Returns the result of the last instruction run.
    pub fn run_until_jumped(&mut self) -> ExecuteResult {
        self.run_until_instruction(|result| {
            result.as_ref().is_ok_and(|(_, outcome)| outcome.jumped)
        })
    }

    /// Execute instructions until the word at the program counter does
    /// not decode to a valid instruction.
    ///
    /// Returns the error encountered decoding the last instruction.
    pub fn run_while_valid(&mut self) -> InstructionError {
        self.run_until_instruction(|result| result.is_err())
            .unwrap_err()
    }

    /// Get an image of the VM.
    pub fn image(&self) -> Image {
        FromIterator::from_iter(ImageEntries::from(self))
    }

    /// Write the contents of the virtual machine
    /// memory, according to the [`Image`] format.
    pub fn write_image_to(&self, writer: impl Write) -> io::Result<()> {
        ImageEntries::from(self).write_to(writer)
    }
}

impl From<&Image> for Vm {
    fn from(value: &Image) -> Self {
        Self::from_iter(value.entries())
    }
}

impl<R: Read> From<R> for Vm {
    fn from(value: R) -> Self {
        Self::from_iter(ImageEntries::from(value))
    }
}

impl<'a> FromIterator<ImageEntryRef<'a>> for Vm {
    fn from_iter<T: IntoIterator<Item = ImageEntryRef<'a>>>(iter: T) -> Self {
        let mut vm = Self::new();
        for entry in iter {
            vm.write_bytes(entry.address(), entry.data());
        }
        vm
    }
}
