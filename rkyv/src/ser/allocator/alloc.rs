use core::{alloc::Layout, fmt, ptr::NonNull};

#[cfg(not(feature = "std"))]
use alloc::{
    alloc::{alloc, alloc_zeroed, dealloc},
    boxed::Box,
    vec::Vec,
};
use rancor::{fail, Error};
#[cfg(feature = "std")]
use std::alloc::{alloc, alloc_zeroed, dealloc};

use crate::{
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

impl<const N: usize, E: Error> Allocator<E> for BumpAllocator<N> {
    #[inline]
    unsafe fn push_alloc(
        &mut self,
        layout: Layout,
    ) -> Result<NonNull<[u8]>, E> {
        self.inner.push_alloc(layout)
    }

    #[inline]
    unsafe fn pop_alloc(
        &mut self,
        ptr: NonNull<u8>,
        layout: Layout,
    ) -> Result<(), E> {
        self.inner.pop_alloc(ptr, layout)
    }
}

#[derive(Debug)]
enum GlobalAllocatorError {
    ExceededLimit {
        requested: usize,
        remaining: usize,
    },
    NotPoppedInReverseOrder {
        expected: usize,
        expected_layout: Layout,
        actual: usize,
        actual_layout: Layout,
    },
    NoAllocationsToPop,
}

impl fmt::Display for GlobalAllocatorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ExceededLimit { requested, remaining } => write!(
                f,
                "exceeded the maximum limit of scratch space: requested {}, remaining {}",
                requested, remaining
            ),
            Self::NotPoppedInReverseOrder {
                expected,
                expected_layout,
                actual,
                actual_layout,
            } => write!(
                f,
                "scratch space was not popped in reverse order: expected {:p} with size {} and align {}, found {:p} with size {} and align {}",
                expected, expected_layout.size(), expected_layout.align(), actual, actual_layout.size(), actual_layout.align()
            ),
            Self::NoAllocationsToPop => write!(
                f, "attempted to pop scratch space but there were no allocations to pop"
            ),
        }
    }
}

#[cfg(feature = "std")]
const _: () = {
    use std::error::Error;

    impl Error for GlobalAllocatorError {}
};

/// Scratch space that always uses the global allocator.
///
/// This allocator will panic if scratch is popped that it did not allocate. For this reason, it
/// should only ever be used as a fallback allocator.
#[derive(Debug, Default)]
pub struct GlobalAllocator {
    remaining: Option<usize>,
    allocations: Vec<(*mut u8, Layout)>,
}

// SAFETY: AllocScratch is safe to send to another thread
// This trait is not automatically implemented because the struct contains a pointer
unsafe impl Send for GlobalAllocator {}

// SAFETY: AllocScratch is safe to share between threads
// This trait is not automatically implemented because the struct contains a pointer
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

impl<E: Error> Allocator<E> for GlobalAllocator {
    #[inline]
    unsafe fn push_alloc(
        &mut self,
        layout: Layout,
    ) -> Result<NonNull<[u8]>, E> {
        if let Some(remaining) = self.remaining {
            if remaining < layout.size() {
                fail!(GlobalAllocatorError::ExceededLimit {
                    requested: layout.size(),
                    remaining,
                });
            }
        }
        let result_ptr = alloc(layout);
        assert!(!result_ptr.is_null());
        self.allocations.push((result_ptr, layout));
        let result_slice =
            ptr_meta::from_raw_parts_mut(result_ptr.cast(), layout.size());
        let result = NonNull::new_unchecked(result_slice);
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
                dealloc(ptr.as_ptr(), layout);
                self.allocations.pop();
                Ok(())
            } else {
                fail!(GlobalAllocatorError::NotPoppedInReverseOrder {
                    expected: last_ptr as usize,
                    expected_layout: last_layout,
                    actual: ptr.as_ptr() as usize,
                    actual_layout: layout,
                });
            }
        } else {
            fail!(GlobalAllocatorError::NoAllocationsToPop);
        }
    }
}
