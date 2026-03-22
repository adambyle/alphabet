//! Parser for the Alpha assembly language.
#![allow(missing_docs)]

use std::{
    error,
    fmt::Display,
    iter::{Map, Peekable},
    result,
};

use crate::lang::{
    SourceLocation,
    alpha::{
        SourceToken, Token,
        lex::{self, NumberBase, RegisterName, SymbolName},
    },
    ascii::{AsciiRef, AsciiStr, AsciiString},
};

/// A component of a statement representing a constant value
/// with a literal number or string.
#[derive(Debug, Clone)]
pub enum Immediate {
    /// A numeric literal immediate.
    Number {
        /// The immediate value.
        value: u32,
        /// The base of the literal.
        kind: NumberBase,
    },
    /// A string literal immediate.
    ///
    /// Wraps the string literal without the quotes, and
    /// with all escapes processed.
    String(AsciiString),
}

/// A component of a statement representing a constant value
/// with a literal number or string or a named symbol.
#[derive(Debug, Clone)]
pub enum ImmediateOrSymbol {
    /// A symbol's value.
    Symbol(AsciiString),
    /// An immediate value.
    Immediate(Immediate),
}

/// A statement that controls the assembler in some way
/// or outputs non-instruction data.
#[derive(Debug, Clone)]
pub struct DirectiveStatement {
    name: AsciiString,
    arguments: Vec<ImmediateOrSymbol>,
}

/// An argument to an instruction.
#[derive(Debug, Clone)]
pub enum InstructionArgument {
    /// A register argument.
    Register(RegisterName),
    /// An immediate-value argument.
    Immediate(ImmediateOrSymbol),
    /// An immediate offset from register value argument.
    Offset {
        /// The register holding the base address.
        base: RegisterName,
        /// The offset value.;
        offset: ImmediateOrSymbol,
    },
}

/// A statement that represents a machine instruction.
#[derive(Debug, Clone)]
pub struct InstructionStatement {
    instruction: AsciiString,
    arguments: Vec<InstructionArgument>,
}

/// A statement of assembly, the highest-level unit of source
/// there is.
#[derive(Debug, Clone)]
pub enum Statement {
    /// A label statement.
    Label(SymbolName),
    /// A directive statement.
    Directive(DirectiveStatement),
    /// An instruction statement.
    Instruction(InstructionStatement),
}

#[derive(Debug)]
pub enum Error {
    /// Upstream error from the lexer.
    TokenError(lex::Error),
    /// Character not valid starting a token.
    InvalidStart,
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "statement error")
    }
}

impl error::Error for Error {}

/// A statement of assembly and its source.
#[derive(Debug, Clone)]
pub struct SourceStatement<'a> {
    statement: Statement,
    source: AsciiRef<'a>,
    location: SourceLocation,
}

impl SourceStatement<'_> {
    pub fn statement(&self) -> &Statement {
        &self.statement
    }

    pub fn source(&self) -> &AsciiStr {
        &self.source
    }

    pub fn location(&self) -> SourceLocation {
        self.location
    }
}

#[derive(Debug)]
pub struct SourceError<'a> {
    pub error: Error,
    pub source: AsciiRef<'a>,
    pub location: SourceLocation,
}

impl Display for SourceError<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "statement error")
    }
}

impl error::Error for SourceError<'_> {}

pub type Result = result::Result<Statement, Error>;
pub type SourceResult<'a> = result::Result<SourceStatement<'a>, SourceError<'a>>;

enum State {
    None,
}

impl State {
    fn parse_none(&mut self, current_token: &Token, next_token: Option<&Token>) -> Option<Result> {
        type Sm = State;
        type St = Statement;
        type Tk = Token;

        match (current_token, next_token) {
            (Tk::Comment | Tk::Space | Tk::Newline, _) => {
                // Ignore comment to start statement.
                None
            }
            (Tk::Label(symbol), _) => {
                // Label statements are made up of a single token.
                Some(Ok(Statement::Label(symbol.clone())))
            }
            (Tk::Directive(name), _) => {
                todo!()
            }
            _ => Some(Err(Error::InvalidStart)),
        }
    }

    fn parse_token(&mut self, current_token: &Token, next_token: Option<&Token>) -> Option<Result> {
        type Sm = State;

        match self {
            Sm::None => self.parse_none(current_token, next_token),
        }
    }
}

/// Iterator over a sequence of tokens that outputs
/// a stream of statements.
pub struct Parser<'a, T, I: Iterator<Item = T>> {
    tokens: Peekable<I>,
    first_token: Option<SourceToken<'a>>,
    state: State,
}

impl<T, I: Iterator<Item = T>> Parser<'_, T, I> {
    pub fn rest(self) -> Peekable<I> {
        self.tokens
    }
}

impl<'a, I: Iterator<Item = lex::SourceResult<'a>>> Parser<'_, lex::SourceResult<'a>, I> {
    pub fn parse(tokens: I) -> Self {
        Parser {
            tokens: tokens.peekable(),
            first_token: None,
            state: State::None,
        }
    }

    fn sourcify(result: Result, first_token: SourceToken<'a>) -> SourceResult<'a> {
        match result {
            Ok(statement) => Ok(SourceStatement {
                statement,
                source: first_token.source,
                location: first_token.location,
            }),
            Err(error) => Err(SourceError {
                error,
                source: first_token.source,
                location: first_token.location,
            }),
        }
    }
}

type FallibleTokenMap<'a> = fn(SourceToken<'a>) -> lex::SourceResult<'a>;

impl<'a, I: Iterator<Item = SourceToken<'a>>>
    Parser<'a, lex::SourceResult<'a>, Map<I, FallibleTokenMap<'a>>>
{
    pub fn parse_infallible(tokens: I) -> Self {
        let to_fallible: FallibleTokenMap = |token| Ok(token);
        let tokens = tokens.map(to_fallible);
        Self::parse(tokens)
    }
}

impl<I: Iterator<Item = Token>> Parser<'_, Token, I> {
    pub fn parse(tokens: I) -> Self {
        Parser {
            tokens: tokens.peekable(),
            first_token: None,
            state: State::None,
        }
    }
}

impl<'a, I> Iterator for Parser<'a, lex::SourceResult<'a>, I>
where
    I: Iterator<Item = lex::SourceResult<'a>>,
{
    type Item = SourceResult<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let Some(current_token) = self.tokens.next() else {
                // Active statement is in invalid state.
                debug_assert!(
                    matches!(self.state, State::None),
                    "unhandled active statement"
                );
                return None;
            };
            let current_token = match current_token {
                Ok(token) => token,
                Err(source_error) => {
                    let source_error = SourceError {
                        error: Error::TokenError(source_error.error),
                        location: source_error.location,
                        source: source_error.source,
                    };
                    return Some(Err(source_error));
                }
            };
            // A next error token is the same as no valid token being next (end statement).
            // TODO consider whether this is an appropriate approach.
            let next_token = self
                .tokens
                .peek()
                .and_then(|t| t.as_ref().ok().map(|t| &t.token));
            let result = self.state.parse_token(&current_token.token, next_token);
            let first_token = self.first_token.take().unwrap_or(current_token);
            let Some(result) = result else {
                // Continue parsing until a statement is complete.
                // Reinsert first token, it was not used.
                self.first_token = Some(first_token);
                continue;
            };
            let result = Self::sourcify(result, first_token);
            // self.first_token = None.
            return Some(result);
        }
    }
}

impl<I> Iterator for Parser<'_, Token, I>
where
    I: Iterator<Item = Token>,
{
    type Item = Result;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let Some(current_token) = self.tokens.next() else {
                // Active statement is in invalid state.
                debug_assert!(
                    matches!(self.state, State::None),
                    "unhandled active statement"
                );
                return None;
            };
            let next_token = self.tokens.peek();
            let result = self.state.parse_token(&current_token, next_token);
            let Some(result) = result else {
                // Continue parsing until a statement is complete.
                continue;
            };
            return Some(result);
        }
    }
}
