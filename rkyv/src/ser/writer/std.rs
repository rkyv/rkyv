use std::io;

use rancor::ResultExt as _;

use crate::ser::{Positional, Writer};

/// Wraps a type that implements [`io::Write`](std::io::Write) and equips it
/// with [`Writer`].
///
/// # Examples
/// ```
/// use rkyv::ser::{serializers::IoWriter, Serializer};
///
/// let mut serializer = IoWriter::new(Vec::new());
/// assert_eq!(serializer.pos(), 0);
/// serializer.write(&[0u8, 1u8, 2u8, 3u8]);
/// assert_eq!(serializer.pos(), 4);
/// let buf = serializer.into_inner();
/// assert_eq!(buf.len(), 4);
/// assert_eq!(buf, vec![0u8, 1u8, 2u8, 3u8]);
/// ```
#[derive(Debug)]
pub struct IoWriter<W> {
    inner: W,
    pos: usize,
}

impl<W> IoWriter<W> {
    /// Creates a new serializer from a writer.
    #[inline]
    pub fn new(inner: W) -> Self {
        Self::with_pos(inner, 0)
    }

    /// Creates a new serializer from a writer, and assumes that the underlying
    /// writer is currently at the given position.
    #[inline]
    pub fn with_pos(inner: W, pos: usize) -> Self {
        Self { inner, pos }
    }

    /// Consumes the serializer and returns the internal writer used to create
    /// it.
    #[inline]
    pub fn into_inner(self) -> W {
        self.inner
    }
}

impl<W> Positional for IoWriter<W> {
    #[inline]
    fn pos(&self) -> usize {
        self.pos
    }
}

impl<W: io::Write, E: rancor::Error> Writer<E> for IoWriter<W> {
    #[inline]
    fn write(&mut self, bytes: &[u8]) -> Result<(), E> {
        self.inner.write_all(bytes).into_error()?;
        self.pos += bytes.len();
        Ok(())
    }
}
