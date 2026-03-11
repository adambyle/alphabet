//! Lexing for the Alpha assembly language.

use std::fmt::Display;

use crate::lang::{
    SourceLocation,
    ascii::{AsciiRef, AsciiStr, AsciiString},
};
use crate::{ascii, vm};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Numerical base of an immediate value.
pub enum NumberKind {
    /// Base 2 (prefix `0b`).
    Binary,
    /// Base 8 (prefix `0o`).
    Octal,
    /// Base 10 (no prefix).
    Decimal,
    /// Base 16 (prefix `0x`).
    Hexadecimal,
}

impl NumberKind {
    /// The prefix associated with this number kind.
    pub const fn prefix(self) -> &'static AsciiStr {
        match self {
            NumberKind::Binary => ascii!("0b"),
            NumberKind::Octal => ascii!("0o"),
            NumberKind::Decimal => ascii!(""),
            NumberKind::Hexadecimal => ascii!("0x"),
        }
    }
}

/// A numerical index for a register.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct RegisterIndex(u8);

impl RegisterIndex {
    /// The value of the register index.
    pub fn value(self) -> u8 {
        self.0
    }
}

impl TryFrom<u8> for RegisterIndex {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value >= vm::REGISTER_COUNT as u8 {
            Err(())
        } else {
            Ok(Self(value))
        }
    }
}

impl From<RegisterIndex> for u8 {
    fn from(value: RegisterIndex) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
/// The name of a register, by number or alias (`ra` and `sp`).
pub enum RegisterName {
    /// The register is indexed (`r0`-`r31`).
    Index(RegisterIndex),
    /// The register alias `ra` (`r30`).
    Ra,
    /// The register alias `sp` (`r31`).
    Sp,
}

impl Display for RegisterName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            RegisterName::Index(RegisterIndex(idx)) => write!(f, "r{idx}"),
            RegisterName::Ra => write!(f, "ra"),
            RegisterName::Sp => write!(f, "sp"),
        }
    }
}

/// A unit of assembly syntax.
#[derive(Debug, Clone)]
pub enum Token {
    /// The start of a new line of source code.
    Newline,
    /// Whitespace between tokens.
    Space,
    /// Human-readable text ignored by the assembler, beginning with a `;` symbol.
    Comment,
    /// A `,` symbol separating instruction and directive arguments.
    Comma,
    /// A `(` symbol preceding a register token, beginning an immediate offset.
    OpenParen,
    /// A `)` symbol following a register token, ending an immediate offset.
    CloseParen,
    /// A sequence of letters and digits representing a directive,
    /// beginning with a `.` symbol.
    Directive,
    /// A sequence of letters and digits naming an immediate value that
    /// may be used in instructions.
    Symbol,
    /// The name of a register, by number or alias (`ra` and `sp`).
    Register(RegisterName),
    /// A sequence of letters and digits naming an instruction address
    /// that may be used in instructions as a word offset, ending with
    /// a `:` symbol.
    Label,
    /// A numeric literal immediate value.
    Number {
        /// The immediate value.
        value: u32,
        /// The base of the literal.
        kind: NumberKind,
    },
    /// A string literal immediate value.
    String {
        /// The characters that make up the string (escapes handled).
        chars: AsciiString,
    },
}

#[derive(Debug, Clone)]
/// A unit of assembly syntax from source.
pub struct SourceToken<'a> {
    kind: Token,
    text: AsciiRef<'a>,
    location: Option<SourceLocation>,
}

impl SourceToken<'_> {
    /// The kind of token, determining the token's role
    /// in the assembly syntax.
    pub fn kind(&self) -> &Token {
        &self.kind
    }

    /// The raw text that the lexer turned into this token.
    pub fn text(&self) -> &AsciiStr {
        &self.text
    }

    /// The location of the token in its source, if known.
    pub fn location(&self) -> Option<SourceLocation> {
        self.location
    }
}
