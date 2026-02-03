//! Instruction set for Alphabet's virtual machine,
//! including instruction decoding logic.

use std::{error::Error, fmt::Display};

use crate::vm::REGISTER_COUNT;

/// A kind of instruction encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encoding {
    /// The encoding of the no-op instruction has no fields.
    Noop,
    /// The encoding of the R-type instruction has 3 register fields.
    RType,
    /// The encoding of the I-type instruction has 2 register fields
    /// and an immediate value field.
    IType,
}

/// A type of instruction (such as `add` or `sub`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Operation(u8);

impl Operation {
    /// How many possible opcodes there could be.
    pub const COUNT: usize = 1 << 6;

    pub const NOOP_CODE: u8 = 0x00;
    /// No operation.
    pub const NOOP: Self = Self(Self::NOOP_CODE);

    // R-type.

    pub const ADD_CODE: u8 = 0x01;
    /// Signed and unsigned addition.
    pub const ADD: Self = Self(Self::ADD_CODE);

    pub const SUB_CODE: u8 = 0x02;
    /// Signed and unsigned subtraction.
    pub const SUB: Self = Self(Self::SUB_CODE);

    pub const SHL_CODE: u8 = 0x03;
    /// Logical bitshift left.
    pub const SHL: Self = Self(Self::SHL_CODE);

    pub const SHR_CODE: u8 = 0x04;
    /// Logical bitshift right.
    pub const SHR: Self = Self(Self::SHR_CODE);

    pub const SAR_CODE: u8 = 0x05;
    /// Arithmetic bitshift right.
    pub const SAR: Self = Self(Self::SAR_CODE);

    pub const AND_CODE: u8 = 0x06;
    /// Bitwise and.
    pub const AND: Self = Self(Self::AND_CODE);

    pub const OR_CODE: u8 = 0x08;
    /// Bitwise or.
    pub const OR: Self = Self(Self::OR_CODE);

    pub const XOR_CODE: u8 = 0x0A;
    /// Bitwise exclusive-or.
    pub const XOR: Self = Self(Self::XOR_CODE);

    pub const SLT_CODE: u8 = 0x0C;
    /// Less-than comparison.
    pub const SLT: Self = Self(Self::SLT_CODE);

    pub const SLTU_CODE: u8 = 0x0D;
    /// Less-than unsigned comparison.
    pub const SLTU: Self = Self(Self::SLTU_CODE);

    // I-type.

    pub const ADDI_CODE: u8 = 0x21;
    /// Immediate unsigned value addition.
    pub const ADDI: Self = Self(Self::ADDI_CODE);

    pub const SUBI_CODE: u8 = 0x22;
    /// Immediate unsigned value subtraction.
    pub const SUBI: Self = Self(Self::SUBI_CODE);

    pub const SHLI_CODE: u8 = 0x23;
    /// Immediate logical bitshift left.
    pub const SHLI: Self = Self(Self::SHLI_CODE);

    pub const SHRI_CODE: u8 = 0x24;
    /// Immediate logical bitshift right.
    pub const SHRI: Self = Self(Self::SHRI_CODE);

    pub const SARI_CODE: u8 = 0x25;
    /// Immediate larithmetic bitshift right.
    pub const SARI: Self = Self(Self::SARI_CODE);

    pub const ANDI_CODE: u8 = 0x26;
    /// Immediate bitwise and, lower 16 bits.
    pub const ANDI: Self = Self(Self::ANDI_CODE);

    pub const ANDUI_CODE: u8 = 0x27;
    /// Immediate bitwise and, upper 16 bits.
    pub const ANDUI: Self = Self(Self::ANDUI_CODE);

    pub const ORI_CODE: u8 = 0x28;
    /// Immediate bitwise or, lower 16 bits.
    pub const ORI: Self = Self(Self::ORI_CODE);

    pub const ORUI_CODE: u8 = 0x29;
    /// Immediate bitwise or, upper 16 bits.
    pub const ORUI: Self = Self(Self::ORUI_CODE);

    pub const XORI_CODE: u8 = 0x2A;
    /// Immediate bitwise exclusive-or, lower 16 bits.
    pub const XORI: Self = Self(Self::XORI_CODE);

    pub const XORUI_CODE: u8 = 0x2B;
    /// Immediate bitwise exclusive-or, upper 16 bits.
    pub const XORUI: Self = Self(Self::XORUI_CODE);

    pub const SLTI_CODE: u8 = 0x2C;
    /// Less-than immediate signed comparison.
    pub const SLTI: Self = Self(Self::SLTI_CODE);

    pub const SLTUI_CODE: u8 = 0x2D;
    /// Less-than immediate unsigned comparison.
    pub const SLTUI: Self = Self(Self::SLTUI_CODE);

    pub const LDW_CODE: u8 = 0x31;
    /// Load word from memory.
    pub const LDW: Self = Self(Self::LDW_CODE);

    pub const LDHW_CODE: u8 = 0x32;
    /// Load half-word from memory.
    pub const LDHW: Self = Self(Self::LDHW_CODE);

    pub const LDHWU_CODE: u8 = 0x33;
    /// Load unsigned half-word from memory.
    pub const LDHWU: Self = Self(Self::LDHWU_CODE);

    pub const LDB_CODE: u8 = 0x34;
    /// Load byte from memory.
    pub const LDB: Self = Self(Self::LDB_CODE);

    pub const LDBU_CODE: u8 = 0x35;
    /// Load unsigned byte from memory.
    pub const LDBU: Self = Self(Self::LDBU_CODE);

    pub const STW_CODE: u8 = 0x36;
    /// Store word to memory.
    pub const STW: Self = Self(Self::STW_CODE);

    pub const STHW_CODE: u8 = 0x37;
    /// Store half-word to memory.
    pub const STHW: Self = Self(Self::STHW_CODE);

    pub const STB_CODE: u8 = 0x38;
    /// Store byte to memory.
    pub const STB: Self = Self(Self::STB_CODE);

    pub const JMP_CODE: u8 = 0x39;
    /// Jump and link by offset.
    pub const JMP: Self = Self(Self::JMP_CODE);

    pub const JMPR_CODE: u8 = 0x3A;
    /// Jump and link relative to register.
    pub const JMPR: Self = Self(Self::JMPR_CODE);

    pub const BEQ_CODE: u8 = 0x3B;
    /// Branch by offset if equal.
    pub const BEQ: Self = Self(Self::BEQ_CODE);

    pub const BNE_CODE: u8 = 0x3C;
    /// Branch by offset if not equal.
    pub const BNE: Self = Self(Self::BNE_CODE);

    /// Lowercase names of instructions, indexed
    /// by opcode.
    const NAMES: [Option<&'static str>; Self::COUNT] = [
        Some("noop"), // 0x00
        Some("add"),  // 0x01
        Some("sub"),  // 0x02
        Some("shl"),  // 0x03
        Some("shr"),  // 0x04
        Some("sar"),  // 0x05
        Some("and"),  // 0x06
        None,
        Some("or"), // 0x08
        None,
        Some("xor"), // 0x0A
        None,
        Some("slt"),  // 0x0C
        Some("sltu"), // 0x0D
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        Some("addi"),  // 0x21
        Some("subi"),  // 0x22
        Some("shli"),  // 0x23
        Some("shri"),  // 0x24
        Some("sari"),  // 0x25
        Some("andi"),  // 0x26
        Some("andui"), // 0x27
        Some("ori"),   // 0x28
        Some("orui"),  // 0x29
        Some("xori"),  // 0x2A
        Some("xorui"), // 0x2B
        Some("slti"),  // 0x2C
        Some("sltui"), // 0x2D
        None,
        None,
        None,
        Some("ldw"),   // 0x31
        Some("ldhw"),  // 0x32
        Some("ldhwu"), // 0x33
        Some("ldb"),   // 0x34
        Some("ldbu"),  // 0x35
        Some("stw"),   // 0x36
        Some("sthw"),  // 0x37
        Some("stb"),   // 0x38
        Some("jmp"),   // 0x39
        Some("jmpr"),  // 0x3A
        Some("bwe"),   // 0x3B
        Some("bne"),   // 0x3C
        None,
        None,
        None,
    ];

    /// Whether the given opcode represents a valid instruction.
    pub const fn is_valid_opcode(opcode: u8) -> bool {
        let opcode = opcode as usize;
        if opcode >= Self::COUNT {
            return false;
        }
        Self::NAMES[opcode].is_some()
    }

    /// Parse an operation from its opcode representation.
    ///
    /// Returns [`None`] if the opcode is invalid.
    pub const fn new(opcode: u8) -> Option<Self> {
        if Self::is_valid_opcode(opcode) {
            Some(Self(opcode))
        } else {
            None
        }
    }

    /// Parse an operation from its name.
    ///
    /// Return [`None`] if the name is invalid.
    pub fn parse(name: &str) -> Option<Self> {
        for (opcode, op_name) in Self::NAMES.iter().enumerate() {
            if op_name.is_some_and(|n| n == name) {
                return Some(Self(opcode as u8));
            }
        }
        None
    }

    /// The byte opcode for this operation.
    pub const fn opcode(&self) -> u8 {
        self.0
    }

    /// The name of the operation.
    pub const fn name(&self) -> &'static str {
        let name = Self::NAMES[self.opcode() as usize];
        name.expect("valid opcode has no name")
    }

    /// The instruction encoding for this operation.
    pub const fn encoding(&self) -> Encoding {
        match self.opcode() {
            Self::NOOP_CODE => Encoding::Noop,
            0x01..=0x1F => Encoding::RType,
            0x20..=0x2F => Encoding::IType,
            _ => unreachable!(),
        }
    }
}

impl Display for Operation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = self.name();
        write!(f, "{name}")
    }
}

/// Bitmask for a 5-bit register index.
pub const REGISTER_MASK: u32 = 0b11111;
/// Bitmask for an instruction opcode.
pub const OPCODE_MASK: u32 = 0b111111;
/// Bitmask for a 16-bit immediate value.
pub const IMMEDIATE_MASK: u32 = 0xFFFF;

/// The provided register index is invalid.
#[derive(Debug)]
pub struct RegisterError(usize);

impl Display for RegisterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let index = self.0;
        write!(f, "register index {index} out of valid range")
    }
}

impl Error for RegisterError {}

/// There is no payload for a no-op instruction.
#[derive(PartialEq, Eq)]
pub struct NoopPayload;

/// R-type payload has 3 register fields.
#[derive(PartialEq, Eq)]
pub struct RTypePayload {
    register_r: usize,
    register_a: usize,
    register_b: usize,
}

impl RTypePayload {
    /// The bit offset of the register R field from the lowest bit.
    pub const REGISTER_R_OFFSET: usize = 21;
    /// The bit offset of the register A field from the lowest bit.
    pub const REGISTER_A_OFFSET: usize = 16;
    /// The bit offset of the register B field from the lowest bit.
    pub const REGISTER_B_OFFSET: usize = 11;

    /// Create R-type payload from data.
    pub const fn new(
        register_r_index: usize,
        register_a_index: usize,
        register_b_index: usize,
    ) -> Result<Self, RegisterError> {
        if register_r_index > REGISTER_COUNT {
            return Err(RegisterError(register_r_index));
        }
        if register_a_index > REGISTER_COUNT {
            return Err(RegisterError(register_a_index));
        }
        if register_b_index > REGISTER_COUNT {
            return Err(RegisterError(register_b_index));
        }

        Ok(Self {
            register_r: register_r_index,
            register_a: register_a_index,
            register_b: register_b_index,
        })
    }

    /// Decode the R-type payload from the binary represetation of an instruction.
    pub const fn decode(word: u32) -> Self {
        let register_r = ((word >> Self::REGISTER_R_OFFSET) & REGISTER_MASK) as usize;
        let register_a = ((word >> Self::REGISTER_A_OFFSET) & REGISTER_MASK) as usize;
        let register_b = ((word >> Self::REGISTER_B_OFFSET) & REGISTER_MASK) as usize;

        Self {
            register_r,
            register_a,
            register_b,
        }
    }

    /// The index of the R register.
    pub const fn register_r_index(&self) -> usize {
        self.register_r
    }

    /// The index of the A register.
    pub const fn register_a_index(&self) -> usize {
        self.register_a
    }

    /// The index of the B register.
    pub const fn register_b_index(&self) -> usize {
        self.register_b
    }

    /// Encode this payload into the lower part of the
    /// binary representation of an instruction.
    pub const fn encode(&self) -> u32 {
        ((self.register_r as u32) << Self::REGISTER_R_OFFSET)
            | ((self.register_a as u32) << Self::REGISTER_A_OFFSET)
            | ((self.register_b as u32) << Self::REGISTER_B_OFFSET)
    }
}

/// I-type payload has 2 register fields and
/// an immediate value field.
#[derive(PartialEq, Eq)]
pub struct ITypePayload {
    register_r: usize,
    register_a: usize,
    immediate: u16,
}

impl ITypePayload {
    /// The bit offset of the register R field from the lowest bit.
    pub const REGISTER_R_OFFSET: usize = 21;
    /// The bit offset of the register A field from the lowest bit.
    pub const REGISTER_A_OFFSET: usize = 16;
    /// The bit offset of the immediate value field from the lowest bit.
    pub const IMMEDIATE_OFFSET: usize = 0;

    /// Create I-type payload from data.
    pub const fn new(
        register_r_index: usize,
        register_a_index: usize,
        immediate_value: u16,
    ) -> Result<Self, RegisterError> {
        if register_r_index > REGISTER_COUNT {
            return Err(RegisterError(register_r_index));
        }
        if register_a_index > REGISTER_COUNT {
            return Err(RegisterError(register_a_index));
        }

        Ok(Self {
            register_r: register_r_index,
            register_a: register_a_index,
            immediate: immediate_value,
        })
    }

    /// Decode the I-type payload from the binary represetation of an instruction.
    pub const fn decode(word: u32) -> Self {
        let register_r = ((word >> Self::REGISTER_R_OFFSET) & REGISTER_MASK) as usize;
        let register_a = ((word >> Self::REGISTER_A_OFFSET) & REGISTER_MASK) as usize;
        let immediate = ((word >> Self::IMMEDIATE_OFFSET) & IMMEDIATE_MASK) as u16;

        Self {
            register_r,
            register_a,
            immediate,
        }
    }

    /// The index of the R register.
    pub const fn register_r_index(&self) -> usize {
        self.register_r
    }

    /// The index of the A register.
    pub const fn register_a_index(&self) -> usize {
        self.register_a
    }

    /// The embedded immediate value.
    pub const fn immediate_value(&self) -> u16 {
        self.immediate
    }

    /// Encode this payload into the lower part of the
    /// binary representation of an instruction.
    pub const fn encode(&self) -> u32 {
        ((self.register_r as u32) << Self::REGISTER_R_OFFSET)
            | ((self.register_a as u32) << Self::REGISTER_A_OFFSET)
            | ((self.immediate as u32) << Self::IMMEDIATE_OFFSET)
    }
}

/// Instruction payload (fields and data).
#[derive(PartialEq, Eq)]
pub enum Payload {
    /// Payload for the no-op instruction.
    Noop(NoopPayload),
    /// Payload for R-type instructions.
    RType(RTypePayload),
    /// Payload for I-type instructions.
    IType(ITypePayload),
}

impl Payload {
    /// No-op payload.
    pub const fn noop() -> Self {
        Self::Noop(NoopPayload)
    }

    /// The encoding of this kind of payload.
    pub const fn encoding(&self) -> Encoding {
        match self {
            Self::Noop(_) => Encoding::Noop,
            Self::RType(_) => Encoding::RType,
            Self::IType(_) => Encoding::IType,
        }
    }

    /// Encode this payload into the lower part of the
    /// binary representation of an instruction.
    pub const fn encode(&self) -> u32 {
        match self {
            Self::Noop(_) => 0x00000000,
            Self::RType(payload) => payload.encode(),
            Self::IType(payload) => payload.encode(),
        }
    }
}

/// A problem creating an instruction.
#[derive(Debug)]
pub enum InstructionError {
    /// The encoding required by the operation and
    /// the encoding of the provided payload did
    /// not match.
    EncodingMismatch {
        /// The operation.
        operation: Operation,
        /// The payload provided for the instruction.
        payload_encoding: Encoding,
    },
    /// The opcode in the encoded instruction did
    /// not represent a valid operation.
    InvalidOperation { opcode: u8 },
}

impl Display for InstructionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EncodingMismatch {
                operation,
                payload_encoding,
            } => {
                let operation_encoding = operation.encoding();
                write!(
                    f,
                    "{operation_encoding:?} encoding expected for operation '{operation}' did not \
                    match {payload_encoding:?} payload encoding",
                )
            }
            Self::InvalidOperation { opcode } => {
                write!(f, "invalid opcode {opcode:02X}")
            }
        }
    }
}

impl Error for InstructionError {}

/// A machine instruction.
pub struct Instruction {
    operation: Operation,
    payload: Payload,
}

impl Instruction {
    /// The bit offset of the opcode field from the lowest bit.
    pub const OPCODE_OFFSET: usize = 26;

    /// Create an instruction from its operation and payload.
    pub fn new(operation: Operation, payload: Payload) -> Result<Self, InstructionError> {
        let payload_encoding = payload.encoding();
        if operation.encoding() != payload_encoding {
            return Err(InstructionError::EncodingMismatch {
                operation,
                payload_encoding,
            });
        }
        Ok(Self { operation, payload })
    }

    /// Decode an instruction from its binary representation.
    pub fn decode(word: u32) -> Result<Self, InstructionError> {
        let opcode = ((word >> Self::OPCODE_OFFSET) & OPCODE_MASK) as u8;
        let operation =
            Operation::new(opcode).ok_or(InstructionError::InvalidOperation { opcode })?;
        let payload = match operation.encoding() {
            Encoding::Noop => Payload::noop(),
            Encoding::RType => Payload::RType(RTypePayload::decode(word)),
            Encoding::IType => Payload::IType(ITypePayload::decode(word)),
        };
        Self::new(operation, payload)
    }

    /// The encoding type of the instruction.
    pub const fn encoding(&self) -> Encoding {
        self.operation.encoding()
    }

    /// The operation of the instruction.
    pub const fn operation(&self) -> Operation {
        self.operation
    }

    /// The payload of the instruction.
    pub const fn payload(&self) -> &Payload {
        &self.payload
    }

    /// Encode the instruction into its binary representation.
    pub const fn encode(&self) -> u32 {
        ((self.operation.opcode() as u32) << Self::OPCODE_OFFSET) | self.payload.encode()
    }
}
