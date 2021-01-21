#[cfg(feature = "std")]
pub mod adapters;
pub mod serializers;

use core::{mem, slice};
use crate::{
    Archive,
    ArchiveRef,
    Fallible,
    Serialize,
    SerializeRef,
};

/// A `#![no_std]` compliant serializer that knows where it is.
///
/// A type that is [`io::Write`](std::io::Write) can be wrapped in a
/// [`WriteSerializer`] to equip it with `Write`. It's important that the memory
/// for archived objects is properly aligned before attempting to read objects
/// out of it, use the [`Aligned`] wrapper if it's appropriate.
pub trait Serializer: Fallible {
    /// Returns the current position of the serializer.
    fn pos(&self) -> usize;

    /// Attempts to write the given bytes to the serializer.
    fn write(&mut self, bytes: &[u8]) -> Result<(), Self::Error>;

    /// Advances the given number of bytes as padding.
    fn pad(&mut self, mut padding: usize) -> Result<(), Self::Error> {
        const ZEROES_LEN: usize = 16;
        const ZEROES: [u8; ZEROES_LEN] = [0; ZEROES_LEN];

        while padding > 0 {
            let len = usize::min(ZEROES_LEN, padding);
            self.write(&ZEROES[0..len])?;
            padding -= len;
        }

        Ok(())
    }

    /// Aligns the position of the serializer to the given alignment.
    fn align(&mut self, align: usize) -> Result<usize, Self::Error> {
        debug_assert!(align & (align - 1) == 0);

        let offset = self.pos() & (align - 1);
        if offset != 0 {
            self.pad(align - offset)?;
        }
        Ok(self.pos())
    }

    /// Aligns the position of the serializer to be suitable to write the given
    /// type.
    fn align_for<T>(&mut self) -> Result<usize, Self::Error> {
        self.align(mem::align_of::<T>())
    }

    /// Resolves the given value with its resolver and writes the archived type.
    ///
    /// Returns the position of the written archived type.
    ///
    /// # Safety
    ///
    /// This is only safe to call when the serializer is already aligned for the
    /// archived version of the given type.
    unsafe fn resolve_aligned<T: Archive + ?Sized>(
        &mut self,
        value: &T,
        resolver: T::Resolver,
    ) -> Result<usize, Self::Error> {
        let pos = self.pos();
        debug_assert!(pos & (mem::align_of::<T::Archived>() - 1) == 0);
        let archived = &value.resolve(pos, resolver);
        let data = (archived as *const T::Archived).cast::<u8>();
        let len = mem::size_of::<T::Archived>();
        self.write(slice::from_raw_parts(data, len))?;
        Ok(pos)
    }

    /// Archives the given object and returns the position it was archived at.
    fn archive<T: Serialize<Self>>(&mut self, value: &T) -> Result<usize, Self::Error> {
        let resolver = value.serialize(self)?;
        self.align_for::<T::Archived>()?;
        unsafe { self.resolve_aligned(value, resolver) }
    }

    unsafe fn resolve_ref_aligned<T: ArchiveRef + ?Sized>(
        &mut self,
        value: &T,
        resolver: usize,
    ) -> Result<usize, Self::Error> {
        let pos = self.pos();
        debug_assert!(pos & (mem::align_of::<T::Reference>() - 1) == 0);
        let reference = &value.resolve_ref(pos, resolver);
        let data = (reference as *const T::Reference).cast::<u8>();
        let len = mem::size_of::<T::Reference>();
        self.write(slice::from_raw_parts(data, len))?;
        Ok(pos)
    }

    /// Archives a reference to the given object and returns the position it was
    /// archived at.
    fn archive_ref<T: SerializeRef<Self> + ?Sized>(&mut self, value: &T) -> Result<usize, Self::Error> {
        let resolver = value.serialize_ref(self)?;
        self.align_for::<T::Reference>()?;
        unsafe { self.resolve_ref_aligned(value, resolver) }
    }
}

/// A serializer that can seek to an absolute position.
pub trait SeekSerializer: Serializer {
    /// Seeks the serializer to the given absolute position.
    fn seek(&mut self, pos: usize) -> Result<(), Self::Error>;

    /// Archives the given value at the nearest available position. If the
    /// serializer is already aligned, it will archive it at the current position.
    fn serialize_root<T: Serialize<Self>>(&mut self, value: &T) -> Result<usize, Self::Error> {
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

    /// Archives a reference to the given value at the nearest available
    /// position. If the serializer is already aligned, it will archive it at the
    /// current position.
    fn serialize_ref_root<T: SerializeRef<Self> + ?Sized>(
        &mut self,
        value: &T,
    ) -> Result<usize, Self::Error> {
        self.align_for::<T::Reference>()?;
        let pos = self.pos();
        self.seek(pos + mem::size_of::<T::Reference>())?;
        let resolver = value.serialize_ref(self)?;
        self.seek(pos)?;
        unsafe {
            self.resolve_ref_aligned(value, resolver)?;
        }
        Ok(pos)
    }
}

pub trait SharedSerializer: Serializer {
    fn archive_shared<T: ArchiveRef + ?Sized>(&mut self, value: &T) -> Result<usize, Self::Error>
    where
        T: SerializeRef<Self>;
}
