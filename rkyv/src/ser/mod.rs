//! Serialization traits and adapters.

pub mod allocator;
pub mod sharing;
pub mod writer;

#[doc(inline)]
pub use self::{
    allocator::Allocator,
    sharing::{Sharing, SharingExt},
    writer::{Positional, Writer, WriterExt},
};

use crate::{
    ser::{
        allocator::BufferAllocator, sharing::Duplicate, writer::BufferWriter,
    },
    util::AlignedBytes,
};
#[cfg(feature = "alloc")]
use crate::{
    ser::{
        allocator::{BackupAllocator, BumpAllocator, GlobalAllocator},
        sharing::Unify,
    },
    util::AlignedVec,
};
use ::core::{alloc::Layout, ptr::NonNull};

/// A serializer built from composeable pieces.
#[derive(Debug, Default)]
pub struct Composite<W = (), A = (), S = ()> {
    /// The writer of the `Composite` serializer.
    pub writer: W,
    /// The allocator of the `Composite` serializer.
    pub allocator: A,
    /// The shared pointer strategy of the `Composite` serializer.
    pub share: S,
}

impl<W, A, S> Composite<W, A, S> {
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

impl<W: Positional, A, S> Positional for Composite<W, A, S> {
    #[inline]
    fn pos(&self) -> usize {
        self.writer.pos()
    }
}

impl<W: Writer<E>, A, S, E> Writer<E> for Composite<W, A, S> {
    #[inline]
    fn write(&mut self, bytes: &[u8]) -> Result<(), E> {
        self.writer.write(bytes)
    }
}

impl<W, A: Allocator<E>, S, E> Allocator<E> for Composite<W, A, S> {
    #[inline]
    unsafe fn push_alloc(
        &mut self,
        layout: Layout,
    ) -> Result<NonNull<[u8]>, E> {
        self.allocator.push_alloc(layout)
    }

    #[inline]
    unsafe fn pop_alloc(
        &mut self,
        ptr: NonNull<u8>,
        layout: Layout,
    ) -> Result<(), E> {
        self.allocator.pop_alloc(ptr, layout)
    }
}

impl<W, A, S: Sharing<E>, E> Sharing<E> for Composite<W, A, S> {
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
///
/// `CoreSerializer` takes two arguments: the amount of serialization memory to
/// allocate and the amount of scratch space to allocate. If you run out of
/// either while serializing, the serializer will return an error.
pub type CoreSerializer<const W: usize, const A: usize> = Composite<
    BufferWriter<AlignedBytes<W>>,
    BufferAllocator<AlignedBytes<A>>,
    Duplicate,
>;

/// A general-purpose serializer suitable for environments where allocations can
/// be made.
///
/// `AllocSerializer` takes one argument: the amount of scratch space to
/// allocate before spilling allocations over into heap memory. A large amount
/// of scratch space may result in some of it not being used, but too little
/// scratch space will result in many allocations and decreased performance. You
/// should consider your use case carefully when determining how much scratch
/// space to pre-allocate.
#[cfg(feature = "alloc")]
pub type AllocSerializer<const A: usize> = Composite<
    AlignedVec,
    BackupAllocator<BumpAllocator<A>, GlobalAllocator>,
    Unify,
>;
