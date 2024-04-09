use core::{fmt, ptr::copy_nonoverlapping};

use rancor::{fail, Source};

use crate::ser::{Positional, Writer};

#[derive(Debug)]
struct BufferOverflow {
    bytes: usize,
    pos: usize,
    len: usize,
}

impl fmt::Display for BufferOverflow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "overflowed buffer while writing {} bytes at pos {} (len is {})",
            self.bytes, self.pos, self.len,
        )
    }
}

#[cfg(feature = "std")]
const _: () = {
    use std::error::Error;

    impl Error for BufferOverflow {}
};

/// Wraps a byte buffer and equips it with [`Writer`].
///
/// Common uses include archiving in `#![no_std]` environments and archiving
/// small objects without allocating.
///
/// # Examples
/// ```
/// use rkyv::{
///     rancor::{Error, Strategy},
///     ser::{writer::BufferWriter, Writer},
///     util::{access_pos_unchecked, AlignedBytes},
///     Archive, Archived, Serialize,
/// };
///
/// #[derive(Archive, Serialize)]
/// enum Event {
///     Spawn,
///     Speak(String),
///     Die,
/// }
///
/// let event = Event::Speak("Help me!".to_string());
/// let mut buffer_writer = BufferWriter::new(AlignedBytes([0u8; 256]));
/// let serializer = Strategy::<_, Error>::wrap(&mut buffer_writer);
/// let pos = event
///     .serialize_and_resolve(serializer)
///     .expect("failed to archive event");
/// let buf = buffer_writer.into_inner();
/// let archived =
///     unsafe { access_pos_unchecked::<Archived<Event>>(buf.as_ref(), pos) };
/// if let Archived::<Event>::Speak(message) = archived {
///     assert_eq!(message.as_str(), "Help me!");
/// } else {
///     panic!("archived event was of the wrong type");
/// }
/// ```
#[derive(Debug, Default)]
pub struct BufferWriter<T> {
    inner: T,
    pos: usize,
}

impl<T> BufferWriter<T> {
    /// Creates a new archive buffer from a byte buffer.
    #[inline]
    pub fn new(inner: T) -> Self {
        Self::with_pos(inner, 0)
    }

    /// Returns a reference to the underlying value in this `BufferWriter`.
    #[inline]
    pub fn inner(&self) -> &T {
        &self.inner
    }

    /// Returns a mutable reference to the underlying value in this
    /// `BufferWriter`.
    #[inline]
    pub fn inner_mut(&mut self) -> &mut T {
        &mut self.inner
    }

    /// Creates a new archive buffer from a byte buffer. The buffer will start
    /// writing at the given position, but the buffer must contain all bytes
    /// (otherwise the alignments of types may not be correct).
    #[inline]
    pub fn with_pos(inner: T, pos: usize) -> Self {
        Self { inner, pos }
    }

    /// Consumes the serializer and returns the underlying type.
    #[inline]
    pub fn into_inner(self) -> T {
        self.inner
    }
}

impl<T> Positional for BufferWriter<T> {
    #[inline]
    fn pos(&self) -> usize {
        self.pos
    }
}

impl<T: AsMut<[u8]>, E: Source> Writer<E> for BufferWriter<T> {
    fn write(&mut self, bytes: &[u8]) -> Result<(), E> {
        let end_pos = self.pos + bytes.len();
        let len = self.inner.as_mut().len();
        if end_pos > len {
            fail!(BufferOverflow {
                bytes: bytes.len(),
                pos: self.pos,
                len,
            });
        } else {
            unsafe {
                copy_nonoverlapping(
                    bytes.as_ptr(),
                    self.inner.as_mut().as_mut_ptr().add(self.pos),
                    bytes.len(),
                );
            }
            self.pos = end_pos;
            Ok(())
        }
    }
}
