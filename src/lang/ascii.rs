//! Custom ASCII text handling.

use std::{
    borrow::{Borrow, BorrowMut},
    ops::{Deref, DerefMut, Index, IndexMut, Range},
    rc::Rc,
};

#[macro_export]
/// Convenience macro for ASCII string literals.
///
/// This macro verifies at compile time that the [`&str`](str) provided
/// as an argument to the macro contains only ASCII characters, and
/// it converts the argument to [`&AsciiStr`](AsciiStr).
macro_rules! ascii {
    ($s:expr) => {{
        const _: () = assert!($s.is_ascii(), "string is not valid ASCII");
        unsafe { &*($s as *const str as *const AsciiStr) }
    }};
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// A transparent wrapper around [`u8`] the guarantees it is a valid
/// ASCII byte.
pub struct AsciiByte(u8);

impl AsciiByte {
    /// The wrapped byte value.
    pub fn byte(self) -> u8 {
        self.0
    }
}

impl TryFrom<u8> for AsciiByte {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value.is_ascii() {
            Ok(AsciiByte(value))
        } else {
            Err(())
        }
    }
}

impl From<AsciiByte> for u8 {
    fn from(value: AsciiByte) -> Self {
        value.0
    }
}

/// Byte slice guaranteed to make up a valid
/// ASCII string.
///
/// This type's API is minimal; it is mostly designed
/// for consumption by a lexer. See [`AsciiStr::char_locations`].
#[repr(transparent)]
#[derive(Debug, PartialEq, Eq)]
pub struct AsciiStr([AsciiByte]);

impl Index<usize> for AsciiStr {
    type Output = AsciiByte;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl IndexMut<usize> for AsciiStr {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index]
    }
}

impl Index<Range<usize>> for AsciiStr {
    type Output = AsciiStr;

    fn index(&self, index: Range<usize>) -> &Self::Output {
        self.0[index].into()
    }
}

impl IndexMut<Range<usize>> for AsciiStr {
    fn index_mut(&mut self, index: Range<usize>) -> &mut Self::Output {
        (&mut self.0[index]).into()
    }
}

impl AsRef<[AsciiByte]> for AsciiStr {
    fn as_ref(&self) -> &[AsciiByte] {
        &self.0
    }
}

impl AsMut<[AsciiByte]> for AsciiStr {
    fn as_mut(&mut self) -> &mut [AsciiByte] {
        &mut self.0
    }
}

impl AsRef<[u8]> for AsciiStr {
    fn as_ref(&self) -> &[u8] {
        unsafe { &*(&self.0 as *const [AsciiByte] as *const [u8]) }
    }
}

impl AsRef<str> for AsciiStr {
    fn as_ref(&self) -> &str {
        unsafe { std::str::from_utf8_unchecked(self.as_ref()) }
    }
}

impl From<&[AsciiByte]> for &AsciiStr {
    fn from(value: &[AsciiByte]) -> Self {
        unsafe { &*(value as *const [AsciiByte] as *const AsciiStr) }
    }
}

impl From<&mut [AsciiByte]> for &mut AsciiStr {
    fn from(value: &mut [AsciiByte]) -> Self {
        unsafe { &mut *(value as *mut [AsciiByte] as *mut AsciiStr) }
    }
}

impl TryFrom<&[u8]> for &AsciiStr {
    type Error = u8;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.is_ascii() {
            Ok(unsafe { &*(value as *const [u8] as *const AsciiStr) })
        } else {
            Err(value.iter().cloned().find(|b| !b.is_ascii()).unwrap())
        }
    }
}

impl TryFrom<&mut [u8]> for &mut AsciiStr {
    type Error = u8;

    fn try_from(value: &mut [u8]) -> Result<Self, Self::Error> {
        if value.is_ascii() {
            Ok(unsafe { &mut *(&mut value[..] as *mut [u8] as *mut AsciiStr) })
        } else {
            Err(value.iter().cloned().find(|b| !b.is_ascii()).unwrap())
        }
    }
}

impl TryFrom<&str> for &AsciiStr {
    type Error = char;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if value.is_ascii() {
            Ok(unsafe { &*(value as *const str as *const AsciiStr) })
        } else {
            Err(value.chars().find(|c| !c.is_ascii()).unwrap())
        }
    }
}

impl TryFrom<&mut str> for &mut AsciiStr {
    type Error = char;

    fn try_from(value: &mut str) -> Result<Self, Self::Error> {
        if value.is_ascii() {
            Ok(unsafe { &mut *(value as *mut str as *mut AsciiStr) })
        } else {
            Err(value.chars().find(|c| !c.is_ascii()).unwrap())
        }
    }
}

impl ToOwned for AsciiStr {
    type Owned = AsciiString;

    fn to_owned(&self) -> Self::Owned {
        AsciiString(self.0.to_owned())
    }
}

/// Owned ASCII string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AsciiString(Vec<AsciiByte>);

impl From<Vec<AsciiByte>> for AsciiString {
    fn from(value: Vec<AsciiByte>) -> Self {
        AsciiString(value)
    }
}

impl FromIterator<AsciiByte> for AsciiString {
    fn from_iter<T: IntoIterator<Item = AsciiByte>>(iter: T) -> Self {
        AsciiString(iter.into_iter().collect())
    }
}

impl TryFrom<Vec<u8>> for AsciiString {
    type Error = u8;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        if value.is_ascii() {
            Ok(Self(unsafe {
                std::mem::transmute::<Vec<u8>, Vec<AsciiByte>>(value)
            }))
        } else {
            Err(value.into_iter().find(|b| !b.is_ascii()).unwrap())
        }
    }
}

impl TryFrom<String> for AsciiString {
    type Error = char;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.is_ascii() {
            Ok(Self(unsafe {
                std::mem::transmute::<Vec<u8>, Vec<AsciiByte>>(value.into_bytes())
            }))
        } else {
            Err(value.chars().find(|c| !c.is_ascii()).unwrap())
        }
    }
}

impl Deref for AsciiString {
    type Target = AsciiStr;

    fn deref(&self) -> &Self::Target {
        self.0.as_slice().into()
    }
}

impl DerefMut for AsciiString {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.as_mut_slice().into()
    }
}

impl Borrow<AsciiStr> for AsciiString {
    fn borrow(&self) -> &AsciiStr {
        self.deref()
    }
}

impl BorrowMut<AsciiStr> for AsciiString {
    fn borrow_mut(&mut self) -> &mut AsciiStr {
        self.deref_mut()
    }
}

/// A reference-counted slice of an owned ASCII string.
#[derive(Debug, Clone)]
pub struct AsciiSlice {
    source: Rc<AsciiString>,
    start: usize,
    end: usize,
}

impl From<AsciiString> for AsciiSlice {
    fn from(value: AsciiString) -> Self {
        AsciiSlice {
            start: 0,
            end: value.0.len(),
            source: Rc::new(value),
        }
    }
}

impl AsciiSlice {
    /// Create a slice of the ASCII string.
    pub fn slice(&self, range: Range<usize>) -> AsciiSlice {
        let start = (self.start + range.start).min(self.end);
        let end = (self.start + range.end).min(self.end);
        AsciiSlice {
            source: Rc::clone(&self.source),
            start,
            end,
        }
    }
}

impl Deref for AsciiSlice {
    type Target = AsciiStr;

    fn deref(&self) -> &Self::Target {
        &self.source[self.start..self.end]
    }
}

impl PartialEq for AsciiSlice {
    fn eq(&self, other: &Self) -> bool {
        **self == **other
    }
}

impl Eq for AsciiSlice {}

#[derive(Debug, Clone)]
/// A shared or borrowed reference to an ASCII string.
pub enum AsciiRef<'a> {
    /// The reference is a slice into some multiple-owned ASCII.
    Shared(AsciiSlice),
    /// The reference is a slice into some ASCII.
    Borrowed(&'a AsciiStr),
}

impl AsciiRef<'_> {
    /// Create a slice of the ASCII string.
    pub fn slice(&self, range: Range<usize>) -> Self {
        match self {
            Self::Shared(shared_slice) => Self::Shared(shared_slice.slice(range)),
            Self::Borrowed(string) => Self::Borrowed(&string[range]),
        }
    }
}

impl Deref for AsciiRef<'_> {
    type Target = AsciiStr;

    fn deref(&self) -> &Self::Target {
        match self {
            AsciiRef::Shared(shared_slice) => shared_slice,
            AsciiRef::Borrowed(string) => string,
        }
    }
}

impl PartialEq for AsciiRef<'_> {
    fn eq(&self, other: &Self) -> bool {
        **self == **other
    }
}
