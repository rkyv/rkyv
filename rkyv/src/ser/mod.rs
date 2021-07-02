//! Serialization traits, serializers, and adapters.

#[cfg(feature = "alloc")]
pub mod adapters;
pub mod serializers;

use crate::{
    Archive, ArchivePointee, ArchiveUnsized, Archived, Fallible, RelPtr, Serialize,
    SerializeUnsized,
};
use core::{mem, slice};

/// A byte sink that knows where it is.
///
/// A type that is [`io::Write`](std::io::Write) can be wrapped in a
/// [`WriteSerializer`](serializers::WriteSerializer) to equip it with `Serializer`.
///
/// It's important that the memory for archived objects is properly aligned before attempting to
/// read objects out of it; use the [`Aligned`](crate::Aligned) wrapper if it's appropriate.
pub trait Serializer: Fallible {
    /// Returns the current position of the serializer.
    fn pos(&self) -> usize;

    /// Attempts to write the given bytes to the serializer.
    fn write(&mut self, bytes: &[u8]) -> Result<(), Self::Error>;

    /// Advances the given number of bytes as padding.
    #[inline]
    fn pad(&mut self, padding: usize) -> Result<(), Self::Error> {
        const MAX_ZEROES: usize = 32;
        const ZEROES: [u8; MAX_ZEROES] = [0; MAX_ZEROES];
        debug_assert!(padding < MAX_ZEROES);

        self.write(&ZEROES[0..padding])
    }

    /// Aligns the position of the serializer to the given alignment.
    #[inline]
    fn align(&mut self, align: usize) -> Result<usize, Self::Error> {
        let mask = align - 1;
        debug_assert_eq!(align & mask, 0);

        self.pad((align - (self.pos() & mask)) & mask)?;
        Ok(self.pos())
    }

    /// Aligns the position of the serializer to be suitable to write the given type.
    #[inline]
    fn align_for<T>(&mut self) -> Result<usize, Self::Error> {
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
    ) -> Result<usize, Self::Error> {
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

    /// Archives the given object and returns the position it was archived at.
    #[inline]
    fn serialize_value<T: Serialize<Self>>(&mut self, value: &T) -> Result<usize, Self::Error> {
        let resolver = value.serialize(self)?;
        self.align_for::<T::Archived>()?;
        unsafe { self.resolve_aligned(value, resolver) }
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
        metadata_resolver: T::MetadataResolver,
    ) -> Result<usize, Self::Error> {
        let from = self.pos();
        debug_assert_eq!(from & (mem::align_of::<RelPtr<T::Archived>>() - 1), 0);

        let mut resolved = mem::MaybeUninit::<RelPtr<T::Archived>>::uninit();
        resolved.as_mut_ptr().write_bytes(0, 1);
        value.resolve_unsized(from, to, metadata_resolver, resolved.as_mut_ptr());

        let data = resolved.as_ptr().cast::<u8>();
        let len = mem::size_of::<RelPtr<T::Archived>>();
        self.write(slice::from_raw_parts(data, len))?;
        Ok(from)
    }

    /// Archives a reference to the given object and returns the position it was archived at.
    #[inline]
    fn serialize_unsized_value<T: SerializeUnsized<Self> + ?Sized>(
        &mut self,
        value: &T,
    ) -> Result<usize, Self::Error> {
        let to = value.serialize_unsized(self)?;
        let metadata_resolver = value.serialize_metadata(self)?;
        self.align_for::<RelPtr<T::Archived>>()?;
        unsafe { self.resolve_unsized_aligned(value, to, metadata_resolver) }
    }
}

/// A serializer that can seek to an absolute position.
pub trait SeekSerializer: Serializer {
    /// Seeks the serializer to the given absolute position.
    fn seek(&mut self, pos: usize) -> Result<(), Self::Error>;

    /// Archives the given value at the nearest available position. If the serializer is already
    /// aligned, it will archive it at the current position.
    fn serialize_front<T: Serialize<Self>>(&mut self, value: &T) -> Result<usize, Self::Error> {
        self.align_for::<T::Archived>()?;
        let pos = self.pos();
        self.seek(pos + mem::size_of::<T::Archived>())?;
        let resolver = value.serialize(self)?;
        self.seek(pos)?;
        unsafe {
            self.resolve_aligned(value, resolver)?;
        }
        Ok(pos)
    }

    /// Archives a reference to the given value at the nearest available position. If the serializer
    /// is already aligned, it will archive it at the current position.
    fn serialize_unsized_front<T: SerializeUnsized<Self> + ?Sized>(
        &mut self,
        value: &T,
    ) -> Result<usize, Self::Error>
    where
        T::Metadata: Serialize<Self>,
        T::Archived: ArchivePointee<ArchivedMetadata = Archived<T::Metadata>>,
    {
        self.align_for::<RelPtr<T::Archived>>()?;
        let pos = self.pos();
        self.seek(pos + mem::size_of::<RelPtr<T::Archived>>())?;
        let to = value.serialize_unsized(self)?;
        let metadata_resolver = value.serialize_metadata(self)?;
        self.seek(pos)?;
        unsafe { self.resolve_unsized_aligned(value, to, metadata_resolver) }
    }
}

/// A serializer that supports serializing shared memory.
///
/// This serializer is required by shared pointers to serialize.
pub trait SharedSerializer: Serializer {
    /// Archives the given shared value and returns its position. If the value has already been
    /// serialized then it returns the position of the previously serialized value.
    fn serialize_shared<T: SerializeUnsized<Self> + ?Sized>(
        &mut self,
        value: &T,
    ) -> Result<usize, Self::Error>;
}
