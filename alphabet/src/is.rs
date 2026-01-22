//! Instruction set for Alphabet's virtual machine,
//! including instruction decoding logic and Alpha
//! language parsing.

/// R-type instruction.
#[derive(Clone, Copy)]
pub struct RType {
    /// The index of the result register.
    pub r_result: usize,
    /// The index of the first operand register.
    pub r_op_1: usize,
    /// The index of the second operand register.
    pub r_op_2: usize,
}

/// I-type instruction.
#[derive(Clone, Copy)]
pub struct IType {
    /// The index of the result register (or source)
    /// register for stores and comparison branches.
    pub r_result: usize,
    /// The index of the operand register.
    pub r_op: usize,
    /// The immediate value.
    pub imm: u16,
}

/// Instruction opcodes.
pub mod op {
    /// No operation.
    pub const NOOP: u8 = 0x00;

    // R-type.

    /// Signed and unsigned addition.
    pub const ADD: u8 = 0x01;
    /// Signed and unsigned subtraction.
    pub const SUB: u8 = 0x02;

    /// Logical bitshift left.
    pub const SHL: u8 = 0x03;
    /// Logical bitshift right.
    pub const SHR: u8 = 0x04;
    /// Arithmetic bitshift right.
    pub const SAR: u8 = 0x05;

    /// Bitwise and.
    pub const AND: u8 = 0x06;
    /// Bitwise or.
    pub const OR: u8 = 0x08;
    /// Bitwise exclusive-or.
    pub const XOR: u8 = 0x0A;

    /// Less-than comparison.
    pub const SLT: u8 = 0x0C;
    /// Less-than unsigned comparison.
    pub const SLTU: u8 = 0x0D;

    // I-type.

    /// Immediate unsigned value addition.
    pub const ADDI: u8 = 0x21;
    /// Immediate unsigned value subtraction.
    pub const SUBI: u8 = 0x22;

    /// Immediate logical bitshift left.
    pub const SHLI: u8 = 0x23;
    /// Immediate logical bitshift right.
    pub const SHRI: u8 = 0x24;
    /// Immediate larithmetic bitshift right.
    pub const SARI: u8 = 0x25;

    /// Immediate bitwise and, lower 16 bits.
    pub const ANDI: u8 = 0x26;
    /// Immediate bitwise and, upper 16 bits.
    pub const ANDUI: u8 = 0x27;
    /// Immediate bitwise or, lower 16 bits.
    pub const ORI: u8 = 0x28;
    /// Immediate bitwise or, upper 16 bits.
    pub const ORUI: u8 = 0x29;
    /// Immediate bitwise exclusive-or, lower 16 bits.
    pub const XORI: u8 = 0x2A;
    /// Immediate bitwise exclusive-or, upper 16 bits.
    pub const XORUI: u8 = 0x2B;

    /// Less-than immediate comparison.
    pub const SLTI: u8 = 0x2C;

    /// Load word from memory.
    pub const LDW: u8 = 0x31;
    /// Load half-word from memory.
    pub const LDHW: u8 = 0x32;
    /// Load unsigned half-word from memory.
    pub const LDHWU: u8 = 0x33;
    /// Load byte from memory.
    pub const LDB: u8 = 0x34;
    /// Load unsigned byte from memory.
    pub const LDBU: u8 = 0x35;
    /// Store word to memory.
    pub const STW: u8 = 0x36;
    /// Store half-word to memory.
    pub const STHW: u8 = 0x37;
    /// Store byte to memory.
    pub const STB: u8 = 0x38;

    /// Jump and link by offset.
    pub const JMP: u8 = 0x39;
    /// Jump and link relative to register.
    pub const JMPR: u8 = 0x3A;
    /// Branch by offset if equal.
    pub const BEQ: u8 = 0x3B;
    /// Branch by offset if not equal.
    pub const BNE: u8 = 0x3C;
}

/// Determine the type of an instruction
/// based on the opcode. Returns true if
/// R-type, false if I-type.
pub const fn is_op_r_type(op: u8) -> bool {
    op < 0x20
}

/// The instruction data, excluding the opcode.
/// The kind of data present is determined by the opcode.
pub union Payload {
    // Empty data (present on 0x00).
    pub noop: (),

    // R-type instruction data (present for 0x01-0x1F).
    pub r_type: RType,

    // I-type instruction data (present for 0x20-0x3F).
    pub i_type: IType,
}

/// A machine instruction.
pub struct Instruction {
    /// The machine instruction opcode (0x00-0x3F).
    pub op: u8,

    /// The instruction data.
    pub payload: Payload,
}

impl Instruction {
    /// Decode a machine instruction from binary.
    pub fn decode(encoded: u32) -> Self {
        const IMM_MASK: u32 = 0xFFFF;
        const REG_MASK: u32 = 0b11111;

        let op = (encoded >> 26) as u8;
        let payload = if op == 0x00 {
            Payload { noop: () }
        } else if is_op_r_type(op) {
            // R-type.
            let r_result = ((encoded >> 21) & REG_MASK) as usize;
            let r_op_1 = ((encoded >> 16) & REG_MASK) as usize;
            let r_op_2 = ((encoded >> 11) & REG_MASK) as usize;
            let payload = RType {
                r_result,
                r_op_1,
                r_op_2,
            };
            Payload { r_type: payload }
        } else {
            // I-type.
            let r_result = ((encoded >> 21) & REG_MASK) as usize;
            let r_op = ((encoded >> 16) & REG_MASK) as usize;
            let imm = (encoded & IMM_MASK) as u16;
            let payload = IType {
                r_result,
                r_op,
                imm,
            };
            Payload { i_type: payload }
        };

        Self { op, payload }
    }
}
