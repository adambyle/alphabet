//! Parser for the Alpha assembly language.

use crate::lang::{
    SourceLocation,
    ascii::{AsciiRef, AsciiStr, AsciiString},
};

use super::lex::{NumberKind, RegisterName};

/// A statement associating an address
/// with a symbol.
pub struct LabelStatement<'a> {
    symbol: AsciiRef<'a>,
}

impl LabelStatement<'_> {
    /// The symbol that is assigned by the label statement.
    pub fn symbol(&self) -> &AsciiStr {
        &self.symbol
    }
}

/// A component of a statement representing a constant value
/// with a literal number or string.
pub enum Immediate {
    /// A numeric literal immediate.
    Number {
        /// The immediate value.
        value: u32,
        /// The base of the literal.
        kind: NumberKind,
    },
    /// A string literal immediate.
    ///
    /// Wraps the string literal without the quotes, and
    /// with all escapes processed.
    String(AsciiString),
}

/// A component of a statement representing a constant value
/// with a literal number or string or a named symbol.
pub enum ImmediateOrSymbol<'a> {
    /// A symbol's value.
    Symbol(AsciiRef<'a>),
    /// An immediate value.
    Immediate(Immediate),
}

/// A statement that controls the assembler in some way
/// or outputs non-instruction data.
pub struct DirectiveStatement<'a> {
    name: AsciiRef<'a>,
    arguments: Vec<ImmediateOrSymbol<'a>>,
}

/// An argument to an instruction.
pub enum InstructionArgument<'a> {
    /// A register argument.
    Register(RegisterName),
    /// An immediate-value argument.
    Immediate(ImmediateOrSymbol<'a>),
    /// An immediate offset from register value argument.
    Offset {
        /// The register holding the base address.
        base: RegisterName,
        /// The offset value.;
        offset: ImmediateOrSymbol<'a>,
    },
}

/// A statement that represents a machine instruction.
pub struct InstructionStatement<'a> {
    instruction: AsciiRef<'a>,
    arguments: Vec<InstructionArgument<'a>>,
}

/// A statement of assembly, the highest-level unit of source
/// there is.
pub enum Statement<'a> {
    /// A label statement.
    Label(LabelStatement<'a>),
    /// A directive statement.
    Directive(DirectiveStatement<'a>),
    /// An instruction statement.
    Instruction(InstructionStatement<'a>),
}

/// A statement of assembly and its source.
pub struct SourceStatement<'a> {
    kind: Statement<'a>,
    text: AsciiRef<'a>,
    location: SourceLocation,
}
