//! Dedicated language submodules.
//!
//! This module has common language features.

use crate::lang::ascii::AsciiChar;

pub mod alpha;
pub mod ascii;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
/// A location in a unit of source.
pub struct SourceLocation {
    /// The line (or row) of source.
    pub line: usize,
    /// The character in line (or column) of source.
    pub column: usize,
}

impl SourceLocation {
    /// The beginning of source.
    pub const ZERO: Self = SourceLocation { line: 0, column: 0 };

    /// Return the location at the next line.
    pub fn newline(self) -> Self {
        SourceLocation {
            line: self.line + 1,
            column: 0,
        }
    }

    /// The next location following the character
    /// at this location (handles newlines).
    pub fn after_char(self, char: AsciiChar) -> Self {
        if char.byte() == b'\n' {
            self.newline()
        } else {
            SourceLocation {
                column: self.column + 1,
                ..self
            }
        }
    }
}
