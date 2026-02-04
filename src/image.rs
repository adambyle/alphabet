//! Images are compressed representations of VM memory, useful for storing
//! and loading programs and states to and from files or other streams.
//!
//! Images do not retain register values or the program counter.
//!
//! The [`Image`] API provides useful methods to construct
//! Alphabet programs, as well as providing services
//! for compilers and assemblers.

/// An entry in the image associated with a block of VM memory.
pub struct ImageEntry {
    block_index: u16,
    data_offset: usize,
    data: Vec<u8>,
}

/// A compressed representation of VM memory.
pub struct Image {}

pub struct ImageBuilder {}
