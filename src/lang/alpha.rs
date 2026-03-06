//! Alpha lexing, parsing, and assembly.

pub use lex::SourceToken;
pub use parse::Statement;

/// Lexing for the Alpha assembly language.
pub mod lex {
    use crate::lang::SourceLocation;

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

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    /// The name of a register, by number or alias (`ra` and `sp`).
    pub enum RegisterName {
        /// The register is indexed (`r0`-`r31`).
        Index(u8),
        /// The register alias `ra` (`r30`).
        Ra,
        /// The register alias `sp` (`r31`).
        Sp,
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
            chars: Vec<u8>,
        },
    }

    #[derive(Debug, Clone)]
    /// A unit of assembly syntax from source.
    pub struct SourceToken {
        kind: Token,
        text: Vec<u8>,
        location: Option<SourceLocation>,
    }

    impl SourceToken {
        /// The kind of token, determining the token's role
        /// in the assembly syntax.
        pub fn kind(&self) -> &Token {
            &self.kind
        }

        /// The raw text that the lexer turned into this token.
        pub fn text(&self) -> &[u8] {
            &self.text
        }

        /// The location of the token in its source, if known.
        pub fn location(&self) -> Option<SourceLocation> {
            self.location
        }
    }
}

/// Parser for the Alpha assembly language.
pub mod parse {

    use crate::lang::SourceLocation;

    use super::lex::{NumberKind, RegisterName};

    /// A statement associating an address
    /// with a symbol.
    pub struct LabelStatement {
        symbol: Vec<u8>,
    }

    impl LabelStatement {
        pub fn symbol(&self) -> &str {
            unsafe { std::str::from_utf8_unchecked(&self.symbol) }
        }

        pub fn symbol_bytes(&self) -> &[u8] {
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
        String(Vec<u8>),
    }

    /// A component of a statement representing a constant value
    /// with a literal number or string or a named symbol.
    pub enum ImmediateOrSymbol {
        Symbol(Vec<u8>),
        Immediate(Immediate),
    }

    pub struct DirectiveStatement {
        directive: Vec<u8>,
        arguments: Vec<ImmediateOrSymbol>,
    }

    pub enum InstructionArgument {
        Register(RegisterName),
        Immediate(ImmediateOrSymbol),
        Offset {
            base: RegisterName,
            offset: ImmediateOrSymbol,
        },
    }

    pub struct InstructionStatement {
        instruction: Vec<u8>,
        arguments: Vec<InstructionArgument>,
    }

    pub enum Statement {
        Label(LabelStatement),
        Directive(DirectiveStatement),
        Instruction(InstructionStatement),
    }

    pub struct SourceStatement {
        kind: Statement,
        text: Vec<u8>,
        location: SourceLocation,
    }
}
