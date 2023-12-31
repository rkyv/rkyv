use crate::{
    ser::{
        serializers::BufferScratch, ScratchSpace, Serializer,
        SharedSerializeRegistry,
    },
    util::{AlignedBytes, AlignedVec},
};
#[cfg(not(feature = "std"))]
use alloc::{alloc, boxed::Box, vec::Vec};
use core::{
    alloc::Layout,
    borrow::{Borrow, BorrowMut},
    fmt,
    ptr::NonNull,
};
#[cfg(not(feature = "std"))]
use hashbrown::hash_map;
use rancor::{fail, Error};
#[cfg(feature = "std")]
use std::alloc;
#[cfg(feature = "std")]
use std::collections::hash_map;

/// A serializer made specifically to work with [`AlignedVec`](crate::util::AlignedVec).
///
/// This serializer makes it easier for the compiler to perform emplacement optimizations and may
/// give better performance than a basic `WriteSerializer`.
#[derive(Debug)]
pub struct AlignedSerializer<A> {
    inner: A,
}

impl<A: Borrow<AlignedVec>> AlignedSerializer<A> {
    /// Creates a new `AlignedSerializer` by wrapping a `Borrow<AlignedVec>`.
    #[inline]
    pub fn new(inner: A) -> Self {
        Self { inner }
    }

    /// Consumes the serializer and returns the underlying type.
    #[inline]
    pub fn into_inner(self) -> A {
        self.inner
    }
}

impl<A: Default> Default for AlignedSerializer<A> {
    #[inline]
    fn default() -> Self {
        Self {
            inner: A::default(),
        }
    }
}

impl<A: Borrow<AlignedVec> + BorrowMut<AlignedVec>, E> Serializer<E>
    for AlignedSerializer<A>
{
    #[inline]
    fn pos(&self) -> usize {
        self.inner.borrow().len()
    }

    #[inline]
    fn write(&mut self, bytes: &[u8]) -> Result<(), E> {
        self.inner.borrow_mut().extend_from_slice(bytes);
        Ok(())
    }

    // TODO: check whether moving this into an extension trait resulted in a
    // benchmark regression from additional memory copying.

    // #[inline]
    // unsafe fn resolve_aligned<T: Archive + ?Sized>(
    //     &mut self,
    //     value: &T,
    //     resolver: T::Resolver,
    // ) -> Result<usize, E> {
    //     let pos = Serializer::<E>::pos(self);
    //     debug_assert_eq!(pos & (mem::align_of::<T::Archived>() - 1), 0);
    //     let vec = self.inner.borrow_mut();
    //     let additional = mem::size_of::<T::Archived>();
    //     vec.reserve(additional);
    //     vec.set_len(vec.len() + additional);

    //     let ptr = vec.as_mut_ptr().add(pos).cast::<T::Archived>();
    //     ptr.write_bytes(0, 1);
    //     value.resolve(pos, resolver, ptr);

    //     Ok(pos)
    // }

    // #[inline]
    // unsafe fn resolve_unsized_aligned<T: ArchiveUnsized + ?Sized>(
    //     &mut self,
    //     value: &T,
    //     to: usize,
    //     metadata_resolver: T::MetadataResolver,
    // ) -> Result<usize, E> {
    //     let from = Serializer::<E>::pos(self);
    //     debug_assert_eq!(
    //         from & (mem::align_of::<RelPtr<T::Archived>>() - 1),
    //         0
    //     );
    //     let vec = self.inner.borrow_mut();
    //     let additional = mem::size_of::<RelPtr<T::Archived>>();
    //     vec.reserve(additional);
    //     vec.set_len(vec.len() + additional);

    //     let ptr = vec.as_mut_ptr().add(from).cast::<RelPtr<T::Archived>>();
    //     ptr.write_bytes(0, 1);

    //     value.resolve_unsized(from, to, metadata_resolver, ptr);
    //     Ok(from)
    // }
}

/// Fixed-size scratch space allocated on the heap.
#[derive(Debug)]
pub struct HeapScratch<const N: usize> {
    inner: BufferScratch<Box<AlignedBytes<N>>>,
}

impl<const N: usize> HeapScratch<N> {
    /// Creates a new heap scratch space.
    pub fn new() -> Self {
        if N != 0 {
            unsafe {
                let layout = Layout::new::<AlignedBytes<N>>();
                let ptr = alloc::alloc_zeroed(layout).cast::<AlignedBytes<N>>();
                assert!(!ptr.is_null());
                let buf = Box::from_raw(ptr);
                Self {
                    inner: BufferScratch::new(buf),
                }
            }
        } else {
            Self {
                inner: BufferScratch::new(Box::default()),
            }
        }
    }

    /// Gets the memory layout of the heap-allocated space.
    pub fn layout() -> Layout {
        unsafe { Layout::from_size_align_unchecked(N, 1) }
    }
}

impl<const N: usize> Default for HeapScratch<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize, E: Error> ScratchSpace<E> for HeapScratch<N> {
    #[inline]
    unsafe fn push_scratch(
        &mut self,
        layout: Layout,
    ) -> Result<NonNull<[u8]>, E> {
        self.inner.push_scratch(layout)
    }

    #[inline]
    unsafe fn pop_scratch(
        &mut self,
        ptr: NonNull<u8>,
        layout: Layout,
    ) -> Result<(), E> {
        self.inner.pop_scratch(ptr, layout)
    }
}

/// Errors that can occur when allocating with the global allocator.
#[derive(Debug)]
pub enum AllocScratchError {
    /// The amount of scratch space requested exceeded the maximum limit
    ExceededLimit {
        /// The amount of scratch space requested
        requested: usize,
        /// The amount of scratch space remaining
        remaining: usize,
    },
    /// Scratch space was not popped in reverse order.
    NotPoppedInReverseOrder {
        /// The pointer of the allocation that was expected to be next
        expected: *mut u8,
        /// The layout of the allocation that was expected to be next
        expected_layout: Layout,
        /// The pointer that was popped instead
        actual: *mut u8,
        /// The layout of the pointer that was popped instead
        actual_layout: Layout,
    },
    /// There are no allocations to pop
    NoAllocationsToPop,
}

// SAFETY: AllocScratchError is safe to send to another thread
// This trait is not automatically implemented because the enum contains a pointer
unsafe impl Send for AllocScratchError {}

// SAFETY: AllocScratchError is safe to share between threads
// This trait is not automatically implemented because the enum contains a pointer
unsafe impl Sync for AllocScratchError {}

impl fmt::Display for AllocScratchError {
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

    impl Error for AllocScratchError {}
};

/// Scratch space that always uses the global allocator.
///
/// This allocator will panic if scratch is popped that it did not allocate. For this reason, it
/// should only ever be used as a fallback allocator.
#[derive(Debug)]
pub struct AllocScratch {
    remaining: Option<usize>,
    allocations: Vec<(*mut u8, Layout)>,
}

// SAFETY: AllocScratch is safe to send to another thread
// This trait is not automatically implemented because the struct contains a pointer
unsafe impl Send for AllocScratch {}

// SAFETY: AllocScratch is safe to share between threads
// This trait is not automatically implemented because the struct contains a pointer
unsafe impl Sync for AllocScratch {}

impl AllocScratch {
    /// Creates a new scratch allocator with no allocation limit.
    pub fn new() -> Self {
        Self {
            remaining: None,
            allocations: Vec::new(),
        }
    }

    /// Creates a new scratch allocator with the given allocation limit.
    pub fn with_limit(limit: usize) -> Self {
        Self {
            remaining: Some(limit),
            allocations: Vec::new(),
        }
    }
}

impl Drop for AllocScratch {
    fn drop(&mut self) {
        for (ptr, layout) in self.allocations.drain(..).rev() {
            unsafe {
                alloc::dealloc(ptr, layout);
            }
        }
    }
}

impl Default for AllocScratch {
    fn default() -> Self {
        Self::new()
    }
}

impl<E: Error> ScratchSpace<E> for AllocScratch {
    #[inline]
    unsafe fn push_scratch(
        &mut self,
        layout: Layout,
    ) -> Result<NonNull<[u8]>, E> {
        if let Some(remaining) = self.remaining {
            if remaining < layout.size() {
                fail!(AllocScratchError::ExceededLimit {
                    requested: layout.size(),
                    remaining,
                });
            }
        }
        let result_ptr = alloc::alloc(layout);
        assert!(!result_ptr.is_null());
        self.allocations.push((result_ptr, layout));
        let result_slice =
            ptr_meta::from_raw_parts_mut(result_ptr.cast(), layout.size());
        let result = NonNull::new_unchecked(result_slice);
        Ok(result)
    }

    #[inline]
    unsafe fn pop_scratch(
        &mut self,
        ptr: NonNull<u8>,
        layout: Layout,
    ) -> Result<(), E> {
        if let Some(&(last_ptr, last_layout)) = self.allocations.last() {
            if ptr.as_ptr() == last_ptr && layout == last_layout {
                alloc::dealloc(ptr.as_ptr(), layout);
                self.allocations.pop();
                Ok(())
            } else {
                fail!(AllocScratchError::NotPoppedInReverseOrder {
                    expected: last_ptr,
                    expected_layout: last_layout,
                    actual: ptr.as_ptr(),
                    actual_layout: layout,
                });
            }
        } else {
            fail!(AllocScratchError::NoAllocationsToPop);
        }
    }
}

/// An error that can occur while serializing shared pointers.
#[derive(Debug)]
pub enum SharedSerializeMapError {
    /// A shared pointer was added multiple times
    DuplicateSharedPointer(*const u8),
}

// SAFETY: SharedSerializeMapError is safe to send to another thread
// This trait is not automatically implemented because the enum contains a pointer
unsafe impl Send for SharedSerializeMapError {}

// SAFETY: SharedSerializeMapError is safe to share between threads
// This trait is not automatically implemented because the enum contains a pointer
unsafe impl Sync for SharedSerializeMapError {}

impl fmt::Display for SharedSerializeMapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateSharedPointer(p) => {
                write!(f, "duplicate shared pointer: {:p}", p)
            }
        }
    }
}

#[cfg(feature = "std")]
const _: () = {
    use std::error::Error;

    impl Error for SharedSerializeMapError {}
};

/// An adapter that adds shared serialization support to a serializer.
#[derive(Debug)]
pub struct SharedSerializeMap {
    shared_resolvers: hash_map::HashMap<*const u8, usize>,
}

// SAFETY: SharedSerializeMap is safe to send to another thread
// This trait is not automatically implemented because the struct contains a pointer
unsafe impl Send for SharedSerializeMap {}

// SAFETY: SharedSerializeMap is safe to share between threads
// This trait is not automatically implemented because the struct contains a pointer
unsafe impl Sync for SharedSerializeMap {}

impl SharedSerializeMap {
    /// Creates a new shared registry map.
    #[inline]
    pub fn new() -> Self {
        Self {
            shared_resolvers: hash_map::HashMap::new(),
        }
    }
}

impl Default for SharedSerializeMap {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<E: Error> SharedSerializeRegistry<E> for SharedSerializeMap {
    fn get_shared_ptr(&self, value: *const u8) -> Option<usize> {
        self.shared_resolvers.get(&value).copied()
    }

    fn add_shared_ptr(
        &mut self,
        value: *const u8,
        pos: usize,
    ) -> Result<(), E> {
        match self.shared_resolvers.entry(value) {
            hash_map::Entry::Occupied(_) => {
                fail!(SharedSerializeMapError::DuplicateSharedPointer(value));
            }
            hash_map::Entry::Vacant(e) => {
                e.insert(pos);
                Ok(())
            }
        }
    }
}
