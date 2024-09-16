//! Archived versions of string types.

pub mod repr;

use core::{
    borrow::Borrow,
    cmp,
    error::Error,
    fmt, hash,
    ops::{
        Deref, Index, Range, RangeFrom, RangeFull, RangeInclusive, RangeTo,
        RangeToInclusive,
    },
    str,
};

use munge::munge;
use rancor::{fail, Fallible, Source};
use repr::{ArchivedStringRepr, INLINE_CAPACITY};

use crate::{
    primitive::FixedUsize, seal::Seal, Place, Portable, SerializeUnsized,
};

/// An archived [`String`].
///
/// This has inline and out-of-line representations. Short strings will use the
/// available space inside the structure to store the string, and long strings
/// will store a [`RelPtr`](crate::RelPtr) to a `str` instead.
#[repr(transparent)]
#[cfg_attr(
    feature = "bytecheck",
    derive(bytecheck::CheckBytes),
    bytecheck(verify)
)]
#[derive(Portable)]
#[rkyv(crate)]
pub struct ArchivedString {
    repr: ArchivedStringRepr,
}

impl ArchivedString {
    /// Extracts a string slice containing the entire `ArchivedString`.
    #[inline]
    pub fn as_str(&self) -> &str {
        self.repr.as_str()
    }

    /// Extracts a sealed mutable string slice containing the entire
    /// `ArchivedString`.
    #[inline]
    pub fn as_str_seal(this: Seal<'_, Self>) -> Seal<'_, str> {
        munge!(let Self { repr } = this);
        ArchivedStringRepr::as_str_seal(repr)
    }

    /// Resolves an archived string from a given `str`.
    #[inline]
    pub fn resolve_from_str(
        value: &str,
        resolver: StringResolver,
        out: Place<Self>,
    ) {
        munge!(let ArchivedString { repr } = out);
        if value.len() <= repr::INLINE_CAPACITY {
            unsafe {
                ArchivedStringRepr::emplace_inline(value, repr.ptr());
            }
        } else {
            unsafe {
                ArchivedStringRepr::emplace_out_of_line(
                    value,
                    resolver.pos as usize,
                    repr,
                );
            }
        }
    }

    /// Serializes an archived string from a given `str`.
    pub fn serialize_from_str<S: Fallible + ?Sized>(
        value: &str,
        serializer: &mut S,
    ) -> Result<StringResolver, S::Error>
    where
        S::Error: Source,
        str: SerializeUnsized<S>,
    {
        if value.len() <= INLINE_CAPACITY {
            Ok(StringResolver { pos: 0 })
        } else if value.len() > repr::OUT_OF_LINE_CAPACITY {
            #[derive(Debug)]
            struct StringTooLongError;

            impl fmt::Display for StringTooLongError {
                fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    write!(
                        f,
                        "String was too long for the archived representation",
                    )
                }
            }

            impl Error for StringTooLongError {}

            fail!(StringTooLongError);
        } else {
            Ok(StringResolver {
                pos: value.serialize_unsized(serializer)? as FixedUsize,
            })
        }
    }
}

impl AsRef<str> for ArchivedString {
    #[inline]
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
        Some(self.cmp(other))
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

impl PartialEq<ArchivedString> for str {
    #[inline]
    fn eq(&self, other: &ArchivedString) -> bool {
        PartialEq::eq(other.as_str(), self)
    }
}

impl PartialOrd<&str> for ArchivedString {
    #[inline]
    fn partial_cmp(&self, other: &&str) -> Option<cmp::Ordering> {
        self.as_str().partial_cmp(*other)
    }
}

impl PartialOrd<str> for ArchivedString {
    #[inline]
    fn partial_cmp(&self, other: &str) -> Option<cmp::Ordering> {
        self.as_str().partial_cmp(other)
    }
}

impl PartialOrd<ArchivedString> for &str {
    #[inline]
    fn partial_cmp(&self, other: &ArchivedString) -> Option<cmp::Ordering> {
        self.partial_cmp(&other.as_str())
    }
}

impl PartialOrd<ArchivedString> for str {
    #[inline]
    fn partial_cmp(&self, other: &ArchivedString) -> Option<cmp::Ordering> {
        self.partial_cmp(other.as_str())
    }
}

/// The resolver for `String`.
pub struct StringResolver {
    pos: FixedUsize,
}

#[cfg(feature = "bytecheck")]
mod verify {
    use bytecheck::{
        rancor::{Fallible, Source},
        CheckBytes, Verify,
    };

    use crate::{
        string::{repr::ArchivedStringRepr, ArchivedString},
        validation::{ArchiveContext, ArchiveContextExt},
    };

    unsafe impl<C> Verify<C> for ArchivedString
    where
        C: Fallible + ArchiveContext + ?Sized,
        C::Error: Source,
    {
        fn verify(&self, context: &mut C) -> Result<(), C::Error> {
            if self.repr.is_inline() {
                unsafe {
                    str::check_bytes(self.repr.as_str_ptr(), context)?;
                }
            } else {
                let base =
                    (&self.repr as *const ArchivedStringRepr).cast::<u8>();
                let offset = unsafe { self.repr.out_of_line_offset() };
                let metadata = self.repr.len();

                let address = base.wrapping_offset(offset).cast::<()>();
                let ptr = ptr_meta::from_raw_parts(address, metadata);

                context.in_subtree(ptr, |context| {
                    // SAFETY: `in_subtree` has guaranteed that `ptr` is
                    // properly aligned and points to enough bytes to represent
                    // the pointed-to `str`.
                    unsafe { str::check_bytes(ptr, context) }
                })?;
            }

            Ok(())
        }
    }
}
