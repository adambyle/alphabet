//! Interface to Alphabet's virtual machine.

use std::{collections::HashMap, io::Read};

use crate::is::{is_op_r_type, op::*, Instruction};

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
type MemoryBlockBytes = [u8; BLOCK_SIZE];

/// Flags marking dirty bits on byte
/// locations in a memory block.
type MemoryBlockFlags = [bool; BLOCK_SIZE];

/// A minimal virtual I/O device controller.
pub trait IoController {
    /// Read a byte of data from the controller.
    fn read_byte(&mut self, offset: u16) -> u8;

    /// Write a byte of data to the controller.
    fn write_byte(&mut self, offset: u16, byte: u8);

    /// Returns which addresses are readable for this device.
    fn readable_addresses(&self) -> MemoryBlockFlags;

    /// Return which addresses are writable for this device.
    fn writeable_addresses(&self) -> MemoryBlockFlags;
}

/// A block of readable and writeable arbitrary memory.
pub struct MemoryBlock {
    dirt: Option<MemoryBlockFlags>,
    data: MemoryBlockBytes,
}

impl MemoryBlock {
    /// Read all the data from this memory block.
    pub fn read_all(&mut self) -> MemoryBlockBytes {
        self.dirt = None;
        self.data
    }

    /// Overwrite all the data in the memory block.
    pub fn write_all(&mut self, data: MemoryBlockBytes, dirty: bool) {
        self.data = data;
        if dirty {
            self.dirt = Some([true; BLOCK_SIZE]);
        }
    }

    fn read_byte(&mut self, offset: u16) -> u8 {
        let offset = offset as usize;
        if let Some(ref mut flags) = self.dirt {
            flags[offset] = false;
        }
        self.data[offset]
    }

    fn write_byte(&mut self, offset: u16, byte: u8) {
        let offset = offset as usize;
        self.data[offset] = byte;

        let flags = self.dirt.get_or_insert([false; BLOCK_SIZE]);
        flags[offset] = true;
    }
}

/// A block of byte-addressed virtual machine memory.
pub enum Block {
    /// Readable and writable memory.
    Memory(MemoryBlock),

    /// A memory-mapped I/O device.
    Io(Box<dyn IoController>),
}

impl Block {
    /// Create a new zeroed memory block.
    pub fn new_memory() -> Self {
        Self::Memory(MemoryBlock {
            dirt: None,
            data: [0; BLOCK_SIZE],
        })
    }

    /// Create a new memory block containing the specified data.
    pub fn with_data(data: MemoryBlockBytes) -> Self {
        Self::Memory(MemoryBlock { dirt: None, data })
    }

    /// Create a new block mapped to an I/O controller.
    pub fn new_controller(controller: Box<dyn IoController>) -> Self {
        Self::Io(controller)
    }

    /// Read a byte of data from the block.
    pub fn read_byte(&mut self, offset: u16) -> u8 {
        match self {
            Self::Memory(memory) => memory.read_byte(offset),
            Self::Io(controller) => controller.read_byte(offset),
        }
    }

    /// Read a half-word of data from the block.
    pub fn read_half_word(&mut self, offset: u16) -> u16 {
        if offset > u16::MAX - 1 {
            return 0;
        }
        let bytes = match self {
            Self::Memory(memory) => [memory.read_byte(offset + 0), memory.read_byte(offset + 1)],
            Self::Io(controller) => [
                controller.read_byte(offset + 0),
                controller.read_byte(offset + 1),
            ],
        };
        u16::from_be_bytes(bytes)
    }

    /// Read a word of data from the block.
    pub fn read_word(&mut self, offset: u16) -> u32 {
        if offset > u16::MAX - 3 {
            return 0;
        }
        let bytes = match self {
            Self::Memory(memory) => [
                memory.read_byte(offset + 0),
                memory.read_byte(offset + 1),
                memory.read_byte(offset + 2),
                memory.read_byte(offset + 3),
            ],
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
            Self::Memory(memory) => {
                memory.write_byte(offset, data);
            }
            Self::Io(controller) => {
                controller.write_byte(offset, data);
            }
        }
    }

    /// Write a half-word of data to the block.
    pub fn write_half_word(&mut self, offset: u16, data: u16) {
        if offset > u16::MAX - 1 {
            return;
        }
        let bytes = data.to_be_bytes();
        match self {
            Self::Memory(memory) => {
                memory.write_byte(offset + 0, bytes[0]);
                memory.write_byte(offset + 1, bytes[1]);
            }
            Self::Io(controller) => {
                controller.write_byte(offset + 0, bytes[0]);
                controller.write_byte(offset + 1, bytes[1]);
            }
        };
    }

    /// Write a word of data to the block.
    pub fn write_word(&mut self, offset: u16, data: u32) {
        if offset > u16::MAX - 3 {
            return;
        }
        let bytes = data.to_be_bytes();
        match self {
            Self::Memory(memory) => {
                memory.write_byte(offset + 0, bytes[0]);
                memory.write_byte(offset + 1, bytes[1]);
                memory.write_byte(offset + 2, bytes[2]);
                memory.write_byte(offset + 3, bytes[3]);
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
    blocks: HashMap<u16, Block>,

    // Cache for indices of I/O controller blocks.
    controller_block_keys: Vec<u16>,
}

impl Vm {
    /// Create a new virtual machine.
    pub fn new() -> Self {
        Self {
            program_counter: 0,
            registers: [0; REGISTER_COUNT],
            blocks: HashMap::new(),
            controller_block_keys: Vec::new(),
        }
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
            vm.blocks.insert(entry.block_index, block);
        }
        vm
    }

    /// Generate an image, a snapshot of the VM's
    /// state.
    pub fn image(&self) -> Image {
        let mut image = Image {
            entries: Vec::new(),
        };
        for (index, block) in self.blocks.values().enumerate() {
            let Block::Memory(mem) = block else {
                continue;
            };
            let start_offset = mem.data.iter().position(|&b| b != 0);
            let Some(start_offset) = start_offset else {
                continue;
            };

            // Unwrap: guaranteed to be at least offset.
            let end_offset = BLOCK_SIZE - 1 - mem.data.iter().rev().position(|&b| b != 0).unwrap();

            let data = mem.data[start_offset..=end_offset].to_vec();
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

    /// Reset the virtual machine's
    /// program counter, registers,
    /// and memory.
    pub fn reset(&mut self) {
        self.program_counter = 0;
        self.registers.fill(0);
        self.blocks.clear();
    }

    /// Restart the program, resettings
    /// the virtual machine's program counter
    /// and registers.
    pub fn restart(&mut self) {
        self.program_counter = 0;
        self.registers.fill(0);
    }

    /// Get the word address of the next
    /// instruction to execute.
    pub fn program_counter(&self) -> u32 {
        self.program_counter
    }

    /// Set the program counter to the
    /// specified word address.
    pub fn seek(&mut self, address: u32) {
        self.program_counter = address & MAX_WORD_ADDRESS;
    }

    /// Get the value of the specified register.
    pub fn read_register(&self, index: usize) -> u32 {
        // r0 should always hold value 0.
        self.registers[index % REGISTER_COUNT]
    }

    /// Read the value of all regiters.
    pub fn read_registers(&self) -> [u32; REGISTER_COUNT] {
        self.registers
    }

    /// Set the value of the specified register.
    pub fn write_register(&mut self, index: usize, word: u32) {
        let index = index % REGISTER_COUNT;
        if index == 0 {
            return;
        }
        self.registers[index] = word;
    }

    fn remove_controller_block_key(&mut self, key: u16) {
        self.controller_block_keys.retain(|&i| i != key);
    }

    /// Reset the specified block, detaching any
    /// associated I/O device.
    pub fn clear_block(&mut self, block_index: u16) {
        self.blocks.remove(&block_index);
        self.remove_controller_block_key(block_index);
    }

    /// Map an I/O device to a block of memory.
    pub fn map_io_controller(&mut self, block_index: u16, controller: Box<dyn IoController>) {
        self.blocks
            .insert(block_index, Block::new_controller(controller));
        if !self.controller_block_keys.contains(&block_index) {
            self.controller_block_keys.push(block_index);
        }
    }

    fn block(&mut self, address: u32) -> Option<&mut Block> {
        let block_index = (address >> 16) as u16;
        self.blocks.get_mut(&block_index)
    }

    fn block_create(&mut self, address: u32) -> &mut Block {
        let block_index = (address >> 16) as u16;
        self.blocks
            .entry(block_index)
            .or_insert(Block::new_memory())
    }

    /// Read a byte of data from virtual memory.
    pub fn read_byte(&mut self, address: u32) -> u8 {
        let Some(block) = self.block(address) else {
            return 0;
        };
        let offset = address as u16;
        block.read_byte(offset)
    }

    /// Read a half-word of data from virtual memory.
    pub fn read_half_word(&mut self, address: u32) -> u16 {
        let Some(block) = self.block(address) else {
            return 0;
        };
        let offset = address as u16;
        block.read_half_word(offset)
    }

    /// Read a word of data from virtual memory.
    pub fn read_word(&mut self, address: u32) -> u32 {
        let Some(block) = self.block(address) else {
            return 0;
        };
        let offset = address as u16;
        block.read_word(offset)
    }

    /// Get or create a block at the specified index.
    ///
    /// If it doesn't exist, an empty memory block is created.
    pub fn get_block(&mut self, block_index: u16) -> &mut Block {
        self.blocks
            .entry(block_index)
            .or_insert(Block::new_memory())
    }

    /// Write a byte of data to virtual memory.
    pub fn write_byte(&mut self, address: u32, data: u8) {
        let block = self.block_create(address);
        let offset = address as u16;
        block.write_byte(offset, data);
    }

    /// Write a half-word of data to virtual memory.
    pub fn write_half_word(&mut self, address: u32, data: u16) {
        let block = self.block_create(address);
        let offset = address as u16;
        block.write_half_word(offset, data);
    }

    /// Write a word of data to virtual memory.
    pub fn write_word(&mut self, address: u32, data: u32) {
        let block = self.block_create(address);
        let offset = address as u16;
        block.write_word(offset, data);
    }

    /// Write a whole block of data, detaching any
    /// I/O device associated with that block.
    pub fn write_whole_block(&mut self, block_index: u16, data: MemoryBlockBytes) {
        // Detach I/O device.
        let is_io = self
            .blocks
            .get(&block_index)
            .is_some_and(|b| matches!(b, Block::Io(_)));
        if is_io {
            self.remove_controller_block_key(block_index);
        }

        let block = Block::with_data(data);
        self.blocks.insert(block_index, block);
    }

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
                self.program_counter =
                    self.program_counter.wrapping_add_signed(imm as i16 as i32) & MAX_WORD_ADDRESS;
                jumped = true;
                Some(ret)
            }
            JMPR => {
                let ret = (self.program_counter + 1) & MAX_WORD_ADDRESS;
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
                    jumped = true;
                }
                None
            }
            BNE => {
                if r_src != r_op {
                    self.program_counter =
                        self.program_counter.wrapping_add_signed(imm as i16 as i32)
                            & MAX_WORD_ADDRESS;
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
            self.write_register(payload.r_result, result);
        }
        InstructionResult {
            valid: true,
            jumped,
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

    /// Tick I/O devices, run the next instruction,
    /// and advance the program counter.
    pub fn step_forward(&mut self) {
        let instruction = self.read_word(self.program_counter * 4);
        let instruction = Instruction::decode(instruction);
        let result = self.execute(instruction);
        if !result.jumped {
            self.program_counter = (self.program_counter + 1) & MAX_WORD_ADDRESS;
        }
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
        }
    }
}
