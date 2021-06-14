//! Archived versions of `String` and `CString`.

use crate::{
    ArchiveUnsized,
    Fallible,
    MetadataResolver,
    RelPtr,
    SerializeUnsized,
};
use core::{
    borrow::Borrow,
    cmp,
    fmt,
    hash,
    mem::MaybeUninit,
    ops::{Deref, Index, Range, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive},
    pin::Pin,
};

/// An archived [`String`].
///
/// Uses a [`RelPtr`] to a `str` under the hood.
#[derive(Debug)]
#[repr(transparent)]
pub struct ArchivedString(RelPtr<str>);

impl ArchivedString {
    /// Extracts a string slice containing the entire `ArchivedString`.
    #[inline]
    pub fn as_str(&self) -> &str {
        unsafe { &*self.0.as_ptr() }
    }

    /// Extracts a pinned mutable string slice containing the entire `ArchivedString`.
    #[inline]
    pub fn as_pin_mut_str(self: Pin<&mut Self>) -> Pin<&mut str> {
        unsafe { self.map_unchecked_mut(|s| &mut *s.0.as_mut_ptr()) }
    }

    /// Resolves the archived string from a given `str`.
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
        out: &mut MaybeUninit<Self>,
    ) {
        let (fp, fo) = out_field!(out.0);
        #[allow(clippy::unit_arg)]
        value.resolve_unsized(pos + fp, resolver.pos, resolver.metadata_resolver, fo);
    }

    /// Serializes the archived string from a given `str`.
    #[inline]
    pub fn serialize_from_str<S: Fallible + ?Sized>(
        value: &str,
        serializer: &mut S,
    ) -> Result<StringResolver, S::Error>
    where
        str: SerializeUnsized<S>,
    {
        Ok(StringResolver {
            pos: value.serialize_unsized(serializer)?,
            metadata_resolver: value.serialize_metadata(serializer)?,
        })
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
    }
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

impl PartialEq<ArchivedString> for &str {
    #[inline]
    fn eq(&self, other: &ArchivedString) -> bool {
        PartialEq::eq(other.as_str(), *self)
    }
}

impl PartialEq<String> for ArchivedString {
    #[inline]
    fn eq(&self, other: &String) -> bool {
        PartialEq::eq(self.as_str(), other.as_str())
    }
}

impl PartialEq<ArchivedString> for String {
    #[inline]
    fn eq(&self, other: &ArchivedString) -> bool {
        PartialEq::eq(other.as_str(), self.as_str())
    }
}

/// The resolver for `String`.
pub struct StringResolver {
    pos: usize,
    metadata_resolver: MetadataResolver<str>,
}

#[cfg(feature = "validation")]
const _: () = {
    use crate::validation::{
        owned::{CheckOwnedPointerError, OwnedPointerError},
        ArchiveBoundsContext,
        ArchiveMemoryContext,
    };
    use bytecheck::CheckBytes;
    #[cfg(feature = "std")]
    use std::error::Error;

    impl<C: ArchiveBoundsContext + ArchiveMemoryContext + ?Sized> CheckBytes<C> for ArchivedString
    where
        C::Error: Error,
    {
        type Error = CheckOwnedPointerError<str, C>;

        unsafe fn check_bytes<'a>(
            value: *const Self,
            context: &mut C,
        ) -> Result<&'a Self, Self::Error> {
            let rel_ptr = RelPtr::<str>::manual_check_bytes(value.cast(), context)
                .map_err(OwnedPointerError::PointerCheckBytesError)?;
            let ptr = context
                .claim_owned_rel_ptr(rel_ptr)
                .map_err(OwnedPointerError::ContextError)?;
            <str as CheckBytes<C>>::check_bytes(ptr, context)
                .map_err(OwnedPointerError::ValueCheckBytesError)?;
            Ok(&*value)
        }
    }
};
