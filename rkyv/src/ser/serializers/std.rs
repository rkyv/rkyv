use crate::{Archive, ArchiveUnsized, Fallible, ser::{SeekSerializer, Serializer}, util::AlignedVec, RelPtr, Unreachable};
use core::mem;
use std::io;

/// Wraps a type that implements [`io::Write`](std::io::Write) and equips it with [`Serializer`].
///
/// ## Examples
/// ```
/// use rkyv::ser::{serializers::WriteSerializer, Serializer};
///
/// let mut serializer = WriteSerializer::new(Vec::new());
/// assert_eq!(serializer.pos(), 0);
/// serializer.write(&[0u8, 1u8, 2u8, 3u8]);
/// assert_eq!(serializer.pos(), 4);
/// let buf = serializer.into_inner();
/// assert_eq!(buf.len(), 4);
/// assert_eq!(buf, vec![0u8, 1u8, 2u8, 3u8]);
/// ```
pub struct WriteSerializer<W: io::Write> {
    inner: W,
    pos: usize,
}

impl<W: io::Write> WriteSerializer<W> {
    /// Creates a new serializer from a writer.
    #[inline]
    pub fn new(inner: W) -> Self {
        Self::with_pos(inner, 0)
    }

    /// Creates a new serializer from a writer, and assumes that the underlying writer is currently
    /// at the given position.
    #[inline]
    pub fn with_pos(inner: W, pos: usize) -> Self {
        Self { inner, pos }
    }

    /// Consumes the serializer and returns the internal writer used to create it.
    #[inline]
    pub fn into_inner(self) -> W {
        self.inner
    }
}

impl<W: io::Write> Fallible for WriteSerializer<W> {
    type Error = io::Error;
}

impl<W: io::Write> Serializer for WriteSerializer<W> {
    #[inline]
    fn pos(&self) -> usize {
        self.pos
    }

    #[inline]
    fn write(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
        self.pos += self.inner.write(bytes)?;
        Ok(())
    }
}

impl<W: io::Write + io::Seek> SeekSerializer for WriteSerializer<W> {
    #[inline]
    fn seek(&mut self, offset: usize) -> Result<(), Self::Error> {
        self.inner.seek(io::SeekFrom::Start(offset as u64))?;
        self.pos = offset;
        Ok(())
    }
}

/// A serializer made specifically to work with [`AlignedVec`](crate::util::AlignedVec).
///
/// This serializer makes it easier for the compiler to perform emplacement optimizations and may
/// give better performance than a basic `WriteSerializer`.
pub struct AlignedSerializer<'a> {
    inner: &'a mut AlignedVec,
}

impl<'a> AlignedSerializer<'a> {
    /// Creates a new `AlignedSerializer` by wrapping an `AlignedVec`.
    pub fn new(inner: &'a mut AlignedVec) -> Self {
        Self {
            inner,
        }
    }
}

impl<'a> Fallible for AlignedSerializer<'a> {
    type Error = Unreachable;
}

impl<'a> Serializer for AlignedSerializer<'a> {
    #[inline]
    fn pos(&self) -> usize {
        self.inner.len()
    }

    #[inline]
    fn write(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
        self.inner.extend_from_slice(bytes);
        Ok(())
    }

    #[inline]
    unsafe fn resolve_aligned<T: Archive + ?Sized>(&mut self, value: &T, resolver: T::Resolver) -> Result<usize, Self::Error> {
        let pos = self.pos();
        debug_assert!(pos & (mem::align_of::<T::Archived>() - 1) == 0);
        self.inner.reserve(mem::size_of::<T::Archived>());
        self.inner.as_mut_ptr().add(pos).cast::<T::Archived>().write(value.resolve(pos, resolver));
        Ok(pos)
    }

    #[inline]
    unsafe fn resolve_unsized_aligned<T: ArchiveUnsized + ?Sized>(&mut self, value: &T, to: usize, metadata_resolver: T::MetadataResolver) -> Result<usize, Self::Error> {
        let from = self.pos();
        debug_assert!(from & (mem::align_of::<RelPtr<T::Archived>>() - 1) == 0);
        self.inner.reserve(mem::size_of::<RelPtr<T::Archived>>());
        self.inner.as_mut_ptr().add(from).cast::<RelPtr<T::Archived>>().write(value.resolve_unsized(from, to, metadata_resolver));
        Ok(from)
    }
}
