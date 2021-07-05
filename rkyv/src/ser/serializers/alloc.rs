use crate::{
    ser::{
        serializers::BufferScratch,
        ScratchSpace,
        Serializer,
        SharedSerializeRegistry,
    },
    AlignedBytes,
    AlignedVec,
    Archive,
    ArchiveUnsized,
    Fallible,
    RelPtr,
};
use core::{
    alloc::Layout,
    borrow::{Borrow, BorrowMut},
    convert::Infallible,
    fmt,
    mem,
};
#[cfg(not(feature = "std"))]
use ::alloc::alloc;
#[cfg(feature = "std")]
use ::std::alloc;
#[cfg(all(feature = "alloc", not(feature = "std")))]
use hashbrown::hash_map;
#[cfg(feature = "std")]
use std::collections::hash_map;

/// A serializer made specifically to work with [`AlignedVec`](crate::util::AlignedVec).
///
/// This serializer makes it easier for the compiler to perform emplacement optimizations and may
/// give better performance than a basic `WriteSerializer`.
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

impl<A> Fallible for AlignedSerializer<A> {
    type Error = Infallible;
}

impl<A: Borrow<AlignedVec> + BorrowMut<AlignedVec>> Serializer for AlignedSerializer<A> {
    #[inline]
    fn pos(&self) -> usize {
        self.inner.borrow().len()
    }

    #[inline]
    fn write(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
        self.inner.borrow_mut().extend_from_slice(bytes);
        Ok(())
    }

    #[inline]
    unsafe fn resolve_aligned<T: Archive + ?Sized>(
        &mut self,
        value: &T,
        resolver: T::Resolver,
    ) -> Result<usize, Self::Error> {
        let pos = self.pos();
        debug_assert_eq!(pos & (mem::align_of::<T::Archived>() - 1), 0);
        let vec = self.inner.borrow_mut();
        let additional = mem::size_of::<T::Archived>();
        vec.reserve(additional);
        vec.set_len(vec.len() + additional);

        let ptr = vec
            .as_mut_ptr()
            .add(pos)
            .cast::<T::Archived>();
        ptr.write_bytes(0, 1);
        value.resolve(pos, resolver, ptr);

        Ok(pos)
    }

    #[inline]
    unsafe fn resolve_unsized_aligned<T: ArchiveUnsized + ?Sized>(
        &mut self,
        value: &T,
        to: usize,
        metadata_resolver: T::MetadataResolver,
    ) -> Result<usize, Self::Error> {
        let from = self.pos();
        debug_assert_eq!(from & (mem::align_of::<RelPtr<T::Archived>>() - 1), 0);
        let vec = self.inner.borrow_mut();
        let additional = mem::size_of::<RelPtr<T::Archived>>();
        vec.reserve(additional);
        vec.set_len(vec.len() + additional);

        let ptr = vec
            .as_mut_ptr()
            .add(from)
            .cast::<RelPtr<T::Archived>>();
        ptr.write_bytes(0, 1);

        value.resolve_unsized(from, to, metadata_resolver, ptr);
        Ok(from)
    }
}

/// Fixed-size scratch space allocated on the heap.
pub struct HeapScratch<const N: usize> {
    inner: BufferScratch<Box<AlignedBytes<N>>>,
}

impl<const N: usize> HeapScratch<N> {
    /// Creates a new heap scratch space.
    pub fn new() -> Self {
        unsafe {
            let layout = Layout::new::<AlignedBytes<N>>();
            let ptr = alloc::alloc(layout).cast();
            let buf = Box::from_raw(ptr);
            Self {
                inner: BufferScratch::new(buf),
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

impl<const N: usize> Fallible for HeapScratch<N> {
    type Error = <BufferScratch<Box<[u8]>> as Fallible>::Error;
}

impl<const N: usize> ScratchSpace for HeapScratch<N> {
    #[inline]
    unsafe fn push_scratch(&mut self, layout: Layout) -> Result<*mut u8, Self::Error> {
        self.inner.push_scratch(layout)
    }

    #[inline]
    unsafe fn pop_scratch(&mut self, ptr: *mut u8, layout: Layout) -> Result<(), Self::Error> {
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
        /// The maximum amount of scratch space available
        maximum: usize,
    }
}

impl fmt::Display for AllocScratchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ExceededLimit { requested, maximum } => write!(
                f,
                "exceeded the maxmium limit of scratch space: requested {}, maximum {}",
                requested, maximum
            )
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
pub struct AllocScratch {
    allocated: usize,
    limit: Option<usize>,
}

impl AllocScratch {
    /// Creates a new scratch allocator with no allocation limit.
    pub fn new() -> Self {
        Self {
            allocated: 0,
            limit: None,
        }
    }

    /// Creates a new scratch allocator with the given allocation limit.
    pub fn with_limit(limit: usize) -> Self {
        Self {
            allocated: 0,
            limit: Some(limit),
        }
    }
}

impl Default for AllocScratch {
    fn default() -> Self {
        Self::new()
    }
}

impl Fallible for AllocScratch {
    type Error = AllocScratchError;
}

impl ScratchSpace for AllocScratch {
    #[inline]
    unsafe fn push_scratch(&mut self, layout: Layout) -> Result<*mut u8, Self::Error> {
        if let Some(limit) = self.limit {
            if limit - self.allocated < layout.size() {
                return Err(AllocScratchError::ExceededLimit {
                    requested: layout.size(),
                    maximum: limit,
                })
            }
        }
        Ok(alloc::alloc(layout))
    }

    #[inline]
    unsafe fn pop_scratch(&mut self, ptr: *mut u8, layout: Layout) -> Result<(), Self::Error> {
        alloc::dealloc(ptr, layout);
        Ok(())
    }
}

/// An error that can occur while serializing shared pointers.
#[derive(Debug)]
pub enum SharedSerializeMapError {
    /// A shared pointer was added multiple times
    DuplicateSharedPointer(*const u8),
}

impl fmt::Display for SharedSerializeMapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateSharedPointer(p) => write!(f, "duplicate shared pointer: {:p}", p),
        }
    }
}

#[cfg(feature = "std")]
const _: () = {
    use std::error::Error;

    impl Error for SharedSerializeMapError {}
};

/// An adapter that adds shared serialization support to a serializer.
pub struct SharedSerializeMap {
    shared_resolvers: hash_map::HashMap<*const u8, usize>,
}

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

impl Fallible for SharedSerializeMap {
    type Error = SharedSerializeMapError;
}

impl SharedSerializeRegistry for SharedSerializeMap {
    fn get_shared_ptr(&mut self, value: *const u8) -> Option<usize> {
        self.shared_resolvers.get(&value).copied()
    }

    fn add_shared_ptr(&mut self, value: *const u8, pos: usize) -> Result<(), Self::Error> {
        match self.shared_resolvers.entry(value) {
            hash_map::Entry::Occupied(_) => Err(SharedSerializeMapError::DuplicateSharedPointer(value)),
            hash_map::Entry::Vacant(e) => {
                e.insert(pos);
                Ok(())
            }
        }
    }
}
