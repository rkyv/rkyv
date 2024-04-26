//! Allocators for serializers to use during serialization.

#[cfg(feature = "alloc")]
mod alloc;
mod core;

use ::core::{alloc::Layout, ptr::NonNull};
use rancor::{Fallible, Strategy};

#[cfg(feature = "alloc")]
pub use self::alloc::*;
pub use self::core::*;

/// A serializer that can allocate scratch space.
pub trait Allocator<E = <Self as Fallible>::Error> {
    /// Allocates scratch space of the requested size.
    ///
    /// # Safety
    ///
    /// `layout` must have non-zero size.
    unsafe fn push_alloc(&mut self, layout: Layout)
        -> Result<NonNull<[u8]>, E>;

    /// Deallocates previously allocated scratch space.
    ///
    /// # Safety
    ///
    /// - `ptr` must be the scratch memory last allocated with `push_scratch`.
    /// - `layout` must be the same layout that was used to allocate that block
    ///   of memory.
    unsafe fn pop_alloc(
        &mut self,
        ptr: NonNull<u8>,
        layout: Layout,
    ) -> Result<(), E>;
}

impl<T: Allocator<E>, E> Allocator<E> for Strategy<T, E> {
    unsafe fn push_alloc(
        &mut self,
        layout: Layout,
    ) -> Result<NonNull<[u8]>, E> {
        // SAFETY: The safety requirements for `push_alloc()` are the same as
        // the requirements for `T::push_alloc`.
        unsafe { T::push_alloc(self, layout) }
    }

    unsafe fn pop_alloc(
        &mut self,
        ptr: NonNull<u8>,
        layout: Layout,
    ) -> Result<(), E> {
        // SAFETY: The safety requirements for `pop_alloc()` are the same as
        // the requirements for `T::pop_alloc`.
        unsafe { T::pop_alloc(self, ptr, layout) }
    }
}

// TODO: This whole thing is dubiously sound because it "tries" to deallocate
// and relies on that failing to try the "correct" thing to do.

/// Allocates space with a primary and backup allocator.
#[derive(Debug, Default)]
pub struct BackupAllocator<P, B> {
    primary: P,
    backup: B,
}

impl<P, B> BackupAllocator<P, B> {
    /// Creates a backup allocator from primary and backup allocators.
    pub fn new(primary: P, backup: B) -> Self {
        Self { primary, backup }
    }
}

impl<P, B, E> Allocator<E> for BackupAllocator<P, B>
where
    P: Allocator<E>,
    B: Allocator<E>,
{
    #[inline]
    unsafe fn push_alloc(
        &mut self,
        layout: Layout,
    ) -> Result<NonNull<[u8]>, E> {
        // dubious soundness
        unsafe {
            self.primary
                .push_alloc(layout)
                .or_else(|_| self.backup.push_alloc(layout))
        }
    }

    #[inline]
    unsafe fn pop_alloc(
        &mut self,
        ptr: NonNull<u8>,
        layout: Layout,
    ) -> Result<(), E> {
        // dubious soundness
        unsafe {
            self.primary
                .pop_alloc(ptr, layout)
                .or_else(|_| self.backup.pop_alloc(ptr, layout))
        }
    }
}

/// A passthrough allocator that tracks usage.
#[derive(Debug, Default)]
pub struct AllocationTracker<T> {
    inner: T,
    bytes_allocated: usize,
    allocations: usize,
    max_bytes_allocated: usize,
    max_allocations: usize,
    max_alignment: usize,
}

impl<T> AllocationTracker<T> {
    /// Returns a new allocation tracker wrapping the given allocator.
    pub fn new(inner: T) -> Self {
        Self {
            inner,
            bytes_allocated: 0,
            allocations: 0,
            max_bytes_allocated: 0,
            max_allocations: 0,
            max_alignment: 1,
        }
    }

    /// Returns the maximum number of bytes that were concurrently allocated.
    pub fn max_bytes_allocated(&self) -> usize {
        self.max_bytes_allocated
    }

    /// Returns the maximum number of concurrent allocations.
    pub fn max_allocations(&self) -> usize {
        self.max_allocations
    }

    /// Returns the maximum alignment of requested allocations.
    pub fn max_alignment(&self) -> usize {
        self.max_alignment
    }

    /// Returns the minimum buffer size required to serialize the same data.
    ///
    /// This calculation takes into account packing efficiency for slab
    /// allocated space. It is not exact, and has an error bound of
    /// `max_allocations * (max_alignment - 1)` bytes. This should be suitably
    /// small for most use cases.
    pub fn min_buffer_size(&self) -> usize {
        self.max_bytes_allocated + self.min_buffer_size_max_error()
    }

    /// Returns the maximum error term for the minimum buffer size calculation.
    pub fn min_buffer_size_max_error(&self) -> usize {
        self.max_allocations * (self.max_alignment - 1)
    }
}

impl<T: Allocator<E>, E> Allocator<E> for AllocationTracker<T> {
    #[inline]
    unsafe fn push_alloc(
        &mut self,
        layout: Layout,
    ) -> Result<NonNull<[u8]>, E> {
        // SAFETY: The safety requirements for `push_alloc` are the same as the
        // requirements for `inner.push_alloc`.
        let result = unsafe { self.inner.push_alloc(layout)? };

        self.bytes_allocated += layout.size();
        self.allocations += 1;
        self.max_bytes_allocated =
            usize::max(self.bytes_allocated, self.max_bytes_allocated);
        self.max_allocations =
            usize::max(self.allocations, self.max_allocations);
        self.max_alignment = usize::max(self.max_alignment, layout.align());

        Ok(result)
    }

    #[inline]
    unsafe fn pop_alloc(
        &mut self,
        ptr: NonNull<u8>,
        layout: Layout,
    ) -> Result<(), E> {
        // SAFETY: The safety requirements for `pop_alloc` are the same as the
        // requirements for `inner.pop_alloc`.
        unsafe {
            self.inner.pop_alloc(ptr, layout)?;
        }

        self.bytes_allocated -= layout.size();
        self.allocations -= 1;

        Ok(())
    }
}

impl<T> From<T> for AllocationTracker<T> {
    fn from(inner: T) -> Self {
        Self::new(inner)
    }
}
