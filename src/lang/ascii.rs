//! Custom ASCII text handling.

use std::{
    borrow::{Borrow, BorrowMut},
    cell::RefCell,
    collections::VecDeque,
    fmt::{Debug, Display},
    io,
    iter::{self, Peekable},
    mem,
    ops::{Deref, DerefMut, Index, IndexMut, Range, RangeFrom, RangeFull, RangeTo},
    rc::Rc,
    slice, str, vec,
};

use crate::lang::SourceLocation;

#[macro_export]
/// Convenience macro for ASCII string literals.
///
/// This macro verifies at compile time that the [`&str`](str) provided
/// as an argument to the macro contains only ASCII characters, and
/// it converts the argument to [`&AsciiStr`](AsciiStr).
macro_rules! ascii {
    ($s:expr) => {{
        use $crate::lang::ascii::AsciiStr;
        const _: () = assert!($s.is_ascii(), "string is not valid ASCII");
        unsafe { &*($s as *const str as *const AsciiStr) }
    }};
}

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
/// A transparent wrapper around [`u8`] the guarantees it is a valid
/// ASCII byte.
pub struct AsciiChar(u8);

impl AsciiChar {
    /// The wrapped byte value.
    pub const fn byte(self) -> u8 {
        self.0
    }
}

impl TryFrom<u8> for AsciiChar {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value.is_ascii() {
            Ok(AsciiChar(value))
        } else {
            Err(())
        }
    }
}

impl From<AsciiChar> for u8 {
    fn from(value: AsciiChar) -> Self {
        value.0
    }
}

impl Display for AsciiChar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ch = self.0 as char;
        write!(f, "{ch}")
    }
}

impl Debug for AsciiChar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ch = self.0 as char;
        write!(f, "{ch}")
    }
}

/// Byte slice guaranteed to make up a valid
/// ASCII string.
///
/// This type's API is minimal; it is mostly designed
/// for consumption by a lexer.
#[repr(transparent)]
#[derive(PartialEq, Eq)]
pub struct AsciiStr([AsciiChar]);

impl AsciiStr {
    /// The length in bytes of the string.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether the string is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Return an iterator over the ASCII characters in the string slice.
    pub fn chars(&self) -> <&Self as IntoIterator>::IntoIter {
        self.into_iter()
    }
}

impl Index<usize> for AsciiStr {
    type Output = AsciiChar;

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

impl Index<RangeFrom<usize>> for AsciiStr {
    type Output = AsciiStr;

    fn index(&self, index: RangeFrom<usize>) -> &Self::Output {
        self.0[index].into()
    }
}

impl Index<RangeTo<usize>> for AsciiStr {
    type Output = AsciiStr;

    fn index(&self, index: RangeTo<usize>) -> &Self::Output {
        self.0[index].into()
    }
}

impl Index<RangeFull> for AsciiStr {
    type Output = AsciiStr;

    fn index(&self, index: RangeFull) -> &Self::Output {
        self.0[index].into()
    }
}

impl IndexMut<Range<usize>> for AsciiStr {
    fn index_mut(&mut self, index: Range<usize>) -> &mut Self::Output {
        (&mut self.0[index]).into()
    }
}

impl IndexMut<RangeFrom<usize>> for AsciiStr {
    fn index_mut(&mut self, index: RangeFrom<usize>) -> &mut Self::Output {
        (&mut self.0[index]).into()
    }
}

impl IndexMut<RangeTo<usize>> for AsciiStr {
    fn index_mut(&mut self, index: RangeTo<usize>) -> &mut Self::Output {
        (&mut self.0[index]).into()
    }
}

impl IndexMut<RangeFull> for AsciiStr {
    fn index_mut(&mut self, index: RangeFull) -> &mut Self::Output {
        (&mut self.0[index]).into()
    }
}

impl AsRef<[AsciiChar]> for AsciiStr {
    fn as_ref(&self) -> &[AsciiChar] {
        &self.0
    }
}

impl AsMut<[AsciiChar]> for AsciiStr {
    fn as_mut(&mut self) -> &mut [AsciiChar] {
        &mut self.0
    }
}

impl AsRef<[u8]> for AsciiStr {
    fn as_ref(&self) -> &[u8] {
        unsafe { &*(&self.0 as *const [AsciiChar] as *const [u8]) }
    }
}

impl AsRef<str> for AsciiStr {
    fn as_ref(&self) -> &str {
        unsafe { str::from_utf8_unchecked(self.as_ref()) }
    }
}

impl From<&[AsciiChar]> for &AsciiStr {
    fn from(value: &[AsciiChar]) -> Self {
        unsafe { &*(value as *const [AsciiChar] as *const AsciiStr) }
    }
}

impl From<&mut [AsciiChar]> for &mut AsciiStr {
    fn from(value: &mut [AsciiChar]) -> Self {
        unsafe { &mut *(value as *mut [AsciiChar] as *mut AsciiStr) }
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

impl<'a> IntoIterator for &'a AsciiStr {
    type Item = AsciiChar;
    type IntoIter = iter::Cloned<slice::Iter<'a, AsciiChar>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter().cloned()
    }
}

impl Display for AsciiStr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let string: &str = self.as_ref();
        write!(f, "{string}")
    }
}

impl Debug for AsciiStr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let string: &str = self.as_ref();
        write!(f, "{string}")
    }
}

/// Owned ASCII string.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AsciiString(Vec<AsciiChar>);

impl AsciiString {
    /// Create a new empty string.
    pub fn new() -> Self {
        AsciiString(Vec::new())
    }

    /// Push a character onto the end of the string.
    pub fn push_char(&mut self, ch: AsciiChar) {
        self.0.push(ch);
    }

    /// Return an iterator over the ASCII characters in the string,
    /// consuming the string.
    pub fn into_chars(self) -> <Self as IntoIterator>::IntoIter {
        self.into_iter()
    }
}

impl From<Vec<AsciiChar>> for AsciiString {
    fn from(value: Vec<AsciiChar>) -> Self {
        AsciiString(value)
    }
}

impl<const N: usize> From<[AsciiChar; N]> for AsciiString {
    fn from(value: [AsciiChar; N]) -> Self {
        AsciiString(value.to_vec())
    }
}

impl FromIterator<AsciiChar> for AsciiString {
    fn from_iter<T: IntoIterator<Item = AsciiChar>>(iter: T) -> Self {
        AsciiString(iter.into_iter().collect())
    }
}

impl TryFrom<Vec<u8>> for AsciiString {
    type Error = u8;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        if value.is_ascii() {
            Ok(Self(unsafe {
                mem::transmute::<Vec<u8>, Vec<AsciiChar>>(value)
            }))
        } else {
            Err(value.into_iter().find(|b| !b.is_ascii()).unwrap())
        }
    }
}

impl<const N: usize> TryFrom<[u8; N]> for AsciiString {
    type Error = u8;

    fn try_from(value: [u8; N]) -> Result<Self, Self::Error> {
        AsciiString::try_from(value.to_vec())
    }
}

impl TryFrom<String> for AsciiString {
    type Error = char;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.is_ascii() {
            Ok(Self(unsafe {
                mem::transmute::<Vec<u8>, Vec<AsciiChar>>(value.into_bytes())
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

impl IntoIterator for AsciiString {
    type Item = AsciiChar;
    type IntoIter = vec::IntoIter<AsciiChar>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
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

    /// Create a slice of the ASCII string.
    pub fn slice_from(&self, start: usize) -> AsciiSlice {
        let start = (self.start + start).min(self.end);
        AsciiSlice {
            source: Rc::clone(&self.source),
            start,
            end: self.end,
        }
    }

    /// Create a slice of the ASCII string.
    pub fn slice_to(&self, end: usize) -> AsciiSlice {
        let end = (self.start + end).min(self.end);
        AsciiSlice {
            source: Rc::clone(&self.source),
            start: self.start,
            end,
        }
    }

    /// Return an iterator over the ASCII characters in the string slice.
    pub fn into_chars(self) -> <Self as IntoIterator>::IntoIter {
        self.into_iter()
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

impl IntoIterator for AsciiSlice {
    type IntoIter = AsciiSliceIntoIter;
    type Item = AsciiChar;

    fn into_iter(self) -> Self::IntoIter {
        AsciiSliceIntoIter {
            index: self.start,
            slice: self,
        }
    }
}

/// An iterator over the characters in an ASCII
/// string slice.
pub struct AsciiSliceIntoIter {
    slice: AsciiSlice,
    index: usize,
}

impl Iterator for AsciiSliceIntoIter {
    type Item = AsciiChar;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.slice.end {
            return None;
        }
        let char = self.slice[self.index];
        self.index += 1;
        Some(char)
    }
}

#[derive(Debug, Clone)]
/// A shared or borrowed reference to an ASCII string.
pub enum AsciiRef<'a> {
    /// The reference is a slice into some multiple-owned ASCII.
    Shared(AsciiSlice),
    /// The reference is a slice into some ASCII.
    Borrowed(&'a AsciiStr),
}

impl From<AsciiSlice> for AsciiRef<'static> {
    fn from(value: AsciiSlice) -> Self {
        Self::Shared(value)
    }
}

impl From<AsciiString> for AsciiRef<'static> {
    fn from(value: AsciiString) -> Self {
        Self::Shared(value.into())
    }
}

impl<'a> From<&'a AsciiStr> for AsciiRef<'a> {
    fn from(value: &'a AsciiStr) -> Self {
        Self::Borrowed(value)
    }
}

impl AsciiRef<'_> {
    /// Create a slice of the ASCII string.
    pub fn slice(&self, range: Range<usize>) -> Self {
        match self {
            Self::Shared(shared_slice) => Self::Shared(shared_slice.slice(range)),
            Self::Borrowed(string) => Self::Borrowed(&string[range]),
        }
    }

    /// Create a slice of the ASCII string.
    pub fn slice_from(&self, start: usize) -> Self {
        match self {
            Self::Shared(shared_slice) => Self::Shared(shared_slice.slice_from(start)),
            Self::Borrowed(string) => Self::Borrowed(&string[start..]),
        }
    }

    /// Create a slice of the ASCII string.
    pub fn slice_to(&self, end: usize) -> Self {
        match self {
            Self::Shared(shared_slice) => Self::Shared(shared_slice.slice_to(end)),
            Self::Borrowed(string) => Self::Borrowed(&string[..end]),
        }
    }

    /// Return an iterator over the ASCII characters in a string.
    pub fn into_chars(self) -> <Self as IntoIterator>::IntoIter {
        self.into_iter()
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

impl Eq for AsciiRef<'_> {}

impl<'a> IntoIterator for AsciiRef<'a> {
    type IntoIter = AsciiRefIntoIter<'a>;
    type Item = AsciiChar;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            Self::Shared(shared_slice) => AsciiRefIntoIter::Shared(shared_slice.into_iter()),
            Self::Borrowed(string) => AsciiRefIntoIter::Borrowed(string.into_iter()),
        }
    }
}

/// An iterator over the characters in an ASCII string.
pub enum AsciiRefIntoIter<'a> {
    /// An iterator over a shared string slice.
    Shared(<AsciiSlice as IntoIterator>::IntoIter),
    /// An iterator over a borrowed string slice.
    Borrowed(<&'a AsciiStr as IntoIterator>::IntoIter),
}

impl Iterator for AsciiRefIntoIter<'_> {
    type Item = AsciiChar;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Shared(iter) => iter.next(),
            Self::Borrowed(iter) => iter.next(),
        }
    }
}

/// An iterator over the ASCII characters in a source,
/// including the line and column of each character.
pub struct CharLocations<I> {
    chars: I,
    location: SourceLocation,
}

impl<I> CharLocations<I> {
    /// Set the current source location of the iterator.
    pub fn with_location(mut self, location: SourceLocation) -> Self {
        self.location = location;
        self
    }
}

impl<I: IntoIterator<Item = AsciiChar>> From<I> for CharLocations<I::IntoIter> {
    fn from(value: I) -> Self {
        CharLocations {
            chars: value.into_iter(),
            location: SourceLocation::ZERO,
        }
    }
}

impl<I: Iterator<Item = AsciiChar>> Iterator for CharLocations<I> {
    type Item = (SourceLocation, AsciiChar);

    fn next(&mut self) -> Option<Self::Item> {
        let char = self.chars.next()?;
        let location = self.location;
        self.location = self.location.after_char(char);
        Some((location, char))
    }
}

/// The source text used for segmentation.
enum SegmenterSource<'a> {
    /// Backed by an `AsciiRef` — supports zero-cost slicing by index.
    /// Covers `&AsciiStr`, `AsciiSlice`, and `AsciiRef` uniformly.
    Indexed {
        source: AsciiRef<'a>,
        start: usize,
        current: usize,
    },
    /// Backed by a raw character stream — characters are accumulated
    /// into an `AsciiString` and wrapped into an `AsciiRef::Shared` on cut.
    Streaming { accumulated: AsciiString },
}

/// An iterator adapter that walks a character stream and allows the caller
/// to cut segments out of it as [`AsciiRef`]s with their source locations.
pub struct Segmenter<'a, I: Iterator<Item = AsciiChar>> {
    source: SegmenterSource<'a>,
    chars: Peekable<CharLocations<I>>,
    segment_start: SourceLocation,
    next_location: SourceLocation,
}

impl<'a> Segmenter<'a, <AsciiRef<'a> as IntoIterator>::IntoIter> {
    /// Segment a shared or borrowed string slice.
    pub fn segment_str<T: Into<AsciiRef<'a>>>(source: T) -> Self {
        let source = source.into();
        Segmenter {
            source: SegmenterSource::Indexed {
                source: source.clone(),
                start: 0,
                current: 0,
            },
            chars: CharLocations::from(source.into_iter()).peekable(),
            next_location: SourceLocation::ZERO,
            segment_start: SourceLocation::ZERO,
        }
    }
}

impl<I: Iterator<Item = AsciiChar>> Segmenter<'static, I> {
    /// Segment a consumed stream of characters.
    pub fn segment_chars<T: IntoIterator<IntoIter = I>>(source: T) -> Self {
        let source = source.into_iter();
        Segmenter {
            source: SegmenterSource::Streaming {
                accumulated: AsciiString::new(),
            },
            chars: CharLocations::from(source).peekable(),
            next_location: SourceLocation::ZERO,
            segment_start: SourceLocation::ZERO,
        }
    }
}

impl<'a, I: Iterator<Item = AsciiChar>> Iterator for Segmenter<'a, I> {
    type Item = (SourceLocation, AsciiChar);

    fn next(&mut self) -> Option<Self::Item> {
        let next @ (location, ch) = self.chars.next()?;
        self.next_location = location.after_char(ch);
        match self.source {
            SegmenterSource::Indexed {
                ref mut current, ..
            } => {
                *current += 1;
            }
            SegmenterSource::Streaming {
                ref mut accumulated,
            } => {
                accumulated.0.push(ch);
            }
        }
        Some(next)
    }
}

impl<'a, I: Iterator<Item = AsciiChar>> Segmenter<'a, I> {
    /// Peek at the next character.
    pub fn peek(&mut self) -> Option<(SourceLocation, AsciiChar)> {
        self.chars.peek().cloned()
    }

    /// Cut the current segment, returning the accumulated text as an
    /// [`AsciiRef`] and the [`SourceLocation`] of the segment's start.
    /// Resets the segment start to the next character's location.
    pub fn cut(&mut self) -> (AsciiRef<'a>, SourceLocation) {
        let segment_location = self.segment_start;
        let segment_text = match self.source {
            SegmenterSource::Indexed {
                ref source,
                ref mut start,
                current,
            } => {
                let segment = source.slice(*start..current);
                *start = current;
                segment
            }
            SegmenterSource::Streaming {
                ref mut accumulated,
            } => {
                let segment = mem::replace(accumulated, AsciiString::new());
                AsciiRef::Shared(AsciiSlice::from(segment))
            }
        };
        // Start of the next segment.
        self.segment_start = self.next_location;
        (segment_text, segment_location)
    }

    /// Return the unconsumed characters, including
    /// all characters that were not made into a segment.
    pub fn rest(self) -> AsciiRef<'a> {
        match self.source {
            SegmenterSource::Indexed { source, start, .. } => source.slice_from(start),
            SegmenterSource::Streaming { mut accumulated } => {
                for (_, ch) in self.chars {
                    accumulated.push_char(ch);
                }
                accumulated.into()
            }
        }
    }
}

pub(crate) struct FallibleAsciiChars<T, I: Iterator<Item = T>> {
    bytes: I,
    invalid: Rc<RefCell<VecDeque<T>>>,
}

impl<T, I: Iterator<Item = T>> FallibleAsciiChars<T, I> {
    pub fn new<J: IntoIterator<IntoIter = I>>(bytes: J) -> (Self, Rc<RefCell<VecDeque<T>>>) {
        let invalid = Rc::new(RefCell::new(VecDeque::with_capacity(1)));
        let chars = FallibleAsciiChars {
            bytes: bytes.into_iter(),
            invalid: Rc::clone(&invalid),
        };
        (chars, invalid)
    }
}

impl<I> Iterator for FallibleAsciiChars<u8, I>
where
    I: Iterator<Item = u8>,
{
    type Item = AsciiChar;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let byte = self.bytes.next()?;
            if let Ok(ch) = AsciiChar::try_from(byte) {
                return Some(ch);
            };
            let mut invalid = RefCell::borrow_mut(&self.invalid);
            invalid.push_back(byte);
        }
    }
}

impl<I> Iterator for FallibleAsciiChars<io::Result<u8>, I>
where
    I: Iterator<Item = io::Result<u8>>,
{
    type Item = AsciiChar;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let result = self.bytes.next()?;
            let Ok(byte) = result else {
                let mut invalid = RefCell::borrow_mut(&self.invalid);
                invalid.push_back(result);
                continue;
            };
            if let Ok(char) = AsciiChar::try_from(byte) {
                return Some(char);
            };
            let mut invalid = RefCell::borrow_mut(&self.invalid);
            invalid.push_back(result);
        }
    }
}
