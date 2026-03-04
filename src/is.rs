//! Instruction set for Alphabet's virtual machine.
//!
//! This module contains the logic to encode and decode
//! Alphabet machine instructions. The [`Operation`]
//! type provides useful constants for each instruction's
//! opcode, and the [`Instruction`] type provides a means
//! of creating or [decoding](Instruction::decode) an instruction
//! in a safe manner.
//!
//! The [`inst`] submodule provides functions for each individual
//! instruction to streamline creation of [`Instruction`] values.
//!
//! See [GitHub](https://github.com/adambyle/alphabet/blob/main/docs/instruction-set.md)
//! for more details on the instruction set.

use std::{error::Error, fmt::Display};

use crate::vm;

#[cfg(test)]
mod tests;

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

    /// Opcode for no operation.
    pub const NOOP_CODE: u8 = 0x00;
    /// No operation.
    pub const NOOP: Self = Self(Self::NOOP_CODE);

    // R-type.

    /// Opcode for signed and unsigned addition.
    pub const ADD_CODE: u8 = 0x01;
    /// Signed and unsigned addition.
    pub const ADD: Self = Self(Self::ADD_CODE);

    /// Opcode for signed and unsigned subtraction.
    pub const SUB_CODE: u8 = 0x02;
    /// Signed and unsigned subtraction.
    pub const SUB: Self = Self(Self::SUB_CODE);

    /// Opcode for logical bitshift left.
    pub const SHL_CODE: u8 = 0x03;
    /// Logical bitshift left.
    pub const SHL: Self = Self(Self::SHL_CODE);

    /// Opcode for logical bitshift right.
    pub const SHR_CODE: u8 = 0x04;
    /// Logical bitshift right.
    pub const SHR: Self = Self(Self::SHR_CODE);

    /// Opcode for arithmetic bitshift right.
    pub const SAR_CODE: u8 = 0x05;
    /// Arithmetic bitshift right.
    pub const SAR: Self = Self(Self::SAR_CODE);

    /// Opcode for bitwise and.
    pub const AND_CODE: u8 = 0x06;
    /// Bitwise and.
    pub const AND: Self = Self(Self::AND_CODE);

    /// Opcode for bitwise or.
    pub const OR_CODE: u8 = 0x08;
    /// Bitwise or.
    pub const OR: Self = Self(Self::OR_CODE);

    /// Opcode for bitwise exclusive-or.
    pub const XOR_CODE: u8 = 0x0A;
    /// Bitwise exclusive-or.
    pub const XOR: Self = Self(Self::XOR_CODE);

    /// Opcode for less-than comparison.
    pub const SLT_CODE: u8 = 0x0C;
    /// Less-than comparison.
    pub const SLT: Self = Self(Self::SLT_CODE);

    /// Opcode for less-than unsigned comparison.
    pub const SLTU_CODE: u8 = 0x0D;
    /// Less-than unsigned comparison.
    pub const SLTU: Self = Self(Self::SLTU_CODE);

    // I-type.

    /// Opcode for immediate unsigned value addition.
    pub const ADDI_CODE: u8 = 0x21;
    /// Immediate unsigned value addition.
    pub const ADDI: Self = Self(Self::ADDI_CODE);

    /// Opcode for immediate unsigned value subtraction.
    pub const SUBI_CODE: u8 = 0x22;
    /// Immediate unsigned value subtraction.
    pub const SUBI: Self = Self(Self::SUBI_CODE);

    /// Opcode for immediate logical bitshift left.
    pub const SHLI_CODE: u8 = 0x23;
    /// Immediate logical bitshift left.
    pub const SHLI: Self = Self(Self::SHLI_CODE);

    /// Opcode for immediate logical bitshift right.
    pub const SHRI_CODE: u8 = 0x24;
    /// Immediate logical bitshift right.
    pub const SHRI: Self = Self(Self::SHRI_CODE);

    /// Opcode for immediate larithmetic bitshift right.
    pub const SARI_CODE: u8 = 0x25;
    /// Immediate larithmetic bitshift right.
    pub const SARI: Self = Self(Self::SARI_CODE);

    /// Opcode for immediate bitwise and, lower 16 bits.
    pub const ANDI_CODE: u8 = 0x26;
    /// Immediate bitwise and, lower 16 bits.
    pub const ANDI: Self = Self(Self::ANDI_CODE);

    /// Opcode for immediate bitwise and, upper 16 bits.
    pub const ANDUI_CODE: u8 = 0x27;
    /// Immediate bitwise and, upper 16 bits.
    pub const ANDUI: Self = Self(Self::ANDUI_CODE);

    /// Opcode for immediate bitwise or, lower 16 bits.
    pub const ORI_CODE: u8 = 0x28;
    /// Immediate bitwise or, lower 16 bits.
    pub const ORI: Self = Self(Self::ORI_CODE);

    /// Opcode for immediate bitwise or, upper 16 bits.
    pub const ORUI_CODE: u8 = 0x29;
    /// Immediate bitwise or, upper 16 bits.
    pub const ORUI: Self = Self(Self::ORUI_CODE);

    /// Opcode for immediate bitwise exclusive-or, lower 16 bits.
    pub const XORI_CODE: u8 = 0x2A;
    /// Immediate bitwise exclusive-or, lower 16 bits.
    pub const XORI: Self = Self(Self::XORI_CODE);

    /// Opcode for immediate bitwise exclusive-or, upper 16 bits.
    pub const XORUI_CODE: u8 = 0x2B;
    /// Immediate bitwise exclusive-or, upper 16 bits.
    pub const XORUI: Self = Self(Self::XORUI_CODE);

    /// Opcode for less-than immediate signed comparison.
    pub const SLTI_CODE: u8 = 0x2C;
    /// Less-than immediate signed comparison.
    pub const SLTI: Self = Self(Self::SLTI_CODE);

    /// Opcode for less-than immediate unsigned comparison.
    pub const SLTUI_CODE: u8 = 0x2D;
    /// Less-than immediate unsigned comparison.
    pub const SLTUI: Self = Self(Self::SLTUI_CODE);

    /// Opcode for load word from memory.
    pub const LDW_CODE: u8 = 0x31;
    /// Load word from memory.
    pub const LDW: Self = Self(Self::LDW_CODE);

    /// Opcode for load half-word from memory.
    pub const LDHW_CODE: u8 = 0x32;
    /// Load half-word from memory.
    pub const LDHW: Self = Self(Self::LDHW_CODE);

    /// Opcode for load unsigned half-word from memory.
    pub const LDHWU_CODE: u8 = 0x33;
    /// Load unsigned half-word from memory.
    pub const LDHWU: Self = Self(Self::LDHWU_CODE);

    /// Opcode for load byte from memory.
    pub const LDB_CODE: u8 = 0x34;
    /// Load byte from memory.
    pub const LDB: Self = Self(Self::LDB_CODE);

    /// Opcode for load unsigned byte from memory.
    pub const LDBU_CODE: u8 = 0x35;
    /// Load unsigned byte from memory.
    pub const LDBU: Self = Self(Self::LDBU_CODE);

    /// Opcode for store word to memory.
    pub const STW_CODE: u8 = 0x36;
    /// Store word to memory.
    pub const STW: Self = Self(Self::STW_CODE);

    /// Opcode for store half-word to memory.
    pub const STHW_CODE: u8 = 0x37;
    /// Store half-word to memory.
    pub const STHW: Self = Self(Self::STHW_CODE);

    /// Opcode for store byte to memory.
    pub const STB_CODE: u8 = 0x38;
    /// Store byte to memory.
    pub const STB: Self = Self(Self::STB_CODE);

    /// Opcode for jump and link by offset.
    pub const JMP_CODE: u8 = 0x39;
    /// Jump and link by offset.
    pub const JMP: Self = Self(Self::JMP_CODE);

    /// Opcode for jump and link relative to register.
    pub const JMPR_CODE: u8 = 0x3A;
    /// Jump and link relative to register.
    pub const JMPR: Self = Self(Self::JMPR_CODE);

    /// Opcode for branch by offset if equal.
    pub const BEQ_CODE: u8 = 0x3B;
    /// Branch by offset if equal.
    pub const BEQ: Self = Self(Self::BEQ_CODE);

    /// Opcode for branch by offset if not equal.
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
        Some("beq"),   // 0x3B
        Some("bne"),   // 0x3C
        None,
        None,
        None,
    ];

    /// Iterate all valid operations in opcode order.
    pub fn all() -> impl Iterator<Item = Operation> {
        Operation::NAMES
            .iter()
            .enumerate()
            .filter_map(|(opcode, name)| name.and(Some(Operation(opcode as u8))))
    }

    /// Whether the given opcode represents a valid instruction.
    pub const fn is_valid_opcode(opcode: u8) -> bool {
        let opcode = opcode as usize;
        if opcode >= Operation::COUNT {
            return false;
        }
        Operation::NAMES[opcode].is_some()
    }

    /// Parse an operation from its opcode representation.
    ///
    /// Returns [`None`] if the opcode is invalid.
    pub const fn new(opcode: u8) -> Option<Self> {
        if Operation::is_valid_opcode(opcode) {
            Some(Operation(opcode))
        } else {
            None
        }
    }

    /// Parse an operation from its name.
    ///
    /// Return [`None`] if the name is invalid. Case-insensitive
    /// ASCII equality is used.
    pub fn parse(name: &str) -> Option<Self> {
        Operation::NAMES
            .iter()
            .position(|op_name| op_name.is_some_and(|op_name| op_name.eq_ignore_ascii_case(name)))
            .map(|opcode| Operation(opcode as u8))
    }

    /// The byte opcode for this operation.
    pub const fn opcode(&self) -> u8 {
        self.0
    }

    /// The name of the operation.
    pub const fn name(&self) -> &'static str {
        let name = Operation::NAMES[self.opcode() as usize];
        name.expect("valid opcode has no name")
    }

    /// The instruction encoding for this operation.
    pub const fn encoding(&self) -> Encoding {
        const COUNT: u8 = Operation::COUNT as u8;
        match self.opcode() {
            Operation::NOOP_CODE => Encoding::Noop,
            0x01..=0x1F => Encoding::RType,
            0x20..=0x3F => Encoding::IType,
            COUNT.. => unreachable!(),
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
/// Bitmask for a 6-bit instruction opcode.
pub const OPCODE_MASK: u32 = 0b111111;
/// Bitmask for a 16-bit immediate value.
pub const IMMEDIATE_MASK: u32 = 0xFFFF;

/// The provided register index is invalid.
#[derive(Debug, Clone, Copy)]
pub struct RegisterError(pub usize);

impl Display for RegisterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let index = self.0;
        write!(f, "register index {index} out of valid range")
    }
}

impl Error for RegisterError {}

/// There is no payload for a no-op instruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoopPayload;

/// R-type payload has 3 register fields.
#[derive(Debug, Clone, PartialEq, Eq)]
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
    ///
    /// # Errors
    ///
    /// This function returns an error if any provided
    /// register index is invalid.
    pub const fn new(
        register_r_index: usize,
        register_a_index: usize,
        register_b_index: usize,
    ) -> Result<Self, RegisterError> {
        if register_r_index >= vm::REGISTER_COUNT {
            return Err(RegisterError(register_r_index));
        }
        if register_a_index >= vm::REGISTER_COUNT {
            return Err(RegisterError(register_a_index));
        }
        if register_b_index >= vm::REGISTER_COUNT {
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
#[derive(Debug, Clone, PartialEq, Eq)]
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
    ///
    /// # Errors
    ///
    /// This function returns an error if any provided
    /// register index is invalid.
    pub const fn new(
        register_r_index: usize,
        register_a_index: usize,
        immediate_value: u16,
    ) -> Result<Self, RegisterError> {
        if register_r_index >= vm::REGISTER_COUNT {
            return Err(RegisterError(register_r_index));
        }
        if register_a_index >= vm::REGISTER_COUNT {
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Payload {
    /// Payload for the no-op instruction.
    Noop(NoopPayload),
    /// Payload for R-type instructions.
    RType(RTypePayload),
    /// Payload for I-type instructions.
    IType(ITypePayload),
}

impl Payload {
    /// Create no-op payload.
    pub const fn new_noop() -> Self {
        Self::Noop(NoopPayload)
    }

    /// Create R-type payload from data.
    ///
    /// # Errors
    ///
    /// This function returns an error if any provided
    /// register index is invalid.
    pub fn new_r_type(
        register_r_index: usize,
        register_a_index: usize,
        register_b_index: usize,
    ) -> Result<Self, RegisterError> {
        let payload = RTypePayload::new(register_r_index, register_a_index, register_b_index)?;
        Ok(Self::RType(payload))
    }

    /// Create I-type payload from data.
    ///
    /// # Errors
    ///
    /// This function returns an error if any provided
    /// register index is invalid.
    pub fn new_i_type(
        register_r_index: usize,
        register_a_index: usize,
        immediate_value: u16,
    ) -> Result<Self, RegisterError> {
        let payload = ITypePayload::new(register_r_index, register_a_index, immediate_value)?;
        Ok(Self::IType(payload))
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
#[derive(Debug, Clone)]
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
    InvalidOperation {
        /// The opcode that does not represent a valid operation.
        opcode: u8,
    },
}

impl InstructionError {
    /// Get the wrapped [`EncodingMismatch`](Self::EncodingMismatch)
    /// error data.
    pub fn encoding_error(&self) -> Option<(Operation, Encoding)> {
        if let Self::EncodingMismatch {
            operation,
            payload_encoding,
        } = *self
        {
            Some((operation, payload_encoding))
        } else {
            None
        }
    }

    /// Get the wrapped [`InvalidOperation`](Self::InvalidOperation)
    /// error data.
    pub fn operation_error(&self) -> Option<u8> {
        if let Self::InvalidOperation { opcode } = *self {
            Some(opcode)
        } else {
            None
        }
    }
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
///
/// The API enforces that all [`Instruction`] values
/// are valid.
///
/// You can create instructions with [`Instruction::new`],
/// decode them with [`Instruction::decode`], and create them
/// with dedicated functions in the [`inst`] submodule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instruction {
    operation: Operation,
    payload: Payload,
}

impl Instruction {
    /// The bit offset of the opcode field from the lowest bit.
    pub const OPCODE_OFFSET: usize = 26;

    /// Create an instruction from its operation and payload.
    ///
    /// # Errors
    ///
    /// This function returns an error when the encoding required
    /// by the operation does not match the encoding of the payload.
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
    ///
    /// # Errors
    ///
    /// This function returns an error when the encoding required
    /// by the operation does not match the encoding of the payload,
    /// or when the embedded opcode does not match a valid operation.
    pub fn decode(word: u32) -> Result<Self, InstructionError> {
        let opcode = ((word >> Self::OPCODE_OFFSET) & OPCODE_MASK) as u8;
        let operation =
            Operation::new(opcode).ok_or(InstructionError::InvalidOperation { opcode })?;
        let payload = match operation.encoding() {
            Encoding::Noop => Payload::new_noop(),
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

    /// Extract the payload of the instruction if it
    /// is an R-type instruction.
    pub const fn r_type_payload(&self) -> Option<&RTypePayload> {
        if let Payload::RType(ref payload) = self.payload {
            Some(payload)
        } else {
            None
        }
    }

    /// Extract the payload of the instruction if it
    /// is an I-type instruction.
    pub const fn i_type_payload(&self) -> Option<&ITypePayload> {
        if let Payload::IType(ref payload) = self.payload {
            Some(payload)
        } else {
            None
        }
    }

    /// Encode the instruction into its binary representation.
    pub const fn encode(&self) -> u32 {
        ((self.operation.opcode() as u32) << Self::OPCODE_OFFSET) | self.payload.encode()
    }
}

/// Preconfigured instructions.
///
/// This module provides functions for each machine instruction
/// to create [`Instruction`] values without having to check
/// for valid opcodes and register values using [`Instruction::new`]
/// and providing a slimmer way to express instruction payload.
///
/// Compare two ways to write the same instruction:
///
/// ```
/// # use alphabetvm::is::{Instruction, Operation, Payload, inst};
/// let jmp = Instruction::new(
///     Operation::JMP,
///     Payload::new_i_type(0, 0, 0x20).expect("invalid register indices"),
/// )
/// .expect("error creating instruction");
///
/// let jmp = inst::jmp(0, 0x20);
/// ```
///
/// # Panics
///
/// Any function in this module will panic if an invalid register index is provided.
pub mod inst {
    use super::*;

    /// A `noop` instruction.
    pub fn noop() -> Instruction {
        Instruction {
            operation: Operation::NOOP,
            payload: Payload::Noop(NoopPayload),
        }
    }

    fn r_type(op: Operation, r_r: usize, r_a: usize, r_b: usize) -> Instruction {
        let payload = RTypePayload::new(r_r, r_a, r_b).expect("invalid register indices");
        Instruction {
            operation: op,
            payload: Payload::RType(payload),
        }
    }

    fn i_type(op: Operation, r_r: usize, r_a: usize, imm: u16) -> Instruction {
        let payload = ITypePayload::new(r_r, r_a, imm).expect("invalid register indices");
        Instruction {
            operation: op,
            payload: Payload::IType(payload),
        }
    }

    /// An` add` instruction.
    pub fn add(register_r: usize, register_a: usize, register_b: usize) -> Instruction {
        r_type(Operation::ADD, register_r, register_a, register_b)
    }

    /// A `sub` instruction.
    pub fn sub(register_r: usize, register_a: usize, register_b: usize) -> Instruction {
        r_type(Operation::SUB, register_r, register_a, register_b)
    }

    /// A `shl` instruction.
    pub fn shl(register_r: usize, register_a: usize, register_b: usize) -> Instruction {
        r_type(Operation::SHL, register_r, register_a, register_b)
    }

    /// A `shr` instruction.
    pub fn shr(register_r: usize, register_a: usize, register_b: usize) -> Instruction {
        r_type(Operation::SHR, register_r, register_a, register_b)
    }

    /// A `sar` instruction.
    pub fn sar(register_r: usize, register_a: usize, register_b: usize) -> Instruction {
        r_type(Operation::SAR, register_r, register_a, register_b)
    }

    /// An` and` instruction.
    pub fn and(register_r: usize, register_a: usize, register_b: usize) -> Instruction {
        r_type(Operation::AND, register_r, register_a, register_b)
    }

    /// An` or` instruction.
    pub fn or(register_r: usize, register_a: usize, register_b: usize) -> Instruction {
        r_type(Operation::OR, register_r, register_a, register_b)
    }

    /// An` xor` instruction.
    pub fn xor(register_r: usize, register_a: usize, register_b: usize) -> Instruction {
        r_type(Operation::XOR, register_r, register_a, register_b)
    }

    /// A `slt` instruction.
    pub fn slt(register_r: usize, register_a: usize, register_b: usize) -> Instruction {
        r_type(Operation::SLT, register_r, register_a, register_b)
    }

    /// A `sltu` instruction.
    pub fn sltu(register_r: usize, register_a: usize, register_b: usize) -> Instruction {
        r_type(Operation::SLTU, register_r, register_a, register_b)
    }

    /// An` addi` instruction.
    pub fn addi(register_r: usize, register_a: usize, immediate_value: u16) -> Instruction {
        i_type(Operation::ADDI, register_r, register_a, immediate_value)
    }

    /// A `subi` instruction.
    pub fn subi(register_r: usize, register_a: usize, immediate_value: u16) -> Instruction {
        i_type(Operation::SUBI, register_r, register_a, immediate_value)
    }

    /// A `shli` instruction.
    pub fn shli(register_r: usize, register_a: usize, immediate_value: u16) -> Instruction {
        i_type(Operation::SHLI, register_r, register_a, immediate_value)
    }

    /// A `shri` instruction.
    pub fn shri(register_r: usize, register_a: usize, immediate_value: u16) -> Instruction {
        i_type(Operation::SHRI, register_r, register_a, immediate_value)
    }

    /// A `sari` instruction.
    pub fn sari(register_r: usize, register_a: usize, immediate_value: u16) -> Instruction {
        i_type(Operation::SARI, register_r, register_a, immediate_value)
    }

    /// An` andi` instruction.
    pub fn andi(register_r: usize, register_a: usize, immediate_value: u16) -> Instruction {
        i_type(Operation::ANDI, register_r, register_a, immediate_value)
    }

    /// An` andui` instruction.
    pub fn andui(register_r: usize, register_a: usize, immediate_value: u16) -> Instruction {
        i_type(Operation::ANDUI, register_r, register_a, immediate_value)
    }

    /// An` ori` instruction.
    pub fn ori(register_r: usize, register_a: usize, immediate_value: u16) -> Instruction {
        i_type(Operation::ORI, register_r, register_a, immediate_value)
    }

    /// An` orui` instruction.
    pub fn orui(register_r: usize, register_a: usize, immediate_value: u16) -> Instruction {
        i_type(Operation::ORUI, register_r, register_a, immediate_value)
    }

    /// An` xori` instruction.
    pub fn xori(register_r: usize, register_a: usize, immediate_value: u16) -> Instruction {
        i_type(Operation::XORI, register_r, register_a, immediate_value)
    }

    /// An` xorui` instruction.
    pub fn xorui(register_r: usize, register_a: usize, immediate_value: u16) -> Instruction {
        i_type(Operation::XORUI, register_r, register_a, immediate_value)
    }

    /// A `slti` instruction.
    pub fn slti(register_r: usize, register_a: usize, immediate_value: u16) -> Instruction {
        i_type(Operation::SLTI, register_r, register_a, immediate_value)
    }

    /// A `sltui` instruction.
    pub fn sltui(register_r: usize, register_a: usize, immediate_value: i16) -> Instruction {
        i_type(
            Operation::SLTUI,
            register_r,
            register_a,
            immediate_value as u16,
        )
    }

    /// A `ldw` instruction.
    pub fn ldw(register_r: usize, register_a: usize, offset: i16) -> Instruction {
        i_type(Operation::LDW, register_r, register_a, offset as u16)
    }

    /// A `ldhw` instruction.
    pub fn ldhw(register_r: usize, register_a: usize, offset: i16) -> Instruction {
        i_type(Operation::LDHW, register_r, register_a, offset as u16)
    }

    /// A `ldhwu` instruction.
    pub fn ldhwu(register_r: usize, register_a: usize, offset: i16) -> Instruction {
        i_type(Operation::LDHWU, register_r, register_a, offset as u16)
    }

    /// A `ldb` instruction.
    pub fn ldb(register_r: usize, register_a: usize, offset: i16) -> Instruction {
        i_type(Operation::LDB, register_r, register_a, offset as u16)
    }

    /// A `ldbu` instruction.
    pub fn ldbu(register_r: usize, register_a: usize, offset: i16) -> Instruction {
        i_type(Operation::LDBU, register_r, register_a, offset as u16)
    }

    /// A `stw` instruction.
    pub fn stw(register_s: usize, register_a: usize, offset: i16) -> Instruction {
        i_type(Operation::STW, register_s, register_a, offset as u16)
    }

    /// A `sthw` instruction.
    pub fn sthw(register_s: usize, register_a: usize, offset: i16) -> Instruction {
        i_type(Operation::STHW, register_s, register_a, offset as u16)
    }

    /// A `stb` instruction.
    pub fn stb(register_s: usize, register_a: usize, offset: i16) -> Instruction {
        i_type(Operation::STB, register_s, register_a, offset as u16)
    }

    /// A `jmp` instruction.
    pub fn jmp(register_r: usize, offset: i16) -> Instruction {
        i_type(Operation::JMP, register_r, 0, offset as u16)
    }

    /// A `jmpr` instruction.
    pub fn jmpr(register_r: usize, register_a: usize, offset: i16) -> Instruction {
        i_type(Operation::JMPR, register_r, register_a, offset as u16)
    }

    /// A `beq` instruction.
    pub fn beq(register_a: usize, register_b: usize, offset: i16) -> Instruction {
        i_type(Operation::BEQ, register_a, register_b, offset as u16)
    }

    /// A `bne` instruction.
    pub fn bne(register_a: usize, register_b: usize, offset: i16) -> Instruction {
        i_type(Operation::BNE, register_a, register_b, offset as u16)
    }
}
