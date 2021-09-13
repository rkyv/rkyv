//! An archived version of `Box`.

use crate::{ArchivePointee, ArchiveUnsized, Fallible, RelPtr, SerializeUnsized};
use core::{borrow::Borrow, cmp, fmt, hash, ops::Deref, pin::Pin};

/// An archived [`Box`].
///
/// This is a thin wrapper around a [`RelPtr`] to the archived type.
#[repr(transparent)]
pub struct ArchivedBox<T: ArchivePointee + ?Sized>(RelPtr<T>);

impl<T: ArchivePointee + ?Sized> ArchivedBox<T> {
    /// Returns a reference to the value of this archived box.
    #[inline]
    pub fn get(&self) -> &T {
        unsafe { &*self.0.as_ptr() }
    }

    /// Returns a pinned mutable reference to the value of this archived box
    #[inline]
    pub fn get_pin_mut(self: Pin<&mut Self>) -> Pin<&mut T> {
        unsafe { self.map_unchecked_mut(|s| &mut *s.0.as_mut_ptr()) }
    }

    /// Resolves an archived box from the given value and parameters.
    ///
    /// # Safety
    ///
    /// - `pos` must be the position of `out` within the archive
    /// - `resolver` must be the result of serializing `value`
    #[inline]
    pub unsafe fn resolve_from_ref<U: ArchiveUnsized<Archived = T> + ?Sized>(
        value: &U,
        pos: usize,
        resolver: BoxResolver<U::MetadataResolver>,
        out: *mut Self,
    ) {
        let (fp, fo) = out_field!(out.0);
        value.resolve_unsized(pos + fp, resolver.pos, resolver.metadata_resolver, fo);
    }

    /// Serializes an archived box from the given value and serializer.
    #[inline]
    pub fn serialize_from_ref<U, S>(
        value: &U,
        serializer: &mut S,
    ) -> Result<BoxResolver<U::MetadataResolver>, S::Error>
    where
        U: SerializeUnsized<S, Archived = T> + ?Sized,
        S: Fallible + ?Sized,
    {
        Ok(BoxResolver {
            pos: value.serialize_unsized(serializer)?,
            metadata_resolver: value.serialize_metadata(serializer)?,
        })
    }

    #[doc(hidden)]
    #[inline]
    pub fn is_null(&self) -> bool {
        self.0.is_null()
    }
}

impl<T: ArchivePointee + ?Sized> ArchivedBox<T>
where
    T::ArchivedMetadata: Default,
{
    #[doc(hidden)]
    #[inline]
    pub unsafe fn emplace_null(pos: usize, out: *mut Self) {
        let (fp, fo) = out_field!(out.0);
        RelPtr::emplace_null(pos + fp, fo);
    }
}

impl<T: ArchivePointee + ?Sized> AsRef<T> for ArchivedBox<T> {
    #[inline]
    fn as_ref(&self) -> &T {
        self.get()
    }
}

impl<T: ArchivePointee + ?Sized> Borrow<T> for ArchivedBox<T> {
    #[inline]
    fn borrow(&self) -> &T {
        self.get()
    }
}

impl<T: ArchivePointee + ?Sized> fmt::Debug for ArchivedBox<T>
where
    T::ArchivedMetadata: fmt::Debug,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("ArchivedBox").field(&self.0).finish()
    }
}

impl<T: ArchivePointee + ?Sized> Deref for ArchivedBox<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.get()
    }
}

impl<T: ArchivePointee + fmt::Display + ?Sized> fmt::Display for ArchivedBox<T> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.get().fmt(f)
    }
}

impl<T: ArchivePointee + Eq + ?Sized> Eq for ArchivedBox<T> {}

impl<T: ArchivePointee + hash::Hash + ?Sized> hash::Hash for ArchivedBox<T> {
    #[inline]
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.get().hash(state);
    }
}

impl<T: ArchivePointee + Ord + ?Sized> Ord for ArchivedBox<T> {
    #[inline]
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.as_ref().cmp(other.as_ref())
    }
}

impl<T: ArchivePointee + PartialEq<U> + ?Sized, U: ArchivePointee + ?Sized>
    PartialEq<ArchivedBox<U>> for ArchivedBox<T>
{
    #[inline]
    fn eq(&self, other: &ArchivedBox<U>) -> bool {
        self.get().eq(other.get())
    }
}

impl<T: ArchivePointee + PartialOrd + ?Sized> PartialOrd for ArchivedBox<T> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        self.get().partial_cmp(other.get())
    }
}

impl<T: ArchivePointee + ?Sized> fmt::Pointer for ArchivedBox<T> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ptr = self.get() as *const T;
        fmt::Pointer::fmt(&ptr, f)
    }
}

/// The resolver for `Box`.
pub struct BoxResolver<T> {
    pos: usize,
    metadata_resolver: T,
}

#[cfg(feature = "validation")]
const _: () = {
    use crate::validation::{
        owned::{CheckOwnedPointerError, OwnedPointerError},
        ArchiveContext, LayoutRaw,
    };
    use bytecheck::{CheckBytes, Error};
    use ptr_meta::Pointee;

    impl<T, C> CheckBytes<C> for ArchivedBox<T>
    where
        T: ArchivePointee + CheckBytes<C> + LayoutRaw + Pointee + ?Sized,
        C: ArchiveContext + ?Sized,
        T::ArchivedMetadata: CheckBytes<C>,
        C::Error: Error,
    {
        type Error = CheckOwnedPointerError<T, C>;

        #[inline]
        unsafe fn check_bytes<'a>(
            value: *const Self,
            context: &mut C,
        ) -> Result<&'a Self, Self::Error> {
            let rel_ptr = RelPtr::<T>::manual_check_bytes(value.cast(), context)
                .map_err(OwnedPointerError::PointerCheckBytesError)?;
            let ptr = context
                .check_subtree_rel_ptr(rel_ptr)
                .map_err(OwnedPointerError::ContextError)?;

            let range = context
                .push_prefix_subtree(ptr)
                .map_err(OwnedPointerError::ContextError)?;
            T::check_bytes(ptr, context).map_err(OwnedPointerError::ValueCheckBytesError)?;
            context
                .pop_prefix_range(range)
                .map_err(OwnedPointerError::ContextError)?;

            Ok(&*value)
        }
    }
};
