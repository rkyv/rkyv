//! Archived versions of FFI types.

use core::{
    borrow::Borrow,
    cmp,
    ffi::CStr,
    fmt, hash,
    ops::{Deref, Index, RangeFull},
};

use munge::munge;
use rancor::Fallible;

use crate::{
    primitive::FixedUsize, ser::Writer, ArchiveUnsized, Place, Portable,
    RelPtr, SerializeUnsized,
};

/// An archived [`CString`](crate::alloc::ffi::CString).
///
/// Uses a [`RelPtr`] to a `CStr` under the hood.
#[derive(Portable)]
#[rkyv(crate)]
#[cfg_attr(
    feature = "bytecheck",
    derive(bytecheck::CheckBytes),
    bytecheck(verify)
)]
#[repr(transparent)]
pub struct ArchivedCString {
    ptr: RelPtr<CStr>,
}

impl ArchivedCString {
    /// Returns the contents of this CString as a slice of bytes.
    ///
    /// The returned slice does **not** contain the trailing nul terminator, and
    /// it is guaranteed to not have any interior nul bytes. If you need the
    /// nul terminator, use
    /// [`as_bytes_with_nul`][ArchivedCString::as_bytes_with_nul()] instead.
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        self.as_c_str().to_bytes()
    }

    /// Equivalent to [`as_bytes`][ArchivedCString::as_bytes()] except that the
    /// returned slice includes the trailing nul terminator.
    #[inline]
    pub fn as_bytes_with_nul(&self) -> &[u8] {
        self.as_c_str().to_bytes_with_nul()
    }

    /// Extracts a `CStr` slice containing the entire string.
    #[inline]
    pub fn as_c_str(&self) -> &CStr {
        unsafe { &*self.ptr.as_ptr() }
    }

    /// Resolves an archived C string from the given C string and parameters.
    #[inline]
    pub fn resolve_from_c_str(
        c_str: &CStr,
        resolver: CStringResolver,
        out: Place<Self>,
    ) {
        munge!(let ArchivedCString { ptr } = out);
        RelPtr::emplace_unsized(
            resolver.pos as usize,
            c_str.archived_metadata(),
            ptr,
        );
    }

    /// Serializes a C string.
    pub fn serialize_from_c_str<S: Fallible + Writer + ?Sized>(
        c_str: &CStr,
        serializer: &mut S,
    ) -> Result<CStringResolver, S::Error> {
        Ok(CStringResolver {
            pos: c_str.serialize_unsized(serializer)? as FixedUsize,
        })
    }
}

impl AsRef<CStr> for ArchivedCString {
    fn as_ref(&self) -> &CStr {
        self.as_c_str()
    }
}

impl Borrow<CStr> for ArchivedCString {
    #[inline]
    fn borrow(&self) -> &CStr {
        self.as_c_str()
    }
}

impl fmt::Debug for ArchivedCString {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_c_str().fmt(f)
    }
}

impl Deref for ArchivedCString {
    type Target = CStr;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_c_str()
    }
}

impl Eq for ArchivedCString {}

impl hash::Hash for ArchivedCString {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.as_bytes_with_nul().hash(state);
    }
}

impl Index<RangeFull> for ArchivedCString {
    type Output = CStr;

    #[inline]
    fn index(&self, _: RangeFull) -> &Self::Output {
        self.as_c_str()
    }
}

impl Ord for ArchivedCString {
    #[inline]
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.as_bytes().cmp(other.as_bytes())
    }
}

impl PartialEq for ArchivedCString {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.as_bytes() == other.as_bytes()
    }
}

impl PartialEq<&CStr> for ArchivedCString {
    #[inline]
    fn eq(&self, other: &&CStr) -> bool {
        PartialEq::eq(self.as_c_str(), other)
    }
}

impl PartialEq<ArchivedCString> for &CStr {
    #[inline]
    fn eq(&self, other: &ArchivedCString) -> bool {
        PartialEq::eq(other.as_c_str(), self)
    }
}

impl PartialOrd for ArchivedCString {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// The resolver for `CString`.
pub struct CStringResolver {
    pos: FixedUsize,
}

#[cfg(feature = "bytecheck")]
mod verify {
    use core::ffi::CStr;

    use bytecheck::{
        rancor::{Fallible, Source},
        CheckBytes, Verify,
    };

    use crate::{
        ffi::ArchivedCString,
        validation::{ArchiveContext, ArchiveContextExt},
    };

    unsafe impl<C> Verify<C> for ArchivedCString
    where
        C: Fallible + ArchiveContext + ?Sized,
        C::Error: Source,
    {
        fn verify(&self, context: &mut C) -> Result<(), C::Error> {
            let ptr = self.ptr.as_ptr_wrapping();
            context.in_subtree(ptr, |context| unsafe {
                CStr::check_bytes(ptr, context)
            })
        }
    }
}
