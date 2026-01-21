use std::collections::HashMap;

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

type MemoryBlockBytes = [u8; BLOCK_SIZE];
type MemoryBlockFlags = [bool; BLOCK_SIZE];

/// A minimal virtual I/O device controller.
pub trait IoController {
    /// Allow I/O device to update relevant state.
    fn tick(&mut self);

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
            Self::Memory(memory) => [
                memory.read_byte(offset + 0),
                memory.read_byte(offset + 1),
            ],
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
        if offset > u16::MAX - 1 {
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

    /// Reset the virtual machine's
    /// program counter, registers,
    /// and memory.
    pub fn reset(&mut self) {
        self.program_counter = 0;
        self.registers.fill(0);
        self.blocks.clear();
    }

    /// Get the word address of the next
    /// instruction to execute.
    pub fn program_counter(&self) -> u32 {
        self.program_counter
    }

    /// Set the program counter to the
    /// specified word address.
    pub fn seek(&mut self, address: u32) {
        self.program_counter = address % MAX_WORD_ADDRESS;
    }

    /// Get the value of the specified register.
    pub fn read_register(&self, index: usize) -> u32 {
        // r0 should always hold value 0.
        self.registers[index % REGISTER_COUNT]
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

    /// Notify all I/O devices that they may update state.
    pub fn tick_io_devices(&mut self) {
        for i in &self.controller_block_keys {
            let Some(Block::Io(controller)) = self.blocks.get_mut(i) else {
                panic!("I/O controller not found at block index {i}");
            };
            controller.tick();
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

    /// Write a word of data to virtual memory.
    pub fn decode_and_execute(&mut self, instruction: u32) -> InstructionResult {
        todo!()
    }

    /// Tick I/O devices, run the next instruction,
    /// and advance the program counter.
    pub fn step_forward(&mut self) {
        self.tick_io_devices();
        let instruction = self.read_word(self.program_counter * 4);
        let result = self.decode_and_execute(instruction);
        if !result.jumped {
            self.program_counter = self.program_counter.wrapping_add(1);
        }
    }
}
