//! Serialization traits, serializers, and adapters.

pub mod serializers;

use crate::{Archive, ArchiveUnsized, RelPtr, SerializeUnsized};
use core::{alloc::Layout, mem, ptr::NonNull, slice};
use rancor::{Fallible, Strategy};

// TODO: try to make calling these methods more ergonomic in contexts where `E`
// isn't well-defined.

/// A byte sink that knows where it is.
///
/// A type that is [`io::Write`](std::io::Write) can be wrapped in a
/// [`WriteSerializer`](serializers::WriteSerializer) to equip it with `Serializer`.
///
/// It's important that the memory for archived objects is properly aligned before attempting to
/// read objects out of it; use an [`AlignedVec`](crate::util::AlignedVec) or the
/// [`AlignedBytes`](crate::util::AlignedBytes) wrappers if they are appropriate.
pub trait Serializer<E = <Self as Fallible>::Error> {
    /// Returns the current position of the serializer.
    fn pos(&self) -> usize;

    /// Attempts to write the given bytes to the serializer.
    fn write(&mut self, bytes: &[u8]) -> Result<(), E>;
}

impl<T, E> Serializer<E> for Strategy<T, E>
where
    T: Serializer<E> + ?Sized,
{
    fn pos(&self) -> usize {
        T::pos(self)
    }

    fn write(&mut self, bytes: &[u8]) -> Result<(), E> {
        T::write(self, bytes)
    }
}

/// TODO: Document
pub trait SerializerExt<E>: Serializer<E> {
    /// Advances the given number of bytes as padding.
    #[inline]
    fn pad(&mut self, padding: usize) -> Result<(), E> {
        const MAX_ZEROES: usize = 32;
        const ZEROES: [u8; MAX_ZEROES] = [0; MAX_ZEROES];
        debug_assert!(padding < MAX_ZEROES);

        self.write(&ZEROES[0..padding])
    }

    /// Aligns the position of the serializer to the given alignment.
    #[inline]
    fn align(&mut self, align: usize) -> Result<usize, E> {
        let mask = align - 1;
        debug_assert_eq!(align & mask, 0);

        self.pad((align - (self.pos() & mask)) & mask)?;
        Ok(self.pos())
    }

    /// Aligns the position of the serializer to be suitable to write the given type.
    #[inline]
    fn align_for<T>(&mut self) -> Result<usize, E> {
        self.align(mem::align_of::<T>())
    }

    /// Resolves the given value with its resolver and writes the archived type.
    ///
    /// Returns the position of the written archived type.
    ///
    /// # Safety
    ///
    /// - `resolver` must be the result of serializing `value`
    /// - The serializer must be aligned for a `T::Archived`
    unsafe fn resolve_aligned<T: Archive + ?Sized>(
        &mut self,
        value: &T,
        resolver: T::Resolver,
    ) -> Result<usize, E> {
        let pos = self.pos();
        debug_assert_eq!(pos & (mem::align_of::<T::Archived>() - 1), 0);

        let mut resolved = mem::MaybeUninit::<T::Archived>::uninit();
        resolved.as_mut_ptr().write_bytes(0, 1);
        value.resolve(pos, resolver, resolved.as_mut_ptr());

        let data = resolved.as_ptr().cast::<u8>();
        let len = mem::size_of::<T::Archived>();
        self.write(slice::from_raw_parts(data, len))?;
        Ok(pos)
    }

    /// Resolves the given reference with its resolver and writes the archived reference.
    ///
    /// Returns the position of the written archived `RelPtr`.
    ///
    /// # Safety
    ///
    /// - `metadata_resolver` must be the result of serializing the metadata of `value`
    /// - `to` must be the position of the serialized `value` within the archive
    /// - The serializer must be aligned for a `RelPtr<T::Archived>`
    unsafe fn resolve_unsized_aligned<T: ArchiveUnsized + ?Sized>(
        &mut self,
        value: &T,
        to: usize,
    ) -> Result<usize, E> {
        let from = self.pos();
        debug_assert_eq!(
            from & (mem::align_of::<RelPtr<T::Archived>>() - 1),
            0
        );

        let mut resolved = mem::MaybeUninit::<RelPtr<T::Archived>>::uninit();
        resolved.as_mut_ptr().write_bytes(0, 1);
        RelPtr::resolve_emplace(
            from,
            to,
            value.archived_metadata(),
            resolved.as_mut_ptr(),
        );

        let data = resolved.as_ptr().cast::<u8>();
        let len = mem::size_of::<RelPtr<T::Archived>>();
        self.write(slice::from_raw_parts(data, len))?;
        Ok(from)
    }
}

impl<T, E> SerializerExt<E> for T where T: Serializer<E> + ?Sized {}

// Someday this can probably be replaced with alloc::Allocator

/// A serializer that can allocate scratch space.
pub trait ScratchSpace<E = <Self as Fallible>::Error> {
    /// Allocates scratch space of the requested size.
    ///
    /// # Safety
    ///
    /// `layout` must have non-zero size.
    unsafe fn push_scratch(
        &mut self,
        layout: Layout,
    ) -> Result<NonNull<[u8]>, E>;

    /// Deallocates previously allocated scratch space.
    ///
    /// # Safety
    ///
    /// - `ptr` must be the scratch memory last allocated with `push_scratch`.
    /// - `layout` must be the same layout that was used to allocate that block of memory.
    unsafe fn pop_scratch(
        &mut self,
        ptr: NonNull<u8>,
        layout: Layout,
    ) -> Result<(), E>;
}

impl<T: ScratchSpace<E>, E> ScratchSpace<E> for Strategy<T, E> {
    unsafe fn push_scratch(
        &mut self,
        layout: Layout,
    ) -> Result<NonNull<[u8]>, E> {
        T::push_scratch(self, layout)
    }

    unsafe fn pop_scratch(
        &mut self,
        ptr: NonNull<u8>,
        layout: Layout,
    ) -> Result<(), E> {
        T::pop_scratch(self, ptr, layout)
    }
}

// TODO: Make this name shorter

/// A registry that tracks serialized shared memory.
///
/// This trait is required to serialize shared pointers.
pub trait SharedSerializeRegistry<E = <Self as Fallible>::Error> {
    /// Gets the position of a previously-added shared pointer.
    ///
    /// Returns `None` if the pointer has not yet been added.
    fn get_shared_ptr(&self, value: *const u8) -> Option<usize>;

    /// Adds the position of a shared pointer to the registry.
    fn add_shared_ptr(&mut self, value: *const u8, pos: usize)
        -> Result<(), E>;
}

impl<T, E> SharedSerializeRegistry<E> for Strategy<T, E>
where
    T: SharedSerializeRegistry<E> + ?Sized,
{
    fn get_shared_ptr(&self, value: *const u8) -> Option<usize> {
        T::get_shared_ptr(self, value)
    }

    fn add_shared_ptr(
        &mut self,
        value: *const u8,
        pos: usize,
    ) -> Result<(), E> {
        T::add_shared_ptr(self, value, pos)
    }
}

/// TODO: Document this
pub trait SharedSerializeRegistryExt<E>: SharedSerializeRegistry<E> {
    /// Gets the position of a previously-added shared value.
    ///
    /// Returns `None` if the value has not yet been added.
    #[inline]
    fn get_shared<T: ?Sized>(&self, value: &T) -> Option<usize> {
        self.get_shared_ptr(value as *const T as *const u8)
    }

    /// Adds the position of a shared value to the registry.
    #[inline]
    fn add_shared<T: ?Sized>(
        &mut self,
        value: &T,
        pos: usize,
    ) -> Result<(), E> {
        self.add_shared_ptr(value as *const T as *const u8, pos)
    }

    /// Archives the given shared value and returns its position. If the value has already been
    /// added then it returns the position of the previously added value.
    #[inline]
    fn serialize_shared<T: SerializeUnsized<Self> + ?Sized>(
        &mut self,
        value: &T,
    ) -> Result<usize, <Self as Fallible>::Error>
    where
        Self: Fallible<Error = E>,
    {
        if let Some(pos) = self.get_shared(value) {
            Ok(pos)
        } else {
            let pos = value.serialize_unsized(self)?;
            self.add_shared(value, pos)?;
            Ok(pos)
        }
    }
}

impl<S, E> SharedSerializeRegistryExt<E> for S where
    S: SharedSerializeRegistry<E> + ?Sized
{
}
