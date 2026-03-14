//! Serialized format for VM programs and data.
//!
//! [Images](Image) are made up of [entries](ImageEntry), which are bundles of
//! binary data with the address they belong at. In practice, loading an image
//! entails creating a new VM and writing each entry sequentially. The [`Vm`]
//! API does not care if these entries overlap or extend past the end of memory;
//! these errors are ignored. The [`ImageBuilder`] API, however, enforces these rules.
//!
//! The [`vm`](crate::vm) API has convenience methods for saving and loading images without
//! needing to use anything in this module.
//!
//! The [`ImageBuilder`] struct in this module is useful for assemblers, compilers, and
//! other cases where programs need to be built in-memory.

use std::{
    borrow::Cow,
    error::Error,
    fmt::Display,
    io::{self, BufReader, Read, Write},
    iter,
};

use crate::{
    is::Instruction,
    vm::{Block, BlockIndex, BlockOffset, ByteAddress, Vm},
};

#[cfg(test)]
mod tests;

/// An issue with creating an image
/// from a builder.
#[derive(Debug, Clone)]
pub enum ImageBuildError {
    /// A write was attempted that would extend past the end
    /// address of memory.
    Overflow(ByteAddress, usize),
    /// Two writes overlap.
    Overlap {
        /// The first write.
        write_1: (ByteAddress, usize),
        /// The overlapping write.
        write_2: (ByteAddress, usize),
    },
}

impl Display for ImageBuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Overflow(cursor, len) => write!(
                f,
                "write at location {:#80X} of length {} overflows",
                cursor.value(),
                len,
            ),
            Self::Overlap { write_1, write_2 } => write!(
                f,
                "write at location {:#80X} of length {} overlaps with write at location {:#80X} of length {}",
                write_2.0.value(),
                write_2.1,
                write_1.0.value(),
                write_1.1,
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
                    .flat_map(|half_word| half_word.to_be_bytes());
                vec.extend(bytes);
            }
            Self::Words(words) => {
                let bytes = words.into_iter().flat_map(|word| word.to_be_bytes());
                vec.extend(bytes);
            }
        }
    }
}

const WRITE_MERGE_BUFFER: u32 = 8;

struct ImageWrite {
    cursor: ByteAddress,
    payload: ImageWritePayload,
    merge_padding: Option<usize>,
}

impl ImageWrite {
    fn end(&self) -> u32 {
        self.cursor.value() + (self.payload.len() as u32 - 1)
    }
}

/// API for constructing VM images.
///
/// This API is useful for compilers and assemblers
/// to synthesize image data in an efficient way,
/// and then either store it immediately to an image or
/// load it to a VM.
///
/// Most methods take ownership of the `ImageBuilder` to enable method
/// chaining. If any of these methods put the builder
/// [in an error state](ImageBuildError),
/// further methods will be ignored and the error will be returned from
/// [`ImageBuilder::entries`] or [`ImageBuilder::build`].
///
/// # API
///
/// The `ImageBuilder` tracks a [cursor](ImageBuilder::cursor) that
/// starts at 0 and advances automatically when data is written. The
/// cursor can be moved with [`ImageBuilder::seek`] and [`ImageBuilder::advance`].
///
/// Multiple `write` methods are provided for different sizes and kinds
/// of data. For maximum efficiency, add writes in address order. The
/// implementation sorts writes automatically otherwise, and the outputted
/// image is guaranteed to have its entries in order.
///
/// # Errors
///
/// The `ImageBuilder` enforces safe and consistent writes to memory,
/// even though an [`Image`] does not carry the same guarantees. It is an
/// error to:
///
/// - write past the end of memory ([`ImageBuildError::Overflow`]).
/// - overlap one write with another ([`ImageBuildError::Overlap`]).
///
/// These rules help the builder output a space-efficient image.
///
/// # Example
///
/// See [crate-level documentation](crate) for a good example of `ImageBuider`.
pub struct ImageBuilder {
    cursor: ByteAddress,
    sequential: bool,
    writes: Vec<ImageWrite>,
    error: Option<ImageBuildError>,
}

impl ImageBuilder {
    /// Start a new image.
    pub fn new() -> Self {
        Self {
            cursor: 0.into(),
            sequential: true,
            writes: Vec::new(),
            error: None,
        }
    }

    /// Get the position of the cursor.
    pub fn cursor(&self) -> ByteAddress {
        self.cursor
    }

    /// Move the cursor to the specified byte address.
    ///
    /// **Note**: It is most efficient to only move the cursor
    /// forward. See [`advance`](Self::advance).
    pub fn seek(mut self, cursor: ByteAddress) -> Self {
        if cursor < self.cursor {
            self.sequential = false;
        }
        self.cursor = cursor;
        self
    }

    /// Move the cursor forward the specified amount of bytes.
    ///
    /// It is an [error](ImageBuildError::Overflow) for the address to overflow.
    pub fn advance(mut self, offset: u32) -> Self {
        self.advance_checked(offset as usize);
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

    /// If the builder is in an error state from a previous write,
    /// return the cause of the error.
    pub fn error(&self) -> Option<ImageBuildError> {
        self.error.clone()
    }

    fn write(&mut self, payload: ImageWritePayload) {
        let len = payload.len();
        let mut write = ImageWrite {
            payload,
            cursor: self.cursor,
            merge_padding: None,
        };

        // If writes have so far been sequential, no need
        // for checking.
        if self.sequential {
            if let Some(previous) = self.writes.last_mut() {
                let end_previous = previous.end();
                let diff = self.cursor.value() - end_previous - 1;
                if diff < WRITE_MERGE_BUFFER {
                    previous.merge_padding = Some(diff as usize);
                }
            }
            self.writes.push(write);
            self.advance_checked(len);
            return;
        }

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
        let end = write.end();
        let end_previous = if index > 0 {
            let previous = &self.writes[index - 1];
            let end_previous = previous.end();
            if end_previous >= self.cursor.value() {
                self.error.get_or_insert(ImageBuildError::Overlap {
                    write_1: (previous.cursor, previous.payload.len()),
                    write_2: (self.cursor, len),
                });
                return;
            }
            Some(end_previous)
        } else {
            None
        };
        let start_next = if index < self.writes.len() {
            let next = &self.writes[index];
            if end >= next.cursor.value() {
                self.error.get_or_insert(ImageBuildError::Overlap {
                    write_2: (self.cursor, len),
                    write_1: (next.cursor, next.payload.len()),
                });
                return;
            }
            Some(next.cursor)
        } else {
            None
        };

        // Insert new write. Merge writes if possible.
        // (Within 8 bytes justifies merging.)
        if let Some(end_previous) = end_previous {
            let diff = self.cursor.value() - end_previous - 1;
            if diff < WRITE_MERGE_BUFFER {
                self.writes[index - 1].merge_padding = Some(diff as usize);
            }
        }
        if let Some(start_next) = start_next {
            let diff = start_next.value() - end - 1;
            if diff < WRITE_MERGE_BUFFER {
                write.merge_padding = Some(diff as usize);
            }
        }
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
    ///
    /// This automatically word-aligns the cursor, rounding up.
    pub fn write_instruction(mut self, instruction: &Instruction) -> Self {
        // Align the cursor. Round up.
        self.cursor = self.cursor.word_address_round_up().0.into();
        self.write_word(instruction.encode())
    }

    /// Write a sequence of instructions at the cursor.
    ///
    /// This automatically word-aligns the cursor, rounding up.
    pub fn write_instructions(mut self, instructions: &[Instruction]) -> Self {
        // Align the cursor. Round up.
        self.cursor = self.cursor.word_address_round_up().0.into();
        self.write_words(instructions.iter().map(|i| i.encode()).collect())
    }

    /// Consume the builder, outputting the entries that
    /// make up the resulting image.
    ///
    /// See [`build`](Self::build) to create a [`Vm`] or [`Image`] directly.
    ///
    /// # Errors
    ///
    /// This returns the error if the builder is in an error state. See
    /// struct-level documentation for details on error states.
    pub fn entries(self) -> Result<ImageEntries<'static>, ImageBuildError> {
        if let Some(err) = self.error {
            return Err(err);
        }

        let mut entries = Vec::new();
        let mut active_entry: Option<ImageEntry> = None;
        for write in self.writes {
            let mut entry = active_entry.unwrap_or(ImageEntry {
                address: write.cursor,
                data: Vec::new(),
            });
            write.payload.write_to(&mut entry.data);
            if let Some(merge_padding) = write.merge_padding {
                // Extend active entry, but do not write it.
                entry.data.extend(iter::repeat_n(0, merge_padding));
                active_entry = Some(entry);
                continue;
            }
            // Finish entry.
            entries.push(entry);
            active_entry = None;
        }
        debug_assert!(active_entry.is_none(), "last entry had merge padding");
        Ok(ImageEntries::Entries(Box::new(
            entries.into_iter().map(Into::into),
        )))
    }

    /// Consume the builder, building into an image or VM.
    ///
    /// # Errors
    ///
    /// This returns the error if the builder is in an error state. See
    /// struct-level documentation for details on error states.
    pub fn build<T: FromIterator<ImageEntryRef<'static>>>(self) -> Result<T, ImageBuildError> {
        let entries = self.entries()?;
        Ok(FromIterator::from_iter(entries))
    }
}

impl Default for ImageBuilder {
    fn default() -> Self {
        Self::new()
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
    BadOffsets {
        /// The start offset.
        start: ByteAddress,
        /// The end offset.
        end: ByteAddress,
    },
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
                    "end offset {:#80X} comes before start offset {:#80X}",
                    start.value(),
                    end.value(),
                )
            }
        }
    }
}

impl Error for ImageEntryError {}

/// An entry in the image associated with a sequence of bytes
/// in memory.
///
/// An entry is simply an address followed by the bytes to be
/// written at that address.
pub struct ImageEntry {
    address: ByteAddress,
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
    ///
    /// # Errors
    ///
    /// See [`ImageEntryError`]. Although consumers of `ImageEntry` can handle
    /// malformed entries, this API prevents invalid representations.
    pub fn new(address: ByteAddress, data: Vec<u8>) -> Result<Self, ImageEntryError> {
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
                start: start_offset.into(),
                end: end_offset.into(),
            });
        }
        let length = end_offset as usize - start_offset as usize + 1;
        let data: Vec<u8> = iter.by_ref().take(length).collect();
        Ok(Self {
            address: start_offset.into(),
            data,
        })
    }

    /// The address of the start of the data.
    pub fn address(&self) -> ByteAddress {
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

/// An owned or borrowed image entry.
///
/// The `ImageEntryRef` API is used in the [`ImageEntries`]
/// iterator to support adaptation from both owned and borrowed
/// image sources.
pub struct ImageEntryRef<'a> {
    address: ByteAddress,
    data: Cow<'a, [u8]>,
}

impl<'a> ImageEntryRef<'a> {
    /// The address of the start of the data.
    pub fn address(&self) -> ByteAddress {
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
        let start = self.address.value();
        let end = start + (self.data.len() - 1) as u32;
        let start_bytes = start.to_be_bytes();
        let end_bytes = end.to_be_bytes();
        writer.write_all(&start_bytes)?;
        writer.write_all(&end_bytes)?;
        writer.write_all(&self.data)?;
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
/// to be written to a VM or file.
///
/// The `ImageEntries` iterator is a flexible adapter between
/// image media. It can facilitate efficient data transfer between
/// [`Vm`] instances, [`Image`] structs, and files (or other readers
/// and writers).
///
/// You will not often used this API directly; each medium has its own
/// helper methods for conversion that leverage `ImageEntries` internally.
pub enum ImageEntries<'a> {
    /// The image entries are produced from each block
    /// of written VM memory.
    Vm {
        /// A reference to the virtual machine the entries come from.
        vm: &'a Vm,
        /// The current block index being read from.
        block_index: BlockIndex,
    },

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
                loop {
                    let current_block_index = *block_index;
                    let block = vm.block(current_block_index);

                    // Advance before any early returns so we never re-visit this block.
                    let (next, overflow) = block_index.value().overflowing_add(1);
                    *block_index = next.into();

                    let Block::Memory(memory) = block else {
                        if overflow {
                            return None;
                        }
                        continue;
                    };

                    let Some(start) = memory.iter().position(|&b| b != 0) else {
                        if overflow {
                            return None;
                        }
                        continue;
                    };
                    let start_offset = BlockOffset::from(start as u16);
                    let end = memory.iter().rposition(|&b| b != 0).unwrap();
                    let non_zero = &memory[start..=end];
                    let address = current_block_index + start_offset;
                    return Some(ImageEntryRef {
                        address,
                        data: Cow::Borrowed(non_zero),
                    });
                }
            }
            Self::Entries(entries) => entries.next(),
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
            block_index: 0.into(),
        }
    }
}

impl<'a, R: Read + 'a> From<R> for ImageEntries<'a> {
    fn from(value: R) -> Self {
        let reader = BufReader::new(value);
        let bytes = reader.bytes().filter_map(|b| b.ok());
        Self::Bytes(Box::new(bytes))
    }
}

/// A compressed representation of VM memory.
///
/// An [`Image`] is the in-memory medium for images. They are
/// simply a wrapper around a [`Vec`] of [`ImageEntry`].
///
/// An `Image` can contain entries that overlap or whose data goes past the
/// last valid address. In the case of overlapping entries, later entries
/// may overwrite earlier ones. Entries have their data cut off at the
/// last valid address.
///
/// The [`ImageBuilder`] API disallows these situations, and the output
/// from [`Vm::image`] guarantees no overlaps or overflows.
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
        let mut split = None;
        self.entries.retain_mut(|entry| {
            let entry_start = entry.address.value();
            let entry_end = entry_start + (entry.data.len() as u32 - 1);

            // No overlap: keep as-is.
            if entry_start > end_address || entry_end < start_address {
                return true;
            }

            // Fully covered: drop entirely.
            if start_address <= entry_start && end_address >= entry_end {
                return false;
            }

            // Overlap from the start: truncate the front.
            if start_address <= entry_start {
                let keep_start = (end_address - entry_start + 1) as usize;
                entry.data.drain(..keep_start);
                entry.address = ByteAddress::from(end_address + 1);
                return true;
            }

            // Overlap from the end — truncate the back.
            if end_address >= entry_end {
                let keep_end = (start_address - entry_start) as usize;
                entry.data.truncate(keep_end);
                return true;
            }

            // Clear range is entirely within entry: split.
            // Retain the left half in place, and stash the right half to insert after.
            let right_start = (end_address - entry_start + 1) as usize;
            let right_data = entry.data[right_start..].to_vec();
            let right_address = ByteAddress::from(end_address + 1);
            entry.data.truncate((start_address - entry_start) as usize);
            split = Some(ImageEntry {
                address: right_address,
                data: right_data,
            });
            true
        });

        // Insert the right half of any split entry at the correct position.
        if let Some(right) = split {
            let index = self
                .entries
                .partition_point(|e| e.address.value() < right.address.value());
            self.entries.insert(index, right);
        }
    }

    /// Write the contents of an image.
    pub fn write_to(&self, writer: impl Write) -> io::Result<()> {
        self.entries().write_to(writer)
    }
}

impl Default for Image {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> FromIterator<ImageEntryRef<'a>> for Image {
    fn from_iter<T: IntoIterator<Item = ImageEntryRef<'a>>>(iter: T) -> Self {
        let entries = iter.into_iter().map(|entry| entry.into()).collect();
        Self { entries }
    }
}

impl<R: Read> From<R> for Image {
    fn from(value: R) -> Self {
        Self::from_iter(ImageEntries::from(value))
    }
}
