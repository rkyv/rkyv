//! An archived version of `Vec`.

use core::{
    borrow::Borrow,
    cmp, fmt, hash,
    ops::{Deref, Index},
    slice::SliceIndex,
};

use munge::munge;
use rancor::Fallible;

use crate::{
    primitive::{ArchivedUsize, FixedUsize},
    seal::Seal,
    ser::{Allocator, Writer, WriterExt as _},
    Archive, Place, Portable, RelPtr, Serialize, SerializeUnsized,
};

/// An archived [`Vec`].
///
/// This uses a [`RelPtr`] to a `[T]` under the hood. Unlike
/// [`ArchivedString`](crate::string::ArchivedString), it does not have an
/// inline representation.
#[derive(Portable)]
#[cfg_attr(
    feature = "bytecheck",
    derive(bytecheck::CheckBytes),
    bytecheck(verify)
)]
#[rkyv(crate)]
#[repr(C)]
pub struct ArchivedVec<T> {
    ptr: RelPtr<T>,
    len: ArchivedUsize,
}

impl<T> ArchivedVec<T> {
    /// Returns a pointer to the first element of the archived vec.
    pub fn as_ptr(&self) -> *const T {
        unsafe { self.ptr.as_ptr() }
    }

    /// Returns the number of elements in the archived vec.
    pub fn len(&self) -> usize {
        self.len.to_native() as usize
    }

    /// Returns whether the archived vec is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Gets the elements of the archived vec as a slice.
    pub fn as_slice(&self) -> &[T] {
        unsafe { core::slice::from_raw_parts(self.as_ptr(), self.len()) }
    }

    /// Gets the elements of the archived vec as a sealed mutable slice.
    pub fn as_slice_seal(this: Seal<'_, Self>) -> Seal<'_, [T]> {
        let len = this.len();
        munge!(let Self { ptr, .. } = this);
        let slice = unsafe {
            core::slice::from_raw_parts_mut(RelPtr::as_mut_ptr(ptr), len)
        };
        Seal::new(slice)
    }

    /// Resolves an archived `Vec` from a given slice.
    pub fn resolve_from_slice<U: Archive<Archived = T>>(
        slice: &[U],
        resolver: VecResolver,
        out: Place<Self>,
    ) {
        Self::resolve_from_len(slice.len(), resolver, out);
    }

    /// Resolves an archived `Vec` from a given length.
    pub fn resolve_from_len(
        len: usize,
        resolver: VecResolver,
        out: Place<Self>,
    ) {
        munge!(let ArchivedVec { ptr, len: out_len } = out);
        RelPtr::emplace(resolver.pos as usize, ptr);
        usize::resolve(&len, (), out_len);
    }

    /// Serializes an archived `Vec` from a given slice.
    pub fn serialize_from_slice<
        U: Serialize<S, Archived = T>,
        S: Fallible + Allocator + Writer + ?Sized,
    >(
        slice: &[U],
        serializer: &mut S,
    ) -> Result<VecResolver, S::Error> {
        Ok(VecResolver {
            pos: slice.serialize_unsized(serializer)? as FixedUsize,
        })
    }

    /// Serializes an archived `Vec` from a given iterator.
    ///
    /// This method is unable to perform copy optimizations; prefer
    /// [`serialize_from_slice`](ArchivedVec::serialize_from_slice) when
    /// possible.
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
                for (value, resolver) in iter.zip(resolvers.drain()) {
                    unsafe {
                        serializer.resolve_aligned(value.borrow(), resolver)?;
                    }
                }

                Ok(VecResolver {
                    pos: pos as FixedUsize,
                })
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

            Ok(VecResolver {
                pos: pos as FixedUsize,
            })
        }
    }
}

impl<T> AsRef<[T]> for ArchivedVec<T> {
    fn as_ref(&self) -> &[T] {
        self.as_slice()
    }
}

impl<T> Borrow<[T]> for ArchivedVec<T> {
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

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl<T: Eq> Eq for ArchivedVec<T> {}

impl<T: hash::Hash> hash::Hash for ArchivedVec<T> {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.as_slice().hash(state)
    }
}

impl<T, I: SliceIndex<[T]>> Index<I> for ArchivedVec<T> {
    type Output = <[T] as Index<I>>::Output;

    fn index(&self, index: I) -> &Self::Output {
        self.as_slice().index(index)
    }
}

impl<T: Ord> Ord for ArchivedVec<T> {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.as_slice().cmp(other.as_slice())
    }
}

impl<T: PartialEq<U>, U> PartialEq<ArchivedVec<U>> for ArchivedVec<T> {
    fn eq(&self, other: &ArchivedVec<U>) -> bool {
        self.as_slice().eq(other.as_slice())
    }
}

impl<T: PartialEq<U>, U, const N: usize> PartialEq<[U; N]> for ArchivedVec<T> {
    fn eq(&self, other: &[U; N]) -> bool {
        self.as_slice().eq(&other[..])
    }
}

impl<T: PartialEq<U>, U, const N: usize> PartialEq<ArchivedVec<T>> for [U; N] {
    fn eq(&self, other: &ArchivedVec<T>) -> bool {
        other.eq(self)
    }
}

impl<T: PartialEq<U>, U> PartialEq<[U]> for ArchivedVec<T> {
    fn eq(&self, other: &[U]) -> bool {
        self.as_slice().eq(other)
    }
}

impl<T: PartialEq<U>, U> PartialEq<ArchivedVec<U>> for [T] {
    fn eq(&self, other: &ArchivedVec<U>) -> bool {
        self.eq(other.as_slice())
    }
}

impl<T: PartialOrd> PartialOrd<ArchivedVec<T>> for ArchivedVec<T> {
    fn partial_cmp(&self, other: &ArchivedVec<T>) -> Option<cmp::Ordering> {
        self.as_slice().partial_cmp(other.as_slice())
    }
}

impl<T: PartialOrd> PartialOrd<[T]> for ArchivedVec<T> {
    fn partial_cmp(&self, other: &[T]) -> Option<cmp::Ordering> {
        self.as_slice().partial_cmp(other)
    }
}

impl<T: PartialOrd> PartialOrd<ArchivedVec<T>> for [T] {
    fn partial_cmp(&self, other: &ArchivedVec<T>) -> Option<cmp::Ordering> {
        self.partial_cmp(other.as_slice())
    }
}

/// The resolver for [`ArchivedVec`].
pub struct VecResolver {
    pos: FixedUsize,
}

impl VecResolver {
    /// Creates a new `VecResolver` from a position in the output buffer where
    /// the elements of the archived vector are stored.
    pub fn from_pos(pos: usize) -> Self {
        Self {
            pos: pos as FixedUsize,
        }
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
            let ptr = core::ptr::slice_from_raw_parts(
                self.ptr.as_ptr_wrapping(),
                self.len.to_native() as usize,
            );

            context.in_subtree(ptr, |context| unsafe {
                <[T]>::check_bytes(ptr, context)
            })
        }
    }
}
