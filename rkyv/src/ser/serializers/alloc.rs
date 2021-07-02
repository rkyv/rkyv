use crate::{ser::Serializer, util::AlignedVec, Archive, ArchiveUnsized, Fallible, RelPtr};
use core::{
    borrow::{Borrow, BorrowMut},
    convert::Infallible,
    mem,
    ptr,
};

/// A serializer made specifically to work with [`AlignedVec`](crate::util::AlignedVec).
///
/// This serializer makes it easier for the compiler to perform emplacement optimizations and may
/// give better performance than a basic `WriteSerializer`.
pub struct AlignedSerializer<A> {
    inner: A,
}

impl<A: Borrow<AlignedVec>> AlignedSerializer<A> {
    /// Creates a new `AlignedSerializer` by wrapping a `Borrow<AlignedVec>`.
    #[inline]
    pub fn new(inner: A) -> Self {
        Self { inner }
    }

    /// Consumes the serializer and returns the underlying type.
    #[inline]
    pub fn into_inner(self) -> A {
        self.inner
    }
}

impl<A> Fallible for AlignedSerializer<A> {
    type Error = Infallible;
}

impl<A: Borrow<AlignedVec> + BorrowMut<AlignedVec>> Serializer for AlignedSerializer<A> {
    #[inline]
    fn pos(&self) -> usize {
        self.inner.borrow().len()
    }

    #[inline]
    fn write(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
        self.inner.borrow_mut().extend_from_slice(bytes);
        Ok(())
    }

    #[inline]
    unsafe fn resolve_aligned<T: Archive + ?Sized>(
        &mut self,
        value: &T,
        resolver: T::Resolver,
    ) -> Result<usize, Self::Error> {
        let pos = self.pos();
        debug_assert_eq!(pos & (mem::align_of::<T::Archived>() - 1), 0);
        let vec = self.inner.borrow_mut();
        let additional = mem::size_of::<T::Archived>();
        vec.reserve(additional);
        vec.set_len(vec.len() + additional);

        if additional > 0 {
            let ptr = vec
                .as_mut_ptr()
                .add(pos)
                .cast::<T::Archived>();
            ptr.write_bytes(0, 1);
            value.resolve(pos, resolver, ptr);
        } else {
            value.resolve(pos, resolver, ptr::NonNull::dangling().as_mut());
        }

        Ok(pos)
    }

    #[inline]
    unsafe fn resolve_unsized_aligned<T: ArchiveUnsized + ?Sized>(
        &mut self,
        value: &T,
        to: usize,
        metadata_resolver: T::MetadataResolver,
    ) -> Result<usize, Self::Error> {
        let from = self.pos();
        debug_assert_eq!(from & (mem::align_of::<RelPtr<T::Archived>>() - 1), 0);
        let vec = self.inner.borrow_mut();
        let additional = mem::size_of::<RelPtr<T::Archived>>();
        vec.reserve(additional);
        vec.set_len(vec.len() + additional);

        let ptr = vec
            .as_mut_ptr()
            .add(from)
            .cast::<RelPtr<T::Archived>>();
        ptr.write_bytes(0, 1);

        value.resolve_unsized(from, to, metadata_resolver, ptr);
        Ok(from)
    }
}
