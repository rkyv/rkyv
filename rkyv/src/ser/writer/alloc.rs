use crate::{
    ser::{Positional, Writer},
    util::AlignedVec,
};

impl Positional for Vec<u8> {
    #[inline]
    fn pos(&self) -> usize {
        self.len()
    }
}

impl<E> Writer<E> for Vec<u8> {
    #[inline]
    fn write(&mut self, bytes: &[u8]) -> Result<(), E> {
        self.extend_from_slice(bytes);
        Ok(())
    }
}

impl Positional for AlignedVec {
    #[inline]
    fn pos(&self) -> usize {
        self.len()
    }
}

impl<E> Writer<E> for AlignedVec {
    #[inline]
    fn write(&mut self, bytes: &[u8]) -> Result<(), E> {
        self.extend_from_slice(bytes);
        Ok(())
    }

    // TODO: check whether moving this into an extension trait resulted in a
    // benchmark regression from additional memory copying.

    // #[inline]
    // unsafe fn resolve_aligned<T: Archive + ?Sized>(
    //     &mut self,
    //     value: &T,
    //     resolver: T::Resolver,
    // ) -> Result<usize, E> {
    //     let pos = Serializer::<E>::pos(self);
    //     debug_assert_eq!(pos & (mem::align_of::<T::Archived>() - 1), 0);
    //     let vec = self.inner.borrow_mut();
    //     let additional = mem::size_of::<T::Archived>();
    //     vec.reserve(additional);
    //     vec.set_len(vec.len() + additional);

    //     let ptr = vec.as_mut_ptr().add(pos).cast::<T::Archived>();
    //     ptr.write_bytes(0, 1);
    //     value.resolve(pos, resolver, ptr);

    //     Ok(pos)
    // }

    // #[inline]
    // unsafe fn resolve_unsized_aligned<T: ArchiveUnsized + ?Sized>(
    //     &mut self,
    //     value: &T,
    //     to: usize,
    //     metadata_resolver: T::MetadataResolver,
    // ) -> Result<usize, E> {
    //     let from = Serializer::<E>::pos(self);
    //     debug_assert_eq!(
    //         from & (mem::align_of::<RelPtr<T::Archived>>() - 1),
    //         0
    //     );
    //     let vec = self.inner.borrow_mut();
    //     let additional = mem::size_of::<RelPtr<T::Archived>>();
    //     vec.reserve(additional);
    //     vec.set_len(vec.len() + additional);

    //     let ptr = vec.as_mut_ptr().add(from).cast::<RelPtr<T::Archived>>();
    //     ptr.write_bytes(0, 1);

    //     value.resolve_unsized(from, to, metadata_resolver, ptr);
    //     Ok(from)
    // }
}
