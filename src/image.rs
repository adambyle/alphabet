//! Images are compressed representations of VM memory, useful for storing
//! and loading programs and states to and from files or other streams.
//!
//! Images do not retain register values or the program counter.
//!
//! The [`Image`] API provides useful methods to construct
//! Alphabet programs, as well as providing services
//! for compilers and assemblers.

use std::{
    borrow::Cow,
    error::Error,
    fmt::Display,
    io::{self, Read, Write},
};

use crate::{
    is::Instruction,
    vm::{Block, Vm},
};

/// An issue with creating an image
/// from a builder.
#[derive(Debug)]
pub enum ImageBuildError {
    /// A write was attempted that would extend past the end
    /// address of memory.
    Overflow(u32, usize),
    /// Two writes overlap.
    Overlap {
        /// The first write.
        write_1: (u32, usize),
        /// The overlapping write.
        write_2: (u32, usize),
    },
}

impl Display for ImageBuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Overflow(cursor, len) => write!(
                f,
                "write at location {cursor:80X} of length {len} overflows"
            ),
            Self::Overlap { write_1, write_2 } => write!(
                f,
                "write at location {:#80X} of length {} overlaps with write at location {:#80X} of length {}",
                write_2.0, write_2.1, write_1.0, write_1.1,
            ),
        }
    }
}

impl Error for ImageBuildError {}

enum ImageWritePayload {
    Byte(u8),
    HalfWord(u16),
    Word(u32),
    Bytes(Vec<u8>),
    HalfWords(Vec<u16>),
    Words(Vec<u32>),
}

impl ImageWritePayload {
    fn len(&self) -> usize {
        match self {
            Self::Byte(_) => 1,
            Self::HalfWord(_) => 2,
            Self::Word(_) => 4,
            Self::Bytes(bytes) => bytes.len(),
            Self::HalfWords(half_words) => 2 * half_words.len(),
            Self::Words(words) => 4 * words.len(),
        }
    }

    fn write_to(self, vec: &mut Vec<u8>) {
        match self {
            Self::Byte(byte) => vec.push(byte),
            Self::HalfWord(half_word) => vec.extend(half_word.to_be_bytes()),
            Self::Word(word) => vec.extend(word.to_be_bytes()),
            Self::Bytes(bytes) => vec.extend(bytes),
            Self::HalfWords(half_words) => {
                let bytes = half_words
                    .into_iter()
                    .map(|half_word| half_word.to_be_bytes())
                    .flatten();
                vec.extend(bytes);
            }
            Self::Words(words) => {
                let bytes = words.into_iter().map(|word| word.to_be_bytes()).flatten();
                vec.extend(bytes);
            }
        }
    }
}

struct ImageWrite {
    cursor: u32,
    payload: ImageWritePayload,
}

/// Helper API for constructing VM images.
///
/// This API is useful for compilers and assemblers
/// to synthesize image data in an efficient way,
/// and then either store it immediately to an image or
/// load it to a VM.
pub struct ImageBuilder {
    cursor: u32,
    writes: Vec<ImageWrite>,
    error: Option<ImageBuildError>,
}

impl ImageBuilder {
    /// Create a new image that organizes its
    /// writes into an entry per VM block.
    pub fn new() -> Self {
        Self {
            cursor: 0,
            writes: Vec::new(),
            error: None,
        }
    }

    /// Move the cursor to the specified location.
    pub fn seek(mut self, cursor: u32) -> Self {
        self.cursor = cursor;
        self
    }

    fn advance_checked(&mut self, len: usize) {
        let (next_cursor, overflow) = self.cursor.overflowing_add(len as u32);
        if overflow {
            self.error
                .get_or_insert(ImageBuildError::Overflow(self.cursor, len));
        }
        self.cursor = next_cursor;
    }

    fn write(&mut self, payload: ImageWritePayload) {
        let len = payload.len();
        let write = ImageWrite {
            payload,
            cursor: self.cursor,
        };

        // Determine insertion point (writes maintain order).
        let index = self.writes.binary_search_by_key(&self.cursor, |w| w.cursor);
        let index = match index {
            Err(index) => index,
            Ok(index) => {
                // Write already exists at this offset.
                let existing = &self.writes[index];
                self.error.get_or_insert(ImageBuildError::Overlap {
                    write_1: (existing.cursor, existing.payload.len()),
                    write_2: (self.cursor, len),
                });
                return;
            }
        };

        // Check for overlaps.
        if index > 0 {
            let previous = &self.writes[index - 1];
            let end_previous = previous.cursor + (previous.payload.len() as u32 - 1);
            if end_previous >= self.cursor {
                self.error.get_or_insert(ImageBuildError::Overlap {
                    write_1: (previous.cursor, previous.payload.len()),
                    write_2: (self.cursor, len),
                });
                return;
            }
        }
        if index < self.writes.len() - 1 {
            let next = &self.writes[index + 1];
            let end = self.cursor + (len as u32 - 1);
            if end >= next.cursor {
                self.error.get_or_insert(ImageBuildError::Overlap {
                    write_1: (next.cursor, next.payload.len()),
                    write_2: (self.cursor, len),
                });
                return;
            }
        }

        // Insert new write.
        // TODO merge writes if possible.
        // (Within 8 bytes justifies merging.)
        self.writes.insert(index, write);
        self.advance_checked(len);
    }

    /// Write a byte of data at the cursor.
    pub fn write_byte(mut self, byte: u8) -> Self {
        if self.error.is_some() {
            return self;
        }
        self.write(ImageWritePayload::Byte(byte));
        self
    }

    /// Write a sequence of bytes at the cursor.
    pub fn write_bytes(mut self, bytes: Vec<u8>) -> Self {
        if bytes.is_empty() {
            return self;
        }
        if self.error.is_some() {
            return self;
        }
        self.write(ImageWritePayload::Bytes(bytes));
        self
    }

    /// Write an ASCII-encoded string at the cursor.
    pub fn write_ascii(self, string: String) -> Self {
        self.write_bytes(string.into_bytes())
    }

    /// Write a half-word of data at the cursor.
    pub fn write_half_word(mut self, half_word: u16) -> Self {
        if self.error.is_some() {
            return self;
        }
        self.write(ImageWritePayload::HalfWord(half_word));
        self
    }

    /// Write a sequence of half-words at the cursor.
    pub fn write_half_words(mut self, half_words: Vec<u16>) -> Self {
        if half_words.is_empty() {
            return self;
        }
        if self.error.is_some() {
            return self;
        }
        self.write(ImageWritePayload::HalfWords(half_words));
        self
    }

    /// Write a word of data at the cursor.
    pub fn write_word(mut self, word: u32) -> Self {
        if self.error.is_some() {
            return self;
        }
        self.write(ImageWritePayload::Word(word));
        self
    }

    /// Write a sequence of words at the cursor.
    pub fn write_words(mut self, words: Vec<u32>) -> Self {
        if words.is_empty() {
            return self;
        }
        if self.error.is_some() {
            return self;
        }
        self.write(ImageWritePayload::Words(words));
        self
    }

    /// Write an instruction at the cursor.
    /// This automatically word-aligns the cursor.
    pub fn write_instruction(mut self, instruction: &Instruction) -> Self {
        // Align the cursor. Round up.
        self.advance_checked(3);
        self.cursor >>= 2;
        self.cursor <<= 2;

        self.write_word(instruction.encode())
    }

    /// Write a sequence of instructions at the cursor.
    /// This automatically word-aligns the cursor.
    pub fn write_instructions(mut self, instructions: &[Instruction]) -> Self {
        // Align the cursor. Round up.
        self.advance_checked(3);
        self.cursor >>= 2;
        self.cursor <<= 2;

        self.write_words(instructions.iter().map(|i| i.encode()).collect())
    }

    /// Consume the builder, outputting the entries that
    /// make up the resulting image.
    pub fn entries(self) -> Result<ImageEntries<'static>, ImageBuildError> {
        if let Some(err) = self.error {
            return Err(err);
        }

        // TODO add optimizations. Right now we just write all raw entries.
        // TODO error on overlap.
        let mut entries = Vec::new();
        for ImageWrite { cursor, payload } in self.writes {
            let mut data = Vec::new();
            payload.write_to(&mut data);
            entries.push(ImageEntry {
                address: cursor,
                data,
            });
        }
        Ok(ImageEntries::Entries(Box::new(
            entries.into_iter().map(Into::into),
        )))
    }

    /// Consume the builder, c
    pub fn build<T: FromIterator<ImageEntryRef<'static>>>(self) -> Result<T, ImageBuildError> {
        let entries = self.entries()?;
        Ok(FromIterator::from_iter(entries))
    }
}

/// The entry data was invalid.
#[derive(Debug)]
pub enum ImageEntryError {
    /// The data extends past the valid address.
    Overflow,
    /// Start and end offset values were not provided.
    Incomplete,
    /// The data is empty.
    Empty,
    /// The end offset is less than the start offset.
    BadOffsets { start: u32, end: u32 },
}

impl Display for ImageEntryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => write!(f, "data in entry is empty"),
            Self::Incomplete => write!(f, "entry start and end offsets not provided"),
            Self::Overflow => write!(f, "data in entry extends past the last valid address"),
            Self::BadOffsets { start, end } => {
                write!(
                    f,
                    "end offset {end:80X} comes before start offset {start:80X}",
                )
            }
        }
    }
}

impl Error for ImageEntryError {}

/// An entry in the image associated with a sequence of bytes
/// in memory.
pub struct ImageEntry {
    address: u32,
    data: Vec<u8>,
}

fn next_be_u32(iter: &mut impl Iterator<Item = u8>) -> Option<u32> {
    let mut bytes = [0u8; 4];
    for byte in &mut bytes {
        *byte = iter.next()?;
    }
    Some(u32::from_be_bytes(bytes))
}

impl ImageEntry {
    /// Create a new image entry.
    pub fn new(address: u32, data: Vec<u8>) -> Result<Self, ImageEntryError> {
        if data.is_empty() {
            return Err(ImageEntryError::Empty);
        }
        let length = data.len() as u32;
        let (_, overflow) = address.overflowing_add(length - 1);
        if overflow {
            return Err(ImageEntryError::Overflow);
        }
        Ok(Self { address, data })
    }

    fn from_byte_iter(iter: &mut impl Iterator<Item = u8>) -> Result<Self, ImageEntryError> {
        let start_offset = next_be_u32(iter).ok_or(ImageEntryError::Incomplete)?;
        let end_offset = next_be_u32(iter).ok_or(ImageEntryError::Incomplete)?;
        if end_offset < start_offset {
            return Err(ImageEntryError::BadOffsets {
                start: start_offset,
                end: end_offset,
            });
        }
        let length = end_offset as usize - start_offset as usize + 1;
        let data: Vec<u8> = iter.by_ref().take(length).collect();
        Ok(Self {
            address: start_offset,
            data,
        })
    }

    /// The address of the start of the data.
    pub fn address(&self) -> u32 {
        self.address
    }

    /// The data in the chunk.
    pub fn data(&self) -> &[u8] {
        &self.data
    }
}

impl<'a> From<ImageEntryRef<'a>> for ImageEntry {
    fn from(value: ImageEntryRef<'a>) -> Self {
        Self {
            address: value.address,
            data: value.data.into_owned(),
        }
    }
}

/// A reference to data in an image entry.
pub struct ImageEntryRef<'a> {
    address: u32,
    data: Cow<'a, [u8]>,
}

impl<'a> ImageEntryRef<'a> {
    /// The address of the start of the data.
    pub fn address(&self) -> u32 {
        self.address
    }

    /// The data in the chunk.
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Write the entry according to the [`Image`] format.
    pub fn write_to(&self, mut writer: impl Write) -> io::Result<()> {
        // It is enforced by the API that all entries are valid, meaning:
        // - No entry goes beyond the last address.
        // - All entries are at least 1 in length.
        let start = self.address;
        let end = self.address + (self.data.len() - 1) as u32;
        let start_bytes = start.to_be_bytes();
        let end_bytes = end.to_be_bytes();
        writer.write(&start_bytes)?;
        writer.write(&end_bytes)?;
        writer.write(&self.data)?;
        Ok(())
    }
}

impl From<ImageEntry> for ImageEntryRef<'static> {
    fn from(value: ImageEntry) -> Self {
        Self {
            address: value.address,
            data: Cow::Owned(value.data),
        }
    }
}

impl<'a> From<&'a ImageEntry> for ImageEntryRef<'a> {
    fn from(value: &'a ImageEntry) -> Self {
        Self {
            address: value.address,
            data: Cow::Borrowed(&value.data),
        }
    }
}

/// An iterator producing image entries, chunks of memory
/// to be written to a VM.
pub enum ImageEntries<'a> {
    /// The image entries are produced from each block
    /// of written VM memory.
    Vm { vm: &'a Vm, block_index: u16 },

    /// The image entries are produced from an Image.
    Entries(Box<dyn Iterator<Item = ImageEntryRef<'a>> + 'a>),

    /// The image entries are parsed from binary representation.
    Bytes(Box<dyn Iterator<Item = u8> + 'a>),
}

impl ImageEntries<'_> {
    /// Write the image entries according to the [`Image`] format.
    pub fn write_to(&mut self, mut writer: impl Write) -> io::Result<()> {
        for entry in self {
            entry.write_to(&mut writer)?;
        }
        Ok(())
    }
}

impl<'a> Iterator for ImageEntries<'a> {
    type Item = ImageEntryRef<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Vm { vm, block_index } => {
                while *block_index <= u16::MAX {
                    // Get block of memory, if present.
                    let current_block_index = *block_index;
                    let block = vm.block(current_block_index);
                    *block_index += 1;
                    let Block::Memory(memory) = block else {
                        continue;
                    };

                    // Find start and end of non-zero data.
                    let Some(start) = memory.iter().position(|&b| b != 0) else {
                        continue;
                    };
                    // Unwrap: guaranteed to be at least `start`.
                    let end = memory.iter().rposition(|&b| b != 0).unwrap();
                    let non_zero = &memory[start..=end];
                    let address = ((current_block_index as u32) << 16) | (start as u32);
                    return Some(ImageEntryRef {
                        address,
                        data: Cow::Borrowed(non_zero),
                    });
                }

                // End of memory reached.
                None
            }
            Self::Entries(entries) => entries.next().map(Into::into),
            Self::Bytes(bytes) => loop {
                let entry = ImageEntry::from_byte_iter(bytes);
                match entry {
                    Ok(entry) => return Some(entry.into()),
                    Err(ImageEntryError::Incomplete) => {
                        // End of input; unable to form valid entry.
                        return None;
                    }
                    Err(ImageEntryError::BadOffsets { .. }) => {
                        // Ignore bad entry and assume data length zero.
                        continue;
                    }
                    _ => unreachable!(),
                }
            },
        }
    }
}

impl<'a> From<&'a Vm> for ImageEntries<'a> {
    fn from(value: &'a Vm) -> Self {
        Self::Vm {
            vm: value,
            block_index: 0,
        }
    }
}

impl<'a, R: Read + 'a> From<R> for ImageEntries<'a> {
    fn from(value: R) -> Self {
        let bytes = value.bytes().filter_map(|b| b.ok());
        Self::Bytes(Box::new(bytes))
    }
}

/// A compressed representation of VM memory.
pub struct Image {
    entries: Vec<ImageEntry>,
}

impl Image {
    /// Create an empty image.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Iterate the image entries.
    pub fn entries<'a>(&'a self) -> ImageEntries<'a> {
        let entries = self.entries.iter().map(Into::into);
        ImageEntries::Entries(Box::new(entries))
    }

    /// Add an entry to the image.
    ///
    /// This method does not automatically
    /// handle overlapping entries.
    pub fn add(&mut self, entry: ImageEntry) {
        self.entries.push(entry);
    }

    /// Clear data from the image.
    pub fn clear(&mut self, start_address: u32, end_address: u32) {
        self.entries.retain_mut(|entry| {
            // Determine whether entry is in removal bounds.
            let entry_start = entry.address;
            let data_length = entry.data.len() as u32;
            let entry_end = entry_start + (data_length - 1);
            if entry_start > end_address || entry_end < start_address {
                return true;
            }
            // If entry is totally included, remove completely.
            if start_address <= entry_start && end_address >= entry_end {
                return false;
            }
            // Otherwise modify entry to remove relevant slice.
            let remove_start = if start_address < entry_start {
                0
            } else {
                (start_address - entry_start) as usize
            };
            let remove_end = if end_address > entry_end {
                (data_length - 1) as usize
            } else {
                (end_address - entry_start) as usize
            };
            entry.data.drain(remove_start..remove_end);
            entry.address += remove_start as u32;
            true
        });
    }

    /// Write the contents of an image.
    pub fn write_to(&self, writer: impl Write) -> io::Result<()> {
        self.entries().write_to(writer)
    }
}

impl<'a> FromIterator<ImageEntryRef<'a>> for Image {
    fn from_iter<T: IntoIterator<Item = ImageEntryRef<'a>>>(iter: T) -> Self {
        let entries = iter.into_iter().map(|entry| entry.into()).collect();
        Self { entries }
    }
}
