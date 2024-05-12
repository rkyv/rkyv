//! Serialization traits and adapters.

pub mod allocator;
pub mod sharing;
pub mod writer;

use ::core::{alloc::Layout, ptr::NonNull};
use rancor::Strategy;

#[doc(inline)]
pub use self::{
    allocator::Allocator,
    sharing::{Sharing, SharingExt},
    writer::{Positional, Writer, WriterExt},
};
use crate::ser::{
    allocator::{ArenaHandle, SubAllocator},
    sharing::Unshare,
    writer::Buffer,
};
#[cfg(feature = "alloc")]
use crate::{ser::sharing::Share, util::AlignedVec};

/// A serializer built from composeable pieces.
#[derive(Debug, Default)]
pub struct Serializer<W = (), A = (), S = ()> {
    /// The writer of the `Composite` serializer.
    pub writer: W,
    /// The allocator of the `Composite` serializer.
    pub allocator: A,
    /// The shared pointer strategy of the `Composite` serializer.
    pub share: S,
}

impl<W, A, S> Serializer<W, A, S> {
    /// Creates a new composite serializer from serializer, scratch, and shared
    /// pointer strategy.
    #[inline]
    pub fn new(writer: W, allocator: A, share: S) -> Self {
        Self {
            writer,
            allocator,
            share,
        }
    }

    /// Consumes the composite serializer and returns the components.
    #[inline]
    pub fn into_raw_parts(self) -> (W, A, S) {
        (self.writer, self.allocator, self.share)
    }

    /// Consumes the composite serializer and returns the serializer.
    ///
    /// The allocator and shared pointer strategies are discarded.
    #[inline]
    pub fn into_writer(self) -> W {
        self.writer
    }
}

impl<W: Positional, A, S> Positional for Serializer<W, A, S> {
    #[inline]
    fn pos(&self) -> usize {
        self.writer.pos()
    }
}

impl<W: Writer<E>, A, S, E> Writer<E> for Serializer<W, A, S> {
    #[inline]
    fn write(&mut self, bytes: &[u8]) -> Result<(), E> {
        self.writer.write(bytes)
    }
}

unsafe impl<W, A: Allocator<E>, S, E> Allocator<E> for Serializer<W, A, S> {
    #[inline]
    unsafe fn push_alloc(
        &mut self,
        layout: Layout,
    ) -> Result<NonNull<[u8]>, E> {
        // SAFETY: The safety requirements for `A::push_alloc()` are the same as
        // the safety requirements for `push_alloc()`.
        unsafe { self.allocator.push_alloc(layout) }
    }

    #[inline]
    unsafe fn pop_alloc(
        &mut self,
        ptr: NonNull<u8>,
        layout: Layout,
    ) -> Result<(), E> {
        // SAFETY: The safety requirements for `A::pop_alloc()` are the same as
        // the safety requirements for `pop_alloc()`.
        unsafe { self.allocator.pop_alloc(ptr, layout) }
    }
}

impl<W, A, S: Sharing<E>, E> Sharing<E> for Serializer<W, A, S> {
    #[inline]
    fn get_shared_ptr(&self, address: usize) -> Option<usize> {
        self.share.get_shared_ptr(address)
    }

    #[inline]
    fn add_shared_ptr(&mut self, address: usize, pos: usize) -> Result<(), E> {
        self.share.add_shared_ptr(address, pos)
    }
}

/// A serializer suitable for environments where allocations cannot be made.
pub type CoreSerializer<'a, E> =
    Strategy<Serializer<Buffer<'a>, SubAllocator<'a>, Unshare>, E>;

/// A general-purpose serializer suitable for environments where allocations can
/// be made.
#[cfg(feature = "alloc")]
pub type DefaultSerializer<'a, E> =
    Strategy<Serializer<AlignedVec, ArenaHandle<'a>, Share>, E>;
