//! Alpha lexing, parsing, and assembly.

use std::str::Bytes;

use crate::lang::SourceLocation;

#[derive(Debug)]
pub struct LexicalError {
    location: SourceLocation,
}

pub enum TokenKind {
    Newline,
}

pub struct Token {
    location: SourceLocation,
    kind: Option<TokenKind>,
    bytes: Vec<u8>,
}

impl Token {
    fn new(location: SourceLocation) -> Self {
        Token {
            location,
            kind: None,
            bytes: Vec::new(),
        }
    }

    fn done(&mut self) -> Token {
        std::mem::replace(self, Token::new(self.location))
    }

    fn add_first_byte(&mut self, byte: u8) -> Option<Result<Token, LexicalError>> {
        match byte {
            b'\n' => {
                self.location.newline();
                self.kind = Some(TokenKind::Newline);
                Some(Ok(self.done()))
            }
            _ => None,
        }
    }

    fn add(&mut self, byte: u8) -> Option<Result<Token, LexicalError>> {
        self.location.advance();
        let Some(ref mut token_kind) = self.kind else {
            return self.add_first_byte(byte);
        };
        None
    }
}

pub struct Lexer<I: Iterator<Item = u8>> {
    active_token: Token,
    iter: I,
}

impl<I: Iterator<Item = u8>> Lexer<I> {
    pub fn lex(ascii_source_bytes: impl IntoIterator<IntoIter = I>) -> Self {
        Lexer {
            iter: ascii_source_bytes.into_iter(),
            active_token: Token::new(SourceLocation::ZERO),
        }
    }
}

impl<'a> Lexer<Bytes<'a>> {
    pub fn lex_str(ascii_source: &'a str) -> Self {
        Self::lex(ascii_source.bytes())
    }
}

impl<I: Iterator<Item = u8>> Iterator for Lexer<I> {
    type Item = Result<Token, LexicalError>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let byte = self.iter.next()?;
            let result = self.active_token.add(byte);
            if result.is_some() {
                return result;
            }
        }
    }
}
