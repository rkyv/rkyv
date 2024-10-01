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
///
/// # Safety
///
/// `push_alloc` must return a pointer to unaliased memory which fits the
/// provided layout.
pub unsafe trait Allocator<E = <Self as Fallible>::Error> {
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
    /// - The allocations pushed on top of the given allocation must not be
    ///   popped after calling `pop_alloc`.
    /// - `layout` must be the same layout that was used to allocate the block
    ///   of memory for the given pointer.
    unsafe fn pop_alloc(
        &mut self,
        ptr: NonNull<u8>,
        layout: Layout,
    ) -> Result<(), E>;
}

unsafe impl<T: Allocator<E>, E> Allocator<E> for Strategy<T, E> {
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

/// Statistics for the allocations which occurred during serialization.
#[derive(Debug)]
pub struct AllocationStats {
    bytes_allocated: usize,
    allocations: usize,
    /// Returns the maximum number of bytes that were concurrently allocated.
    pub max_bytes_allocated: usize,
    /// Returns the maximum number of concurrent allocations.
    pub max_allocations: usize,
    /// Returns the maximum alignment of requested allocations.
    pub max_alignment: usize,
}

impl AllocationStats {
    /// Returns the minimum arena capacity required to serialize the same data.
    ///
    /// This calculation takes into account packing efficiency for slab
    /// allocated space. It is not exact, and has an error bound of
    /// `max_allocations * (max_alignment - 1)` bytes. This should be suitably
    /// small for most use cases.
    #[inline]
    pub fn min_arena_capacity(&self) -> usize {
        self.max_bytes_allocated + self.min_arena_capacity_max_error()
    }

    /// Returns the maximum error term for the minimum arena capacity
    /// calculation.
    #[inline]
    pub fn min_arena_capacity_max_error(&self) -> usize {
        self.max_allocations * (self.max_alignment - 1)
    }
}

impl AllocationStats {
    #[inline]
    fn push(&mut self, layout: Layout) {
        self.bytes_allocated += layout.size();
        self.allocations += 1;
        self.max_bytes_allocated =
            usize::max(self.bytes_allocated, self.max_bytes_allocated);
        self.max_allocations =
            usize::max(self.allocations, self.max_allocations);
        self.max_alignment = usize::max(self.max_alignment, layout.align());
    }

    #[inline]
    fn pop(&mut self, layout: Layout) {
        self.bytes_allocated -= layout.size();
        self.allocations -= 1;
    }
}

/// A passthrough allocator that tracks usage.
pub struct AllocationTracker<T> {
    inner: T,
    stats: AllocationStats,
}

impl<T> AllocationTracker<T> {
    /// Returns a new allocation tracker wrapping the given allocator.
    pub fn new(inner: T) -> Self {
        Self {
            inner,
            stats: AllocationStats {
                bytes_allocated: 0,
                allocations: 0,
                max_bytes_allocated: 0,
                max_allocations: 0,
                max_alignment: 1,
            },
        }
    }

    /// Returns the allocation stats accumulated during serialization.
    pub fn into_stats(self) -> AllocationStats {
        self.stats
    }
}

unsafe impl<T: Allocator<E>, E> Allocator<E> for AllocationTracker<T> {
    unsafe fn push_alloc(
        &mut self,
        layout: Layout,
    ) -> Result<NonNull<[u8]>, E> {
        self.stats.push(layout);
        // SAFETY: The safety requirements for `push_alloc` are the same as the
        // requirements for `inner.push_alloc`.
        unsafe { self.inner.push_alloc(layout) }
    }

    unsafe fn pop_alloc(
        &mut self,
        ptr: NonNull<u8>,
        layout: Layout,
    ) -> Result<(), E> {
        self.stats.pop(layout);
        // SAFETY: The safety requirements for `pop_alloc` are the same as the
        // requirements for `inner.pop_alloc`.
        unsafe { self.inner.pop_alloc(ptr, layout) }
    }
}

impl<T> From<T> for AllocationTracker<T> {
    fn from(inner: T) -> Self {
        Self::new(inner)
    }
}

#[cfg(test)]
mod tests {
    use core::mem::MaybeUninit;

    use rancor::{Panic, Strategy};

    use crate::{
        api::serialize_using,
        ser::{
            allocator::{AllocationStats, AllocationTracker, SubAllocator},
            sharing::Unshare,
            writer::Buffer,
            Serializer,
        },
        util::Align,
        Serialize,
    };

    type TrackerSerializer<'a> = Strategy<
        Serializer<Buffer<'a>, AllocationTracker<SubAllocator<'a>>, Unshare>,
        Panic,
    >;

    fn track_serialize<T>(value: &T) -> AllocationStats
    where
        T: for<'a> Serialize<TrackerSerializer<'a>>,
    {
        let mut output = Align([MaybeUninit::<u8>::uninit(); 256]);
        let mut scratch = [MaybeUninit::<u8>::uninit(); 256];

        let mut serializer = Serializer::new(
            Buffer::from(&mut *output),
            AllocationTracker::new(SubAllocator::new(&mut scratch)),
            Unshare,
        );
        serialize_using(value, &mut serializer).unwrap();
        serializer.into_raw_parts().1.into_stats()
    }

    #[test]
    fn simple() {
        let stats = track_serialize(&42);
        assert_eq!(stats.max_bytes_allocated, 0);
        assert_eq!(stats.max_allocations, 0);
        assert_eq!(stats.max_alignment, 1);
        assert_eq!(stats.min_arena_capacity(), 0);
        assert_eq!(stats.min_arena_capacity_max_error(), 0);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn nested() {
        use crate::alloc::vec;

        let stats = track_serialize(&vec![1, 2, 3, 4]);
        assert_eq!(stats.max_bytes_allocated, 0);
        assert_eq!(stats.max_allocations, 0);
        assert_eq!(stats.max_alignment, 1);
        assert_eq!(stats.min_arena_capacity(), 0);
        assert_eq!(stats.min_arena_capacity_max_error(), 0);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn doubly_nested() {
        use crate::alloc::vec;

        let stats = track_serialize(&vec![vec![1, 2], vec![3, 4]]);
        assert_ne!(stats.max_bytes_allocated, 0);
        assert_eq!(stats.max_allocations, 1);
        assert_ne!(stats.min_arena_capacity(), 0);
    }
}
