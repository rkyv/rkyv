//! Writing backends for serializers.

#[cfg(feature = "alloc")]
mod alloc;
mod core;
#[cfg(feature = "std")]
mod std;

use ::core::mem;
use rancor::{Fallible, Strategy};

pub use self::core::*;
#[cfg(feature = "std")]
pub use self::std::*;
use crate::{Archive, ArchiveUnsized, Place, RelPtr};

/// A writer that knows its current position.
pub trait Positional {
    /// Returns the current position of the writer.
    fn pos(&self) -> usize;
}

impl<T> Positional for &T
where
    T: Positional + ?Sized,
{
    fn pos(&self) -> usize {
        T::pos(*self)
    }
}

impl<T> Positional for &mut T
where
    T: Positional + ?Sized,
{
    fn pos(&self) -> usize {
        T::pos(*self)
    }
}

impl<T, E> Positional for Strategy<T, E>
where
    T: Positional + ?Sized,
{
    fn pos(&self) -> usize {
        T::pos(self)
    }
}

/// A type that writes bytes to some output.
///
/// A type that is [`Write`](::std::io::Write) can be wrapped in an [`IoWriter`]
/// to equip it with `Writer`.
///
/// It's important that the memory for archived objects is properly aligned
/// before attempting to read objects out of it; use an
/// [`AlignedVec`](crate::util::AlignedVec) or the [`Align`](crate::util::Align)
/// wrapper as appropriate.
pub trait Writer<E = <Self as Fallible>::Error>: Positional {
    /// Attempts to write the given bytes to the serializer.
    fn write(&mut self, bytes: &[u8]) -> Result<(), E>;
}

impl<T, E> Writer<E> for &mut T
where
    T: Writer<E> + ?Sized,
{
    fn write(&mut self, bytes: &[u8]) -> Result<(), E> {
        T::write(*self, bytes)
    }
}

impl<T, E> Writer<E> for Strategy<T, E>
where
    T: Writer<E> + ?Sized,
{
    fn write(&mut self, bytes: &[u8]) -> Result<(), E> {
        T::write(self, bytes)
    }
}

/// Helper methods for [`Writer`].
pub trait WriterExt<E>: Writer<E> {
    /// Advances the given number of bytes as padding.
    fn pad(&mut self, padding: usize) -> Result<(), E> {
        const MAX_ZEROS: usize = 32;
        const ZEROS: [u8; MAX_ZEROS] = [0; MAX_ZEROS];
        debug_assert!(padding < MAX_ZEROS);

        self.write(&ZEROS[0..padding])
    }

    /// Aligns the position of the serializer to the given alignment.
    fn align(&mut self, align: usize) -> Result<usize, E> {
        let mask = align - 1;
        debug_assert_eq!(align & mask, 0);

        self.pad((align - (self.pos() & mask)) & mask)?;
        Ok(self.pos())
    }

    /// Aligns the position of the serializer to be suitable to write the given
    /// type.
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
        // SAFETY: `resolved` is properly aligned and valid for writes of
        // `size_of::<T::Archived>()` bytes.
        unsafe {
            resolved.as_mut_ptr().write_bytes(0, 1);
        }
        // SAFETY: `resolved.as_mut_ptr()` points to a local zeroed
        // `MaybeUninit`, and so is properly aligned, dereferenceable, and all
        // of its bytes are initialized.
        let out = unsafe { Place::new_unchecked(pos, resolved.as_mut_ptr()) };
        value.resolve(resolver, out);
        self.write(out.as_slice())?;
        Ok(pos)
    }

    /// Resolves the given reference with its resolver and writes the archived
    /// reference.
    ///
    /// Returns the position of the written archived `RelPtr`.
    ///
    /// # Safety
    ///
    /// The serializer must be aligned for a `RelPtr<T::Archived>`.
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
        // SAFETY: `resolved` is properly aligned and valid for writes of
        // `size_of::<RelPtr<T::Archived>>()` bytes.
        unsafe {
            resolved.as_mut_ptr().write_bytes(0, 1);
        }
        // SAFETY: `resolved.as_mut_ptr()` points to a local zeroed
        // `MaybeUninit`, and so is properly aligned, dereferenceable, and all
        // of its bytes are initialized.
        let out = unsafe { Place::new_unchecked(from, resolved.as_mut_ptr()) };
        RelPtr::emplace_unsized(to, value.archived_metadata(), out);

        self.write(out.as_slice())?;
        Ok(from)
    }
}

impl<T, E> WriterExt<E> for T where T: Writer<E> + ?Sized {}

#[cfg(test)]
mod tests {
    #[cfg(feature = "alloc")]
    #[test]
    fn reusable_writer() {
        use rend::{u16_le, u32_le};

        use crate::{api::high::to_bytes_in, util::AlignedVec};

        let mut writer = AlignedVec::<16>::new();

        _ = to_bytes_in::<_, rancor::Error>(
            &u32_le::from_native(42),
            &mut writer,
        );
        assert_eq!(&writer[..], &[42, 0, 0, 0]);
        writer.clear(); // keeps capacity of 4

        _ = to_bytes_in::<_, rancor::Error>(
            &u16_le::from_native(1337),
            &mut writer,
        );
        assert_eq!(&writer[..], &[57, 5]);
        writer.clear();

        assert_eq!(writer.capacity(), 4);
    }
}
