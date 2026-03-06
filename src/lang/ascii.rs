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
        <&$crate::lang::ascii::AsciiStr>::try_from($s).unwrap()
    }};
}

/// Byte slice guaranteed to make up a valid
/// ASCII string.s
#[repr(transparent)]
#[derive(Debug, PartialEq, Eq)]
pub struct AsciiStr([u8]);

impl AsciiStr {
    /// The length of the string.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether the string is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns an iterator over each character (byte)
    /// in the ASCII string.
    pub fn chars(&self) -> impl Iterator<Item = u8> {
        self.0.iter().cloned()
    }

    /// Whether all the letters in the string are uppercase.
    pub fn is_uppercase(&self) -> bool {
        self.0.iter().all(|c| c.is_ascii_uppercase())
    }

    /// Make the string uppercase in place.
    pub fn uppercase(&mut self) {
        for b in &mut self.0 {
            if b.is_ascii_lowercase() {
                *b = b.to_ascii_uppercase();
            }
        }
    }

    /// Make the string lowercase in place.
    pub fn lowercase(&mut self) {
        for b in &mut self.0 {
            if b.is_ascii_uppercase() {
                *b = b.to_ascii_lowercase();
            }
        }
    }

    /// Whether all the letters in the string are uppercase.
    pub fn is_lowercase(&self) -> bool {
        self.0.iter().all(|c| c.is_ascii_lowercase())
    }
}

impl Index<usize> for AsciiStr {
    type Output = u8;

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
        TryFrom::try_from(&self.0[index]).unwrap()
    }
}

impl IndexMut<Range<usize>> for AsciiStr {
    fn index_mut(&mut self, index: Range<usize>) -> &mut Self::Output {
        TryFrom::try_from(&mut self.0[index]).unwrap()
    }
}

impl AsRef<[u8]> for AsciiStr {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl AsRef<str> for AsciiStr {
    fn as_ref(&self) -> &str {
        unsafe { std::str::from_utf8_unchecked(&self.0) }
    }
}

impl TryFrom<&[u8]> for &AsciiStr {
    type Error = u8;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.is_ascii() {
            Ok(unsafe { &*(&value[..] as *const [u8] as *const AsciiStr) })
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
pub struct AsciiString(Vec<u8>);

impl AsciiString {
    /// Create a reference-counted slice out of the string.
    pub fn into_slice(self) -> AsciiSlice {
        self.into()
    }
}

impl TryFrom<Vec<u8>> for AsciiString {
    type Error = u8;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        if value.is_ascii() {
            Ok(Self(value))
        } else {
            Err(value.into_iter().find(|b| !b.is_ascii()).unwrap())
        }
    }
}

impl TryFrom<String> for AsciiString {
    type Error = char;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.is_ascii() {
            Ok(Self(value.into_bytes()))
        } else {
            Err(value.chars().find(|c| !c.is_ascii()).unwrap())
        }
    }
}

impl Deref for AsciiString {
    type Target = AsciiStr;

    fn deref(&self) -> &Self::Target {
        TryFrom::try_from(&self.0[..]).unwrap()
    }
}

impl DerefMut for AsciiString {
    fn deref_mut(&mut self) -> &mut Self::Target {
        TryFrom::try_from(&mut self.0[..]).unwrap()
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
            end: value.len(),
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
        self.source.as_ref() == other.source.as_ref()
    }
}

impl Eq for AsciiSlice {}
