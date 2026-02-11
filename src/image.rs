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

use crate::vm::{Block, Vm};

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
    Entries(Box<dyn Iterator<Item = &'a ImageEntry> + 'a>),

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
        let entries = self.entries.iter();
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
