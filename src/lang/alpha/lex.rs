//! Lexing for the Alpha assembly language.

use std::{
    cell::RefCell,
    collections::VecDeque,
    error,
    fmt::Display,
    io::{self, BufReader, Bytes, Read},
    mem,
    num::IntErrorKind,
    rc::Rc,
    result,
};

use crate::lang::{
    SourceLocation,
    ascii::{AsciiChar, AsciiRef, AsciiStr, AsciiString, FallibleAsciiChars, Segmenter},
};
use crate::{ascii, vm};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Numerical base of an immediate value.
pub enum NumberBase {
    /// Base 2 (prefix `0b`).
    Binary,
    /// Base 8 (prefix `0o`).
    Octal,
    /// Base 10 (no prefix).
    Decimal,
    /// Base 16 (prefix `0x`).
    Hexadecimal,
}

impl NumberBase {
    /// The prefix associated with this number kind.
    pub const fn prefix(self) -> &'static AsciiStr {
        match self {
            Self::Binary => ascii!("0b"),
            Self::Octal => ascii!("0o"),
            Self::Decimal => ascii!(""),
            Self::Hexadecimal => ascii!("0x"),
        }
    }

    /// Get the base system from the prefix character.
    pub const fn from_prefix(prefix: AsciiChar) -> Option<Self> {
        match prefix.byte() {
            b'b' => Some(Self::Binary),
            b'o' => Some(Self::Octal),
            b'x' => Some(Self::Hexadecimal),
            _ => None,
        }
    }

    /// Return the radix of this number base.
    pub const fn radix(self) -> u32 {
        match self {
            Self::Binary => 2,
            Self::Octal => 8,
            Self::Decimal => 10,
            Self::Hexadecimal => 16,
        }
    }

    /// Whether the provided character is a valid digit
    /// for this base system.
    pub const fn is_valid_digit(self, digit: AsciiChar) -> bool {
        let byte = digit.byte();
        match self {
            Self::Binary => matches!(byte, b'0' | b'1'),
            Self::Octal => matches!(byte, b'0'..=b'7'),
            Self::Decimal => byte.is_ascii_digit(),
            Self::Hexadecimal => byte.is_ascii_hexdigit(),
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

    fn try_from(value: u8) -> result::Result<Self, Self::Error> {
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

#[derive(Debug, Clone)]
/// Error lexing an ASCII hexcode escape.
pub enum HexEscapeError {
    /// A non-hex-digit character was found.
    NonHexChar,
    /// The parsed byte is outside the valid ASCII range.
    NonAscii,
}

/// An error parsing a token.
#[derive(Debug)]
pub enum Error {
    /// I/O error.
    Io(io::Error),
    /// Non-ASCII digit encountered in input.
    NonAscii(u8),
    /// Character not valid starting a token.
    InvalidStart,
    /// Directive expected alphabetic first character.
    ExpectedDirective,
    /// Number expected digit after prefix.
    ExpectedDigit,
    /// Unclosed string literal.
    UnclosedString,
    /// Invalid escape code.
    InvalidEscape,
    /// Invalid hex digits for escape code.
    InvalidHexEscape(HexEscapeError),
    /// Numeric literal is outside the valid range.
    NumericOutOfRange,
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "token error")
    }
}

impl error::Error for Error {}

/// An error parsing a token, bundled with information
/// about the source the token came from.
#[derive(Debug)]
pub struct SourceError<'a> {
    /// The error with the token.
    pub error: Error,
    /// The source that caused the error.
    pub source: AsciiRef<'a>,
    /// The location of the source that caused the error;.
    pub location: SourceLocation,
}

impl Display for SourceError<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "source token error")
    }
}

impl error::Error for SourceError<'_> {}

/// Result of lexing.
pub type Result = result::Result<Token, Error>;

/// Result of lexing from source..
pub type SourceResult<'a> = result::Result<SourceToken<'a>, SourceError<'a>>;

/// A valid name for a symbol, which consists of
/// alphanumeric characters and underscores and may
/// not begin with a digit.
#[derive(Debug, Clone)]
pub struct SymbolName(AsciiString);

impl From<SymbolName> for AsciiString {
    fn from(value: SymbolName) -> Self {
        value.0
    }
}

impl TryFrom<AsciiString> for SymbolName {
    type Error = AsciiChar;

    fn try_from(value: AsciiString) -> result::Result<Self, Self::Error> {
        let symbol = SymbolName(value);
        let mut chars = symbol.0.chars();
        let Some(first) = chars.next() else {
            return Ok(symbol);
        };
        if !first.byte().is_ascii_alphabetic() && first.byte() != b'_' {
            return Err(first);
        }
        for char in chars {
            if !char.byte().is_ascii_alphanumeric() && char.byte() != b'_' {
                return Err(char);
            }
        }
        Ok(symbol)
    }
}

impl AsRef<AsciiString> for SymbolName {
    fn as_ref(&self) -> &AsciiString {
        &self.0
    }
}

/// A valid name for an assembler directive,
/// consisting of alphabetic characters.
#[derive(Debug, Clone)]
pub struct DirectiveName(AsciiString);

impl From<DirectiveName> for AsciiString {
    fn from(value: DirectiveName) -> Self {
        value.0
    }
}

impl TryFrom<AsciiString> for DirectiveName {
    type Error = AsciiChar;

    fn try_from(value: AsciiString) -> result::Result<Self, Self::Error> {
        for char in value.chars() {
            if !char.byte().is_ascii_alphabetic() {
                return Err(char);
            }
        }
        Ok(DirectiveName(value))
    }
}

impl AsRef<AsciiString> for DirectiveName {
    fn as_ref(&self) -> &AsciiString {
        &self.0
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
    /// A sequence of letters representing a directive,
    /// beginning with a `.` symbol.
    Directive(DirectiveName),
    /// A sequence of letters and digits naming an immediate value that
    /// may be used in instructions.
    Symbol(SymbolName),
    /// The name of a register, by number or alias (`ra` and `sp`).
    Register(RegisterName),
    /// A sequence of letters and digits naming an instruction address
    /// that may be used in instructions as a word offset, ending with
    /// a `:` symbol.
    Label(SymbolName),
    /// A numeric literal immediate value.
    Number {
        /// The immediate value.
        value: u32,
        /// The base of the literal.
        base: NumberBase,
    },
    /// A string literal immediate value.
    String(AsciiString),
}

#[derive(Debug, Clone)]
/// A unit of assembly syntax from source.
pub struct SourceToken<'a> {
    pub(crate) token: Token,
    pub(crate) source: AsciiRef<'a>,
    pub(crate) location: SourceLocation,
}

impl SourceToken<'_> {
    /// The kind of token, determining the token's role
    /// in the assembly syntax.
    pub fn token(&self) -> &Token {
        &self.token
    }

    /// The raw text that the lexer turned into this token.
    pub fn source(&self) -> &AsciiStr {
        &self.source
    }

    /// The location of the token in its source, if known.
    pub fn location(&self) -> SourceLocation {
        self.location
    }
}

impl AsRef<Token> for SourceToken<'_> {
    fn as_ref(&self) -> &Token {
        &self.token
    }
}

fn is_valid_string_char(ch: u8) -> bool {
    ch.is_ascii_graphic() || ch == b' '
}

fn escape_char(ch: AsciiChar) -> Option<AsciiChar> {
    Some(
        match ch.byte() {
            b'0' => b'\0',
            b'n' => b'\n',
            b'r' => b'\r',
            b't' => b'\t',
            ch @ (b'\\' | b'"' | b'\'') => ch,
            _ => return None,
        }
        .try_into()
        .unwrap(),
    )
}

fn parse_register(name: &AsciiStr) -> Option<RegisterName> {
    let len = name.len();
    if len != 2 && len != 3 {
        return None;
    }
    if name == ascii!("sp") {
        return Some(RegisterName::Sp);
    }
    if name == ascii!("ra") {
        return Some(RegisterName::Ra);
    };
    if name[0].byte() != b'r' {
        return None;
    }
    let index: &str = name[1..].as_ref();
    if let Ok(index) = index.parse::<u8>()
        && let Ok(index) = RegisterIndex::try_from(index)
    {
        Some(RegisterName::Index(index))
    } else {
        None
    }
}

fn to_negative_i32_bits(val: u32) -> Option<u32> {
    if val == 0 || val > (i32::MAX as u32) + 1 {
        return None;
    }

    Some(!val + 1)
}

#[derive(Clone, Copy)]
enum StringEscape {
    // Not escaping.
    Literal,
    // Escape next character.
    Escape,
    // First character of hex escape.
    HexOne,
    // Second character of hex escape.
    HexTwo(AsciiChar),
}

struct StringState {
    quote_kind: AsciiChar,
    escape: StringEscape,
    chars: AsciiString,
}

#[derive(Clone, Copy)]
enum NumberExpect {
    /// Expect a prefix (e.g. 0x) or for the number to start.
    Start,
    /// Expect a prefix character (b, o, x).
    Prefix,
    /// Expect a digit valid to the current base.
    Digit,
}

struct NumberState {
    negative: bool,
    expect: NumberExpect,
    base: NumberBase,
    digits: AsciiString,
}

#[derive(Default)]
enum State {
    #[default]
    /// No active token state.
    None,
    /// Whitespace state consumes whitespace characters.
    Whitespace,
    /// Comment state consumes everything until newline.
    Comment,
    /// Directive state consumes alphabetic characters.
    Directive(AsciiString),
    /// String state consumes valid string literal characters.
    String(StringState),
    /// Number state consumes numeral prefixes and digits
    /// for an immediate value.
    Number(NumberState),
    /// Identifier state consumes alphanumeric characters
    /// for symbols, register IDs, and labels.
    Identifier(AsciiString),
}

fn lex_string(
    state: &mut StringState,
    current_char: AsciiChar,
    next_char: Option<AsciiChar>,
) -> Option<result::Result<(), Error>> {
    let next_char_valid = next_char.is_some_and(|n| is_valid_string_char(n.byte()));
    match state.escape {
        StringEscape::Literal => {
            // End string with matching unescaped quote.
            if current_char == state.quote_kind {
                Some(Ok(()))
            } else if !next_char_valid {
                // Verify next character.
                Some(Err(Error::UnclosedString))
            } else if current_char.byte() == b'\\' {
                // Begin escape.
                state.escape = StringEscape::Escape;
                None
            } else {
                state.chars.push_char(current_char);
                None
            }
        }
        _ if !next_char_valid => Some(Err(Error::UnclosedString)),
        StringEscape::Escape => {
            if current_char.byte() == b'x' {
                state.escape = StringEscape::HexOne;
                None
            } else if let Some(escaped) = escape_char(current_char) {
                state.escape = StringEscape::Literal;
                state.chars.push_char(escaped);
                None
            } else {
                Some(Err(Error::InvalidEscape))
            }
        }
        StringEscape::HexOne => {
            if current_char.byte().is_ascii_hexdigit() {
                state.escape = StringEscape::HexTwo(current_char);
                None
            } else {
                Some(Err(Error::InvalidHexEscape(HexEscapeError::NonHexChar)))
            }
        }
        StringEscape::HexTwo(first_char) => {
            if current_char.byte().is_ascii_hexdigit() {
                let string: &AsciiStr = [first_char, current_char].as_slice().into();
                let string: &str = string.as_ref();
                let byte = u8::from_str_radix(string, 16).expect("parsing hex failed");
                if let Ok(ch) = byte.try_into() {
                    state.chars.push_char(ch);
                    None
                } else {
                    Some(Err(Error::InvalidHexEscape(HexEscapeError::NonAscii)))
                }
            } else {
                state.escape = StringEscape::Literal;
                Some(Err(Error::InvalidHexEscape(HexEscapeError::NonHexChar)))
            }
        }
    }
}

fn lex_number(
    state: &mut NumberState,
    current_char: AsciiChar,
    next_char: Option<AsciiChar>,
) -> Option<result::Result<(), Error>> {
    let end_digits = |base: NumberBase| next_char.is_none_or(|n| !base.is_valid_digit(n));

    match state.expect {
        NumberExpect::Start => {
            if current_char.byte() == b'0'
                && next_char.is_some_and(|n| NumberBase::from_prefix(n).is_some())
            {
                state.expect = NumberExpect::Prefix;
                return None;
            }
            state.digits.push_char(current_char);
            if end_digits(state.base) {
                Some(Ok(()))
            } else {
                state.expect = NumberExpect::Digit;
                None
            }
        }
        NumberExpect::Prefix => {
            state.expect = NumberExpect::Digit;
            state.base = NumberBase::from_prefix(current_char)
                .expect("invalid token state: expected valid prefix char but found {current_char}");
            if end_digits(state.base) {
                return Some(Err(Error::ExpectedDigit));
            }
            None
        }
        NumberExpect::Digit => {
            state.digits.push_char(current_char);
            end_digits(state.base).then_some(Ok(()))
        }
    }
}

impl State {
    fn lex_none(
        &mut self,
        current_char: AsciiChar,
        next_char: Option<AsciiChar>,
    ) -> Option<Result> {
        let current_byte = current_char.byte();
        let next_byte = next_char.map(|ch| ch.byte());
        type Sm = State;
        type Tk = Token;

        match (current_byte, next_byte) {
            // Newline must come before whitespace.
            (b'\n', _) => Some(Ok(Tk::Newline)),
            (c, n) if c.is_ascii_whitespace() => {
                if n.is_some_and(|n| n != b'\n' && n.is_ascii_whitespace()) {
                    *self = Sm::Whitespace;
                    None
                } else {
                    Some(Ok(Tk::Space))
                }
            }
            (b';', None | Some(b'\n')) => Some(Ok(Tk::Comment)),
            (b';', _) => {
                *self = Sm::Comment;
                None
            }
            (b',', _) => Some(Ok(Tk::Comma)),
            (b'(', _) => Some(Ok(Tk::OpenParen)),
            (b')', _) => Some(Ok(Tk::CloseParen)),
            (b'.', n) => {
                if n.is_some_and(|n| n.is_ascii_alphabetic()) {
                    *self = Sm::Directive(AsciiString::new());
                    None
                } else {
                    Some(Err(Error::ExpectedDirective))
                }
            }
            (b'\'' | b'"', n) => {
                if n.is_some_and(is_valid_string_char) {
                    *self = Sm::String(StringState {
                        quote_kind: current_char,
                        escape: StringEscape::Literal,
                        chars: AsciiString::new(),
                    });
                    None
                } else {
                    Some(Err(Error::UnclosedString))
                }
            }
            (c, n) if c.is_ascii_alphabetic() || c == b'_' => {
                if n.is_some_and(|n| n.is_ascii_alphanumeric() || n == b'_' || n == b':') {
                    *self = Sm::Identifier([current_char].into());
                    None
                } else {
                    let symbol = SymbolName([current_char].into());
                    Some(Ok(Tk::Symbol(symbol)))
                }
            }
            (c, n) if c.is_ascii_digit() => {
                if n.is_some_and(|n| n.is_ascii_digit()) {
                    *self = Sm::Number(NumberState {
                        negative: false,
                        expect: NumberExpect::Digit,
                        base: NumberBase::Decimal,
                        digits: [current_char].into(),
                    });
                    None
                } else if c == b'0'
                    && next_char.is_some_and(|n| NumberBase::from_prefix(n).is_some())
                {
                    *self = Sm::Number(NumberState {
                        negative: false,
                        expect: NumberExpect::Prefix,
                        base: NumberBase::Decimal,
                        digits: AsciiString::new(),
                    });
                    None
                } else {
                    *self = Sm::None;
                    Some(Ok(Token::Number {
                        value: (c - b'0') as u32,
                        base: NumberBase::Decimal,
                    }))
                }
            }
            (c @ (b'+' | b'-'), n) => {
                if n.is_some_and(|n| n.is_ascii_digit()) {
                    *self = Sm::Number(NumberState {
                        negative: c == b'-',
                        expect: NumberExpect::Start,
                        base: NumberBase::Decimal,
                        digits: AsciiString::new(),
                    });
                    None
                } else {
                    Some(Err(Error::ExpectedDigit))
                }
            }
            _ => Some(Err(Error::InvalidStart)),
        }
    }

    fn lex_char(
        &mut self,
        current_char: AsciiChar,
        next_char: Option<AsciiChar>,
    ) -> Option<Result> {
        type Sm = State;
        type Tk = Token;

        match self {
            Sm::None => self.lex_none(current_char, next_char),
            Sm::Comment => {
                if next_char.is_none_or(|n| n.byte() == b'\n') {
                    *self = Sm::None;
                    Some(Ok(Tk::Comment))
                } else {
                    None
                }
            }
            Sm::Whitespace => {
                if next_char.is_some_and(|n| n.byte() != b'\n' && n.byte().is_ascii_whitespace()) {
                    None
                } else {
                    *self = Sm::None;
                    Some(Ok(Tk::Space))
                }
            }
            Sm::Directive(name) => {
                name.push_char(current_char);
                if let Some(n) = next_char
                    && n.byte().is_ascii_alphabetic()
                {
                    None
                } else {
                    let name = mem::take(name);
                    *self = Sm::None;
                    Some(Ok(Tk::Directive(DirectiveName(name))))
                }
            }
            Sm::Identifier(ident) => {
                if let Some(n) = next_char
                    && (n.byte().is_ascii_alphanumeric() || n.byte() == b'_' || n.byte() == b':')
                {
                    return if n.byte() == b':'
                        && let Some(register) = parse_register(ident)
                    {
                        *self = Sm::None;
                        Some(Ok(Tk::Register(register)))
                    } else {
                        ident.push_char(current_char);
                        None
                    };
                }
                let mut ident = mem::take(ident);
                *self = Sm::None;
                let token = if current_char.byte() == b':' {
                    Tk::Label(SymbolName(ident))
                } else {
                    ident.push_char(current_char);
                    if let Some(register) = parse_register(&ident) {
                        Tk::Register(register)
                    } else {
                        Tk::Symbol(SymbolName(ident))
                    }
                };
                Some(Ok(token))
            }
            Sm::String(state) => {
                let result = lex_string(state, current_char, next_char)?;
                if let Err(err) = result {
                    *self = Sm::None;
                    return Some(Err(err));
                }
                let chars = mem::take(&mut state.chars);
                *self = Sm::None;
                Some(Ok(Tk::String(chars)))
            }
            Sm::Number(state) => {
                let result = lex_number(state, current_char, next_char)?;
                if let Err(err) = result {
                    *self = Sm::None;
                    return Some(Err(err));
                }
                let digits = mem::take(&mut state.digits);
                let digits: &str = digits.as_ref();
                let base = state.base;
                let value = u32::from_str_radix(digits, base.radix());
                let negative = state.negative;
                *self = Sm::None;
                Some(match value {
                    Ok(value) => {
                        if negative {
                            if let Some(value) = to_negative_i32_bits(value) {
                                Ok(Tk::Number { value, base })
                            } else {
                                Err(Error::NumericOutOfRange)
                            }
                        } else {
                            Ok(Tk::Number { value, base })
                        }
                    }
                    Err(err) if *err.kind() == IntErrorKind::PosOverflow => {
                        Err(Error::NumericOutOfRange)
                    }
                    Err(err) => panic!("unexpected int parse error {err:?} from {digits:?}"),
                })
            }
        }
    }
}

/// Iterator over source assembly that outputs a stream of tokens.
pub struct Lexer<'a, I: Iterator<Item = AsciiChar>> {
    segmenter: Segmenter<'a, I>,
    state: State,
}

impl<'a> Lexer<'a, <AsciiRef<'a> as IntoIterator>::IntoIter> {
    /// Lex a string slice; each resulting token slices this string.
    pub fn lex_str<T: Into<AsciiRef<'a>>>(source: T) -> Self {
        let segmenter = Segmenter::segment_str(source);
        Lexer {
            segmenter,
            state: State::None,
        }
    }
}

impl<I: Iterator<Item = AsciiChar>> Lexer<'static, I> {
    /// Lex a stream of characters; each token builds its own string contents.
    pub fn lex_chars<T: IntoIterator<IntoIter = I>>(source: T) -> Self {
        let segmenter = Segmenter::segment_chars(source);
        Lexer {
            segmenter,
            state: State::None,
        }
    }
}

impl<'a, I: Iterator<Item = AsciiChar>> Lexer<'a, I> {
    /// Stop lexing and return the rest of the unlexed characters.
    pub fn rest(self) -> AsciiRef<'a> {
        self.segmenter.rest()
    }

    fn sourcify(&mut self, result: Result) -> SourceResult<'a> {
        let (source, location) = self.segmenter.cut();
        match result {
            Ok(token) => Ok(SourceToken {
                token,
                source,
                location,
            }),
            Err(error) => Err(SourceError {
                error,
                source,
                location,
            }),
        }
    }
}

impl<'a, I: Iterator<Item = AsciiChar>> Iterator for Lexer<'a, I> {
    type Item = SourceResult<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let Some((_, current_char)) = self.segmenter.next() else {
                // Active token is invalid state.
                debug_assert!(matches!(self.state, State::None), "unhandled active token");
                return None;
            };
            let next_char = self.segmenter.peek().map(|(_, ch)| ch);
            let result = self.state.lex_char(current_char, next_char);
            let Some(result) = result else {
                // Continue lexing until a token is complete.
                continue;
            };
            return Some(self.sourcify(result));
        }
    }
}

pub struct ByteLexer<I: Iterator<Item = u8>> {
    invalid: Rc<RefCell<VecDeque<u8>>>,
    lexer: Lexer<'static, FallibleAsciiChars<u8, I>>,
}

impl<I> ByteLexer<I>
where
    I: Iterator<Item = u8>,
{
    pub fn lex<J: IntoIterator<IntoIter = I>>(bytes: J) -> Self {
        let (chars, invalid) = FallibleAsciiChars::new(bytes);
        ByteLexer {
            invalid,
            lexer: Lexer::lex_chars(chars),
        }
    }

    fn check_invalid(&mut self) -> Option<<Self as Iterator>::Item> {
        let mut invalid = RefCell::borrow_mut(&self.invalid);
        let byte = invalid.pop_front()?;
        let (source, location) = self.lexer.segmenter.cut();
        self.lexer.state = State::None;
        let error = SourceError {
            error: Error::NonAscii(byte),
            source,
            location,
        };
        Some(Err(error))
    }
}

impl<I> Iterator for ByteLexer<I>
where
    I: Iterator<Item = u8>,
{
    type Item = SourceResult<'static>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(err) = self.check_invalid() {
            return Some(err);
        }
        let result = self.lexer.next()?;
        let invalid = self.check_invalid();
        invalid.or(Some(result))
    }
}

pub struct ReadLexer<I: Iterator<Item = io::Result<u8>>> {
    invalid: Rc<RefCell<VecDeque<io::Result<u8>>>>,
    lexer: Lexer<'static, FallibleAsciiChars<io::Result<u8>, I>>,
}

impl<I> ReadLexer<I>
where
    I: Iterator<Item = io::Result<u8>>,
{
    pub fn lex<J: IntoIterator<IntoIter = I>>(bytes: J) -> Self {
        let (chars, invalid) = FallibleAsciiChars::new(bytes);
        ReadLexer {
            invalid,
            lexer: Lexer::lex_chars(chars),
        }
    }

    fn check_invalid(&mut self) -> Option<<Self as Iterator>::Item> {
        let mut invalid = RefCell::borrow_mut(&self.invalid);
        let result = invalid.pop_front()?;
        let (source, location) = self.lexer.segmenter.cut();
        self.lexer.state = State::None;
        let error = match result {
            Ok(byte) => Error::NonAscii(byte),
            Err(err) => Error::Io(err),
        };
        let error = SourceError {
            error,
            source,
            location,
        };
        Some(Err(error))
    }
}

impl<R: Read> ReadLexer<Bytes<BufReader<R>>> {
    /// Lex the contents of a reader, which is
    /// automatically buffered.
    pub fn lex_reader(reader: R) -> Self {
        let buf = BufReader::new(reader);
        Self::lex(buf.bytes())
    }
}

impl<I> Iterator for ReadLexer<I>
where
    I: Iterator<Item = io::Result<u8>>,
{
    type Item = SourceResult<'static>;

    fn next(&mut self) -> Option<Self::Item> {
        let invalid = self.check_invalid();
        if invalid.is_some() {
            return invalid;
        }
        let result = self.lexer.next()?;
        let invalid = self.check_invalid();
        invalid.or(Some(result))
    }
}
