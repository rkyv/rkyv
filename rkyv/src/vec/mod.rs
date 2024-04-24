//! An archived version of `Vec`.

use core::{
    borrow::Borrow,
    cmp, fmt, hash,
    ops::{Deref, Index, IndexMut},
    pin::Pin,
    slice::SliceIndex,
};

use munge::munge;
use rancor::Fallible;

use crate::{
    primitive::ArchivedUsize,
    ser::{Allocator, Writer, WriterExt as _},
    Archive, Place, Portable, RelPtr, Serialize, SerializeUnsized,
};

// pub use self::raw::*;

/// An archived [`Vec`].
///
/// This uses a [`RelPtr`] to a `[T]` under the hood. Unlike
/// [`ArchivedString`](crate::string::ArchivedString), it does not have an
/// inline representation.
#[derive(Portable)]
#[archive(crate)]
#[repr(C)]
#[cfg_attr(
    feature = "bytecheck",
    derive(bytecheck::CheckBytes),
    check_bytes(verify)
)]
pub struct ArchivedVec<T> {
    ptr: RelPtr<T>,
    len: ArchivedUsize,
}

impl<T> ArchivedVec<T> {
    /// Returns a pointer to the first element of the archived vec.
    #[inline]
    pub fn as_ptr(&self) -> *const T {
        unsafe { self.ptr.as_ptr() }
    }

    /// Returns the number of elements in the archived vec.
    #[inline]
    pub fn len(&self) -> usize {
        self.len.to_native() as usize
    }

    /// Returns whether the archived vec is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Gets the elements of the archived vec as a slice.
    #[inline]
    pub fn as_slice(&self) -> &[T] {
        unsafe { core::slice::from_raw_parts(self.as_ptr(), self.len()) }
    }

    /// Gets the elements of the archived vec as a pinned mutable slice.
    #[inline]
    pub fn pin_mut_slice(self: Pin<&mut Self>) -> Pin<&mut [T]> {
        let len = self.len();
        let ptr = unsafe { self.map_unchecked_mut(|s| &mut s.ptr) };
        unsafe {
            Pin::new_unchecked(core::slice::from_raw_parts_mut(
                ptr.as_mut_ptr(),
                len,
            ))
        }
    }

    // This method can go away once pinned slices have indexing support
    // https://github.com/rust-lang/rust/pull/78370

    /// Gets the element at the given index to this archived vec as a pinned
    /// mutable reference.
    #[inline]
    pub fn index_pin<I>(
        self: Pin<&mut Self>,
        index: I,
    ) -> Pin<&mut <[T] as Index<I>>::Output>
    where
        [T]: IndexMut<I>,
    {
        unsafe { self.pin_mut_slice().map_unchecked_mut(|s| &mut s[index]) }
    }

    /// Resolves an archived `Vec` from a given slice.
    #[inline]
    pub fn resolve_from_slice<U: Archive<Archived = T>>(
        slice: &[U],
        resolver: VecResolver,
        out: Place<Self>,
    ) {
        Self::resolve_from_len(slice.len(), resolver, out);
    }

    /// Resolves an archived `Vec` from a given length.
    #[inline]
    pub fn resolve_from_len(
        len: usize,
        resolver: VecResolver,
        out: Place<Self>,
    ) {
        munge!(let ArchivedVec { ptr, len: out_len } = out);
        RelPtr::emplace(resolver.pos, ptr);
        usize::resolve(&len, (), out_len);
    }

    /// Serializes an archived `Vec` from a given slice.
    #[inline]
    pub fn serialize_from_slice<
        U: Serialize<S, Archived = T>,
        S: Fallible + Allocator + Writer + ?Sized,
    >(
        slice: &[U],
        serializer: &mut S,
    ) -> Result<VecResolver, S::Error> {
        Ok(VecResolver {
            pos: slice.serialize_unsized(serializer)?,
        })
    }

    /// Serializes an archived `Vec` from a given slice by directly copying
    /// bytes.
    ///
    /// # Safety
    ///
    /// The type being serialized must be copy-safe. Copy-safe types must be
    /// trivially copyable (have the same archived and unarchived
    /// representations) and contain no padding bytes. In situations where
    /// copying uninitialized bytes the output is acceptable, this function may
    /// be used with types that contain padding bytes.
    #[inline]
    pub unsafe fn serialize_copy_from_slice<U, S>(
        slice: &[U],
        serializer: &mut S,
    ) -> Result<VecResolver, S::Error>
    where
        U: Serialize<S, Archived = T>,
        S: Fallible + Writer + ?Sized,
    {
        use core::{mem::size_of, slice::from_raw_parts};

        let pos = serializer.align_for::<T>()?;

        let bytes = from_raw_parts(
            slice.as_ptr().cast::<u8>(),
            size_of::<T>() * slice.len(),
        );
        serializer.write(bytes)?;

        Ok(VecResolver { pos })
    }

    // TODO: try to remove `U` parameter
    /// Serializes an archived `Vec` from a given iterator.
    ///
    /// This method is unable to perform copy optimizations; prefer
    /// [`serialize_from_slice`](ArchivedVec::serialize_from_slice) when
    /// possible.
    #[inline]
    pub fn serialize_from_iter<U, I, S>(
        iter: I,
        serializer: &mut S,
    ) -> Result<VecResolver, S::Error>
    where
        U: Serialize<S, Archived = T>,
        I: ExactSizeIterator + Clone,
        I::Item: Borrow<U>,
        S: Fallible + Allocator + Writer + ?Sized,
    {
        use crate::util::SerVec;

        SerVec::with_capacity(
            serializer,
            iter.len(),
            |resolvers, serializer| {
                for value in iter.clone() {
                    let resolver = value.borrow().serialize(serializer)?;
                    resolvers.push(resolver);
                }

                let pos = serializer.align_for::<T>()?;
                for (value, resolver) in iter.zip(resolvers.drain(..)) {
                    unsafe {
                        serializer.resolve_aligned(value.borrow(), resolver)?;
                    }
                }

                Ok(VecResolver { pos })
            },
        )?
    }

    /// Serializes an archived `Vec` from a given iterator. Compared to
    /// `serialize_from_iter()`, this function:
    /// - supports iterators whose length is not known in advance, and
    /// - does not collect the data in memory before serializing.
    ///
    /// This method will panic if any item writes during `serialize` (i.e no
    /// additional data written per item).
    #[inline]
    pub fn serialize_from_unknown_length_iter<B, I, S>(
        iter: &mut I,
        serializer: &mut S,
    ) -> Result<VecResolver, S::Error>
    where
        B: Serialize<S, Archived = T>,
        I: Iterator<Item = B>,
        S: Fallible + Allocator + Writer + ?Sized,
    {
        unsafe {
            let pos = serializer.align_for::<T>()?;

            for value in iter {
                let pos_cached = serializer.pos();
                let resolver = value.serialize(serializer)?;
                assert!(serializer.pos() == pos_cached);
                serializer.resolve_aligned(value.borrow(), resolver)?;
            }

            Ok(VecResolver { pos })
        }
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

impl<T: fmt::Debug> fmt::Debug for ArchivedVec<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.as_slice()).finish()
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

impl<T: PartialEq<U>, U, const N: usize> PartialEq<[U; N]> for ArchivedVec<T> {
    #[inline]
    fn eq(&self, other: &[U; N]) -> bool {
        self.as_slice().eq(&other[..])
    }
}

impl<T: PartialEq<U>, U, const N: usize> PartialEq<ArchivedVec<T>> for [U; N] {
    #[inline]
    fn eq(&self, other: &ArchivedVec<T>) -> bool {
        other.eq(self)
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
pub struct VecResolver {
    pos: usize,
}

impl VecResolver {
    /// Creates a new `VecResolver` from a position in the output buffer where
    /// the elements of the archived vector are stored.
    pub fn from_pos(pos: usize) -> Self {
        Self { pos }
    }
}

#[cfg(feature = "bytecheck")]
mod verify {
    use bytecheck::{
        rancor::{Fallible, Source},
        CheckBytes, Verify,
    };

    use crate::{
        validation::{ArchiveContext, ArchiveContextExt},
        vec::ArchivedVec,
    };

    unsafe impl<T, C> Verify<C> for ArchivedVec<T>
    where
        T: CheckBytes<C>,
        C: Fallible + ArchiveContext + ?Sized,
        C::Error: Source,
    {
        fn verify(&self, context: &mut C) -> Result<(), C::Error> {
            let ptr = unsafe {
                context.bounds_check_subtree_base_offset::<[T]>(
                    self.ptr.base(),
                    self.ptr.offset(),
                    self.len.to_native() as usize,
                )?
            };

            let range = unsafe { context.push_prefix_subtree(ptr)? };
            unsafe {
                <[T]>::check_bytes(ptr, context)?;
            }
            unsafe {
                context.pop_subtree_range(range)?;
            }

            Ok(())
        }
    }
}
