//! Archived versions of string types.

use crate::{Archive, Archived, Fallible, FixedIsize, SerializeUnsized};
use core::{
    borrow::Borrow,
    cmp, fmt, hash, mem,
    ops::{Deref, Index, Range, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive},
    pin::Pin,
    ptr, slice, str,
};

const OFFSET_BYTES: usize = mem::size_of::<FixedIsize>();

#[derive(Clone, Copy)]
#[repr(C)]
struct OutOfLineRepr {
    len: Archived<usize>,
    // offset is always stored in little-endian format to put the sign bit at the end
    // this representation is optimized for little-endian architectures
    offset: [u8; OFFSET_BYTES],
}

const INLINE_CAPACITY: usize = mem::size_of::<OutOfLineRepr>() - 1;

#[derive(Clone, Copy)]
#[repr(C)]
struct InlineRepr {
    bytes: [u8; INLINE_CAPACITY],
    len: u8,
}

union ArchivedStringRepr {
    out_of_line: OutOfLineRepr,
    inline: InlineRepr,
}

impl ArchivedStringRepr {
    #[inline]
    fn is_inline(&self) -> bool {
        unsafe { self.inline.len & 0x80 == 0 }
    }

    #[inline]
    unsafe fn out_of_line_offset(&self) -> isize {
        FixedIsize::from_le_bytes(self.out_of_line.offset) as isize
    }

    #[inline]
    fn as_ptr(&self) -> *const u8 {
        unsafe {
            if self.is_inline() {
                self.inline.bytes.as_ptr()
            } else {
                (self as *const Self)
                    .cast::<u8>()
                    .offset(self.out_of_line_offset())
            }
        }
    }

    #[inline]
    fn as_mut_ptr(&mut self) -> *mut u8 {
        unsafe {
            if self.is_inline() {
                self.inline.bytes.as_mut_ptr()
            } else {
                (self as *mut Self)
                    .cast::<u8>()
                    .offset(self.out_of_line_offset())
            }
        }
    }

    #[inline]
    fn len(&self) -> usize {
        unsafe {
            if self.is_inline() {
                self.inline.len as usize
            } else {
                from_archived!(self.out_of_line.len) as usize
            }
        }
    }

    #[cfg(feature = "validation")]
    #[inline]
    fn as_str_ptr(&self) -> *const str {
        ptr_meta::from_raw_parts(self.as_ptr().cast(), self.len())
    }

    #[inline]
    fn bytes(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.as_ptr(), self.len()) }
    }

    #[inline]
    fn bytes_mut(&mut self) -> &mut [u8] {
        unsafe { slice::from_raw_parts_mut(self.as_mut_ptr(), self.len()) }
    }

    #[inline]
    fn as_str(&self) -> &str {
        unsafe { str::from_utf8_unchecked(self.bytes()) }
    }

    #[inline]
    fn as_mut_str(&mut self) -> &mut str {
        unsafe { str::from_utf8_unchecked_mut(self.bytes_mut()) }
    }

    #[inline]
    unsafe fn emplace_inline(value: &str, out: *mut Self) {
        let out_bytes = ptr::addr_of_mut!((*out).inline.bytes);
        ptr::copy_nonoverlapping(value.as_bytes().as_ptr(), out_bytes.cast(), value.len());

        let out_len = ptr::addr_of_mut!((*out).inline.len);
        *out_len = value.len() as u8;
    }

    #[inline]
    unsafe fn emplace_out_of_line(value: &str, pos: usize, target: usize, out: *mut Self) {
        let out_len = ptr::addr_of_mut!((*out).out_of_line.len);
        usize::resolve(&value.len(), pos, (), out_len);

        let out_offset = ptr::addr_of_mut!((*out).out_of_line.offset);
        let offset = crate::rel_ptr::signed_offset(pos, target).unwrap();
        *out_offset = (offset as FixedIsize).to_le_bytes();
    }
}

/// An archived [`String`].
///
/// This has inline and out-of-line representations. Short strings will use the available space
/// inside the structure to store the string, and long strings will store a
/// [`RelPtr`](crate::RelPtr) to a `str` instead.
#[repr(transparent)]
pub struct ArchivedString(ArchivedStringRepr);

impl ArchivedString {
    /// Extracts a string slice containing the entire `ArchivedString`.
    #[inline]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// Extracts a pinned mutable string slice containing the entire `ArchivedString`.
    #[inline]
    pub fn pin_mut_str(self: Pin<&mut Self>) -> Pin<&mut str> {
        unsafe { self.map_unchecked_mut(|s| s.0.as_mut_str()) }
    }

    /// Resolves an archived string from a given `str`.
    ///
    /// # Safety
    ///
    /// - `pos` must be the position of `out` within the archive
    /// - `resolver` must be the result of serializing `value`
    #[inline]
    pub unsafe fn resolve_from_str(
        value: &str,
        pos: usize,
        resolver: StringResolver,
        out: *mut Self,
    ) {
        if value.len() <= INLINE_CAPACITY {
            ArchivedStringRepr::emplace_inline(value, out.cast());
        } else {
            ArchivedStringRepr::emplace_out_of_line(value, pos, resolver.pos, out.cast());
        }
    }

    /// Serializes an archived string from a given `str`.
    #[inline]
    pub fn serialize_from_str<S: Fallible + ?Sized>(
        value: &str,
        serializer: &mut S,
    ) -> Result<StringResolver, S::Error>
    where
        str: SerializeUnsized<S>,
    {
        if value.len() <= INLINE_CAPACITY {
            Ok(StringResolver { pos: 0 })
        } else {
            Ok(StringResolver {
                pos: value.serialize_unsized(serializer)?,
            })
        }
    }
}

impl AsRef<str> for ArchivedString {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Borrow<str> for ArchivedString {
    #[inline]
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Debug for ArchivedString {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.as_str(), f)
    }
}

impl Deref for ArchivedString {
    type Target = str;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl fmt::Display for ArchivedString {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.as_str(), f)
    }
}

impl Eq for ArchivedString {}

impl hash::Hash for ArchivedString {
    #[inline]
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.as_str().hash(state)
    }
}

macro_rules! impl_index {
    ($index:ty) => {
        impl Index<$index> for ArchivedString {
            type Output = str;

            #[inline]
            fn index(&self, index: $index) -> &Self::Output {
                self.as_str().index(index)
            }
        }
    };
}

impl_index!(Range<usize>);
impl_index!(RangeFrom<usize>);
impl_index!(RangeFull);
impl_index!(RangeInclusive<usize>);
impl_index!(RangeTo<usize>);
impl_index!(RangeToInclusive<usize>);

impl Ord for ArchivedString {
    #[inline]
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.as_str().cmp(other.as_str())
    }
}

impl PartialEq for ArchivedString {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}

impl PartialOrd for ArchivedString {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        self.as_str().partial_cmp(other.as_str())
    }
}

impl PartialEq<&str> for ArchivedString {
    #[inline]
    fn eq(&self, other: &&str) -> bool {
        PartialEq::eq(self.as_str(), *other)
    }
}

impl PartialEq<str> for ArchivedString {
    #[inline]
    fn eq(&self, other: &str) -> bool {
        PartialEq::eq(self.as_str(), other)
    }
}

impl PartialEq<ArchivedString> for &str {
    #[inline]
    fn eq(&self, other: &ArchivedString) -> bool {
        PartialEq::eq(other.as_str(), *self)
    }
}

/// The resolver for `String`.
pub struct StringResolver {
    pos: usize,
}

#[cfg(feature = "validation")]
const _: () = {
    use crate::validation::{
        owned::{CheckOwnedPointerError, OwnedPointerError},
        ArchiveContext,
    };
    use bytecheck::{CheckBytes, Error};

    impl<C: ArchiveContext + ?Sized> CheckBytes<C> for ArchivedString
    where
        C::Error: Error,
    {
        type Error = CheckOwnedPointerError<str, C>;

        #[inline]
        unsafe fn check_bytes<'a>(
            value: *const Self,
            context: &mut C,
        ) -> Result<&'a Self, Self::Error> {
            // The repr is always valid
            let repr = &*value.cast::<ArchivedStringRepr>();

            if repr.is_inline() {
                str::check_bytes(repr.as_str_ptr(), context)
                    .map_err(OwnedPointerError::ValueCheckBytesError)?;
            } else {
                let base = value.cast();
                let offset = repr.out_of_line_offset();
                let metadata = repr.len();

                let ptr = context
                    .check_subtree_ptr::<str>(base, offset, metadata)
                    .map_err(OwnedPointerError::ContextError)?;

                let range = context
                    .push_prefix_subtree(ptr)
                    .map_err(OwnedPointerError::ContextError)?;
                str::check_bytes(repr.as_str_ptr(), context)
                    .map_err(OwnedPointerError::ValueCheckBytesError)?;
                context
                    .pop_prefix_range(range)
                    .map_err(OwnedPointerError::ContextError)?;
            }

            Ok(&*value)
        }
    }
};
