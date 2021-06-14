//! An archived version of `Vec`.

use crate::{
    ser::Serializer,
    Archive,
    ArchiveUnsized,
    MetadataResolver,
    Serialize,
    SerializeUnsized,
    RelPtr,
};
use core::{
    borrow::Borrow,
    cmp,
    hash,
    mem::MaybeUninit,
    ops::{Deref, Index, IndexMut},
    pin::Pin,
    slice::SliceIndex,
};

/// An archived [`Vec`].
///
/// Uses a [`RelPtr`] to a `T` slice under the hood.
#[derive(Debug)]
#[repr(transparent)]
pub struct ArchivedVec<T>(RelPtr<[T]>);

impl<T> ArchivedVec<T> {
    /// Gets the elements of the archived vec as a slice.
    #[inline]
    pub fn as_slice(&self) -> &[T] {
        unsafe { &*self.0.as_ptr() }
    }

    /// Gets the elements of the archived vec as a pinned mutable slice.
    #[inline]
    pub fn as_pin_mut_slice(self: Pin<&mut Self>) -> Pin<&mut [T]> {
        unsafe { self.map_unchecked_mut(|s| &mut *s.0.as_mut_ptr()) }
    }

    // This method can go away once pinned slices have indexing support
    // https://github.com/rust-lang/rust/pull/78370

    /// Gets the element at the given index ot this archived vec as a pinned mutable reference.
    #[inline]
    pub fn index_pin<I>(self: Pin<&mut Self>, index: I) -> Pin<&mut <[T] as Index<I>>::Output>
    where
        [T]: IndexMut<I>,
    {
        unsafe { self.map_unchecked_mut(|s| &mut (*s.0.as_mut_ptr())[index]) }
    }

    #[inline]
    pub unsafe fn resolve_from_slice<U: Archive<Archived = T>>(slice: &[U], pos: usize, resolver: VecResolver<U>, out: &mut MaybeUninit<Self>) {
        let (fp, fo) = out_field!(out.0);
        slice.resolve_unsized(pos + fp, resolver.pos, resolver.metadata_resolver, fo);
    }

    #[inline]
    pub fn serialize_from_slice<U: Serialize<S, Archived = T>, S: Serializer + ?Sized>(slice: &[U], serializer: &mut S) -> Result<VecResolver<U>, S::Error> {
        Ok(VecResolver {
            pos: slice.serialize_unsized(serializer)?,
            metadata_resolver: slice.serialize_metadata(serializer)?,
        })
    }
}

impl<T> AsRef<[T]> for ArchivedVec<T> {
    #[inline]
    fn as_ref(&self) -> &[T] {
        self.as_slice()
    }
}

impl<T> Borrow<[T]> for ArchivedVec<T> {
    #[inline]
    fn borrow(&self) -> &[T] {
        self.as_slice()
    }
}

impl<T> Deref for ArchivedVec<T> {
    type Target = [T];

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl<T: Eq> Eq for ArchivedVec<T> {}

impl<T: hash::Hash> hash::Hash for ArchivedVec<T> {
    #[inline]
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.as_slice().hash(state)
    }
}

impl<T, I: SliceIndex<[T]>> Index<I> for ArchivedVec<T> {
    type Output = <[T] as Index<I>>::Output;

    #[inline]
    fn index(&self, index: I) -> &Self::Output {
        self.as_slice().index(index)
    }
}

impl<T: Ord> Ord for ArchivedVec<T> {
    #[inline]
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.as_slice().cmp(other.as_slice())
    }
}

impl<T: PartialEq<U>, U> PartialEq<ArchivedVec<U>> for ArchivedVec<T> {
    #[inline]
    fn eq(&self, other: &ArchivedVec<U>) -> bool {
        self.as_slice().eq(other.as_slice())
    }
}

impl<T: PartialEq<U>, U> PartialEq<[U]> for ArchivedVec<T> {
    #[inline]
    fn eq(&self, other: &[U]) -> bool {
        self.as_slice().eq(other)
    }
}

impl<T: PartialEq<U>, U> PartialEq<ArchivedVec<U>> for [T] {
    #[inline]
    fn eq(&self, other: &ArchivedVec<U>) -> bool {
        self.eq(other.as_slice())
    }
}

impl<T: PartialOrd> PartialOrd<ArchivedVec<T>> for ArchivedVec<T> {
    #[inline]
    fn partial_cmp(&self, other: &ArchivedVec<T>) -> Option<cmp::Ordering> {
        self.as_slice().partial_cmp(other.as_slice())
    }
}

impl<T: PartialOrd> PartialOrd<[T]> for ArchivedVec<T> {
    #[inline]
    fn partial_cmp(&self, other: &[T]) -> Option<cmp::Ordering> {
        self.as_slice().partial_cmp(other)
    }
}

impl<T: PartialOrd> PartialOrd<ArchivedVec<T>> for [T] {
    #[inline]
    fn partial_cmp(&self, other: &ArchivedVec<T>) -> Option<cmp::Ordering> {
        self.partial_cmp(other.as_slice())
    }
}

/// The resolver for [`ArchivedVec`].
pub struct VecResolver<T: Archive> {
    pos: usize,
    metadata_resolver: MetadataResolver<[T]>,
}

#[cfg(feature = "validation")]
const _: () = {
    use crate::{
        validation::{
            owned::{CheckOwnedPointerError, OwnedPointerError},
            ArchiveBoundsContext,
            ArchiveMemoryContext,
            LayoutMetadata,
        },
        ArchivePointee,
    };
    use bytecheck::CheckBytes;
    use ptr_meta::Pointee;

    impl<T: CheckBytes<C>, C: ArchiveBoundsContext + ArchiveMemoryContext + ?Sized> CheckBytes<C>
        for ArchivedVec<T>
    where
        [T]: ArchivePointee,
        <[T] as ArchivePointee>::ArchivedMetadata: CheckBytes<C>,
        C::Error: std::error::Error,
        <[T] as Pointee>::Metadata: LayoutMetadata<[T]>,
    {
        type Error = CheckOwnedPointerError<[T], C>;

        unsafe fn check_bytes<'a>(
            value: *const Self,
            context: &mut C,
        ) -> Result<&'a Self, Self::Error> {
            let rel_ptr = RelPtr::<[T]>::manual_check_bytes(value.cast(), context)
                .map_err(OwnedPointerError::PointerCheckBytesError)?;
            let ptr = context
                .claim_owned_rel_ptr(rel_ptr)
                .map_err(OwnedPointerError::ContextError)?;
            <[T]>::check_bytes(ptr, context).map_err(OwnedPointerError::ValueCheckBytesError)?;
            Ok(&*value)
        }
    }
};
