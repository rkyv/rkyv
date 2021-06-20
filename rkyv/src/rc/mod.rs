//! Archived versions of shared pointers.

#[cfg(feature = "validation")]
pub mod validation;

use crate::{
    ser::SharedSerializer, ArchivePointee, ArchiveUnsized, MetadataResolver, RelPtr,
    SerializeUnsized,
};
use core::{borrow::Borrow, cmp, fmt, hash, mem::MaybeUninit, ops::Deref, pin::Pin, ptr};

/// An archived `Rc`.
///
/// This is a thin wrapper around a [`RelPtr`] to the archived type.
#[repr(transparent)]
pub struct ArchivedRc<T: ArchivePointee + ?Sized>(RelPtr<T>);

impl<T: ArchivePointee + ?Sized> ArchivedRc<T> {
    /// Gets the value of the `ArchivedRc`.
    #[inline]
    pub fn get(&self) -> &T {
        unsafe { &*self.0.as_ptr() }
    }

    /// Gets the pinned mutable value of this `ArchivedRc`.
    ///
    /// # Safety
    ///
    /// Any other `ArchivedRc` pointers to the same value must not be dereferenced for the duration
    /// of the returned borrow.
    #[inline]
    pub unsafe fn get_pin_mut_unchecked(self: Pin<&mut Self>) -> Pin<&mut T> {
        self.map_unchecked_mut(|s| &mut *s.0.as_mut_ptr())
    }

    /// Resolves an archived `Rc` from a given reference.
    ///
    /// # Safety
    ///
    /// - `pos` must be the position of `out` within the archive
    /// - `resolver` must be the result of serializing `value`
    #[inline]
    pub unsafe fn resolve_from_ref<U: ArchiveUnsized<Archived = T> + ?Sized>(
        value: &U,
        pos: usize,
        resolver: RcResolver<MetadataResolver<U>>,
        out: &mut MaybeUninit<Self>,
    ) {
        let (fp, fo) = out_field!(out.0);
        value.resolve_unsized(pos + fp, resolver.pos, resolver.metadata_resolver, fo);
    }

    /// Serializes an archived `Rc` from a given reference.
    #[inline]
    pub fn serialize_from_ref<U: SerializeUnsized<S> + ?Sized, S: SharedSerializer + ?Sized>(
        value: &U,
        serializer: &mut S,
    ) -> Result<RcResolver<MetadataResolver<U>>, S::Error> {
        Ok(RcResolver {
            pos: serializer.serialize_shared(value)?,
            metadata_resolver: value.serialize_metadata(serializer)?,
        })
    }
}

impl<T: ArchivePointee + ?Sized> AsRef<T> for ArchivedRc<T> {
    #[inline]
    fn as_ref(&self) -> &T {
        self.get()
    }
}

impl<T: ArchivePointee + ?Sized> Borrow<T> for ArchivedRc<T> {
    #[inline]
    fn borrow(&self) -> &T {
        self.get()
    }
}

impl<T: ArchivePointee + fmt::Debug + ?Sized> fmt::Debug for ArchivedRc<T> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.get().fmt(f)
    }
}

impl<T: ArchivePointee + ?Sized> Deref for ArchivedRc<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.get()
    }
}

impl<T: ArchivePointee + fmt::Display + ?Sized> fmt::Display for ArchivedRc<T> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.get().fmt(f)
    }
}

impl<T: ArchivePointee + Eq + ?Sized> Eq for ArchivedRc<T> {}

impl<T: ArchivePointee + hash::Hash + ?Sized> hash::Hash for ArchivedRc<T> {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.get().hash(state)
    }
}

impl<T: ArchivePointee + Ord + ?Sized> Ord for ArchivedRc<T> {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.get().cmp(other.get())
    }
}

impl<T, U> PartialEq<ArchivedRc<U>> for ArchivedRc<T>
where
    T: ArchivePointee + PartialEq<U> + ?Sized,
    U: ArchivePointee + ?Sized,
{
    fn eq(&self, other: &ArchivedRc<U>) -> bool {
        self.get().eq(other.get())
    }
}

impl<T, U> PartialOrd<ArchivedRc<U>> for ArchivedRc<T>
where
    T: ArchivePointee + PartialOrd<U> + ?Sized,
    U: ArchivePointee + ?Sized,
{
    fn partial_cmp(&self, other: &ArchivedRc<U>) -> Option<cmp::Ordering> {
        self.get().partial_cmp(other.get())
    }
}

impl<T> fmt::Pointer for ArchivedRc<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Pointer::fmt(&self.0.base(), f)
    }
}

/// The resolver for `Rc`.
pub struct RcResolver<T> {
    pos: usize,
    metadata_resolver: T,
}

/// An archived `rc::Weak`.
#[repr(u8)]
pub enum ArchivedRcWeak<T: ArchivePointee + ?Sized> {
    /// A null weak pointer
    None,
    /// A weak pointer to some shared pointer
    Some(ArchivedRc<T>),
}

impl<T: ArchivePointee + ?Sized> ArchivedRcWeak<T> {
    /// Attempts to upgrade the weak pointer to an `ArchivedArc`.
    ///
    /// Returns `None` if a null weak pointer was serialized.
    #[inline]
    pub fn upgrade(&self) -> Option<&ArchivedRc<T>> {
        match self {
            ArchivedRcWeak::None => None,
            ArchivedRcWeak::Some(r) => Some(r),
        }
    }

    /// Attempts to upgrade a pinned mutable weak pointer.
    #[inline]
    pub fn upgrade_pin_mut(self: Pin<&mut Self>) -> Option<Pin<&mut ArchivedRc<T>>> {
        unsafe {
            match self.get_unchecked_mut() {
                ArchivedRcWeak::None => None,
                ArchivedRcWeak::Some(r) => Some(Pin::new_unchecked(r)),
            }
        }
    }

    /// Resolves an archived `Weak` from a given optional reference.
    ///
    /// # Safety
    ///
    /// - `pos` must be the position of `out` within the archive
    /// - `resolver` must be the result of serializing `value`
    #[inline]
    pub unsafe fn resolve_from_ref<U: ArchiveUnsized<Archived = T> + ?Sized>(
        value: Option<&U>,
        pos: usize,
        resolver: RcWeakResolver<MetadataResolver<U>>,
        out: &mut MaybeUninit<Self>,
    ) {
        match resolver {
            RcWeakResolver::None => {
                let out = &mut *out
                    .as_mut_ptr()
                    .cast::<MaybeUninit<ArchivedRcWeakVariantNone>>();
                ptr::addr_of_mut!((*out.as_mut_ptr()).0).write(ArchivedRcWeakTag::None);
            }
            RcWeakResolver::Some(resolver) => {
                let out = &mut *out
                    .as_mut_ptr()
                    .cast::<MaybeUninit<ArchivedRcWeakVariantSome<T>>>();
                ptr::addr_of_mut!((*out.as_mut_ptr()).0).write(ArchivedRcWeakTag::Some);

                let (fp, fo) = out_field!(out.1);
                ArchivedRc::resolve_from_ref(value.unwrap(), pos + fp, resolver, fo);
            }
        }
    }

    /// Serializes an archived `Weak` from a given optional reference.
    #[inline]
    pub fn serialize_from_ref<U, S>(
        value: Option<&U>,
        serializer: &mut S,
    ) -> Result<RcWeakResolver<MetadataResolver<U>>, S::Error>
    where
        U: SerializeUnsized<S, Archived = T> + ?Sized,
        S: SharedSerializer + ?Sized,
    {
        Ok(match value {
            None => RcWeakResolver::None,
            Some(r) => RcWeakResolver::Some(ArchivedRc::<T>::serialize_from_ref(r, serializer)?),
        })
    }
}

/// The resolver for `rc::Weak`.
pub enum RcWeakResolver<T> {
    /// The weak pointer was null
    None,
    /// The weak pointer was to some shared pointer
    Some(RcResolver<T>),
}

#[allow(dead_code)]
#[repr(u8)]
enum ArchivedRcWeakTag {
    None,
    Some,
}

#[repr(C)]
struct ArchivedRcWeakVariantNone(ArchivedRcWeakTag);

#[repr(C)]
struct ArchivedRcWeakVariantSome<T: ArchivePointee + ?Sized>(ArchivedRcWeakTag, ArchivedRc<T>);
