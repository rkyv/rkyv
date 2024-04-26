#[cfg(not(feature = "std"))]
use alloc::{
    alloc::{alloc, alloc_zeroed, dealloc},
    boxed::Box,
    vec::Vec,
};
use core::{alloc::Layout, fmt, ptr::NonNull};
#[cfg(feature = "std")]
use std::alloc::{alloc, alloc_zeroed, dealloc};

use rancor::{fail, Source};

use crate::{
    fmt::Pointer,
    ser::{allocator::BufferAllocator, Allocator},
    util::AlignedBytes,
};

/// Fixed-size scratch space allocated on the heap.
#[derive(Debug, Default)]
pub struct BumpAllocator<const N: usize> {
    inner: BufferAllocator<Box<AlignedBytes<N>>>,
}

impl<const N: usize> BumpAllocator<N> {
    /// Creates a new heap scratch space.
    pub fn new() -> Self {
        if N != 0 {
            unsafe {
                let layout = Layout::new::<AlignedBytes<N>>();
                let ptr = alloc_zeroed(layout).cast::<AlignedBytes<N>>();
                assert!(!ptr.is_null());
                let buf = Box::from_raw(ptr);
                Self {
                    inner: BufferAllocator::new(buf),
                }
            }
        } else {
            Self {
                inner: BufferAllocator::new(Box::default()),
            }
        }
    }

    /// Gets the memory layout of the heap-allocated space.
    pub fn layout() -> Layout {
        unsafe { Layout::from_size_align_unchecked(N, 1) }
    }
}

impl<const N: usize, E: Source> Allocator<E> for BumpAllocator<N> {
    #[inline]
    unsafe fn push_alloc(
        &mut self,
        layout: Layout,
    ) -> Result<NonNull<[u8]>, E> {
        // SAFETY: The safety requirements for `inner.push_alloc()` are the same
        // as the safety conditions for `push_alloc()`.
        unsafe { self.inner.push_alloc(layout) }
    }

    #[inline]
    unsafe fn pop_alloc(
        &mut self,
        ptr: NonNull<u8>,
        layout: Layout,
    ) -> Result<(), E> {
        // SAFETY: The safety requirements for `inner.pop_alloc()` are the same
        // as the safety conditions for `pop_alloc()`.
        unsafe { self.inner.pop_alloc(ptr, layout) }
    }
}

#[derive(Debug)]
struct ExceededLimit {
    requested: usize,
    remaining: usize,
}

impl fmt::Display for ExceededLimit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "exceeded the maximum limit of scratch space: requested {}, \
             remaining {}",
            self.requested, self.remaining
        )
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ExceededLimit {}

#[derive(Debug)]
struct NotPoppedInReverseOrder {
    expected: usize,
    expected_layout: Layout,
    actual: usize,
    actual_layout: Layout,
}

impl fmt::Display for NotPoppedInReverseOrder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "scratch space was not popped in reverse order: expected {} with \
             size {} and align {}, found {} with size {} and align {}",
            Pointer(self.expected),
            self.expected_layout.size(),
            self.expected_layout.align(),
            Pointer(self.actual),
            self.actual_layout.size(),
            self.actual_layout.align(),
        )
    }
}

#[cfg(feature = "std")]
impl std::error::Error for NotPoppedInReverseOrder {}

#[derive(Debug)]
struct NoAllocationsToPop;

impl fmt::Display for NoAllocationsToPop {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "attempted to pop scratch space but there were no allocations to \
             pop"
        )
    }
}

#[cfg(feature = "std")]
impl std::error::Error for NoAllocationsToPop {}

/// Scratch space that always uses the global allocator.
///
/// This allocator will panic if scratch is popped that it did not allocate. For
/// this reason, it should only ever be used as a fallback allocator.
#[derive(Debug, Default)]
pub struct GlobalAllocator {
    remaining: Option<usize>,
    allocations: Vec<(*mut u8, Layout)>,
}

// SAFETY: AllocScratch is safe to send to another thread
// This trait is not automatically implemented because the struct contains a
// pointer
unsafe impl Send for GlobalAllocator {}

// SAFETY: AllocScratch is safe to share between threads
// This trait is not automatically implemented because the struct contains a
// pointer
unsafe impl Sync for GlobalAllocator {}

impl GlobalAllocator {
    /// Creates a new scratch allocator with no allocation limit.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new scratch allocator with the given allocation limit.
    pub fn with_limit(limit: usize) -> Self {
        Self {
            remaining: Some(limit),
            allocations: Vec::new(),
        }
    }
}

impl Drop for GlobalAllocator {
    fn drop(&mut self) {
        for (ptr, layout) in self.allocations.drain(..).rev() {
            unsafe {
                dealloc(ptr, layout);
            }
        }
    }
}

impl<E: Source> Allocator<E> for GlobalAllocator {
    #[inline]
    unsafe fn push_alloc(
        &mut self,
        layout: Layout,
    ) -> Result<NonNull<[u8]>, E> {
        if let Some(remaining) = self.remaining {
            if remaining < layout.size() {
                fail!(ExceededLimit {
                    requested: layout.size(),
                    remaining,
                });
            }
        }
        // SAFETY: The caller has guaranteed that `layout` has non-zero size.
        let result_ptr = unsafe { alloc(layout) };
        assert!(!result_ptr.is_null());
        self.allocations.push((result_ptr, layout));
        let result_slice =
            ptr_meta::from_raw_parts_mut(result_ptr.cast(), layout.size());
        // SAFETY: We asserted that `result_ptr` was not null.
        let result = unsafe { NonNull::new_unchecked(result_slice) };
        Ok(result)
    }

    #[inline]
    unsafe fn pop_alloc(
        &mut self,
        ptr: NonNull<u8>,
        layout: Layout,
    ) -> Result<(), E> {
        if let Some(&(last_ptr, last_layout)) = self.allocations.last() {
            if ptr.as_ptr() == last_ptr && layout == last_layout {
                // SAFETY: `ptr` and `alloc` correspond to a valid call to
                // `alloc` because they were on the allocation stack.
                unsafe { dealloc(ptr.as_ptr(), layout) };
                self.allocations.pop();
                Ok(())
            } else {
                fail!(NotPoppedInReverseOrder {
                    expected: last_ptr as usize,
                    expected_layout: last_layout,
                    actual: ptr.as_ptr() as usize,
                    actual_layout: layout,
                });
            }
        } else {
            fail!(NoAllocationsToPop);
        }
    }
}
