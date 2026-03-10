//! Dedicated language submodules.
//!
//! This module has common language features.

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

    /// Move the line down by 1 and reset the column.
    pub fn newline(&mut self) {
        self.line += 1;
        self.column = 0;
    }
}
