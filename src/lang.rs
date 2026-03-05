//! Dedicated language submodules.
//!
//! This module has common language features.

pub mod alpha;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SourceLocation {
    line: usize,
    column: usize,
}

impl SourceLocation {
    pub const ZERO: Self = SourceLocation { line: 0, column: 0 };

    pub fn advance(&mut self) {
        self.column += 1;
    }

    pub fn newline(&mut self) {
        self.line += 1;
        self.column = 0;
    }
}
