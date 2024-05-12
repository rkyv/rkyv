use core::{
    alloc::Layout,
    fmt,
    marker::PhantomData,
    mem::MaybeUninit,
    ptr::{slice_from_raw_parts_mut, NonNull},
};

use rancor::{fail, Source};

use crate::ser::Allocator;

#[derive(Debug)]
struct OutOfSpaceError {
    layout: Layout,
}

impl fmt::Display for OutOfSpaceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "not enough space to allocate request of size {} and align {}",
            self.layout.size(),
            self.layout.align()
        )
    }
}

#[cfg(feature = "std")]
impl std::error::Error for OutOfSpaceError {}

/// An allocator that sub-allocates a fixed-size memory space.
#[derive(Debug)]
pub struct SubAllocator<'a> {
    bytes: NonNull<u8>,
    used: usize,
    size: usize,
    _phantom: PhantomData<&'a mut [MaybeUninit<u8>]>,
}

impl<'a> SubAllocator<'a> {
    /// Creates an empty suballocator.
    pub fn empty() -> Self {
        Self {
            bytes: NonNull::dangling(),
            used: 0,
            size: 0,
            _phantom: PhantomData,
        }
    }

    /// Creates a new sub-allocator from the given byte slice.
    pub fn new(bytes: &'a mut [MaybeUninit<u8>]) -> Self {
        Self {
            bytes: unsafe { NonNull::new_unchecked(bytes.as_mut_ptr().cast()) },
            used: 0,
            size: bytes.len(),
            _phantom: PhantomData,
        }
    }
}

unsafe impl<E> Allocator<E> for SubAllocator<'_>
where
    E: Source,
{
    #[inline]
    unsafe fn push_alloc(
        &mut self,
        layout: Layout,
    ) -> Result<NonNull<[u8]>, E> {
        let pos = self.bytes.as_ptr() as usize + self.used;
        let pad = 0usize.wrapping_sub(pos) % layout.align();
        if pad + layout.size() <= self.size - self.used {
            self.used += pad;
        } else {
            fail!(OutOfSpaceError { layout });
        }

        // SAFETY: `self.used` is always less than the length of the allocated
        // block that `self.bytes` points to.
        let ptr = unsafe { self.bytes.as_ptr().add(self.used) };
        let slice_ptr = slice_from_raw_parts_mut(ptr, layout.size());
        // SAFETY: `slice_ptr` is guaranteed not to be null because it is
        // offset from `self.bytes` which is always non-null.
        let result = unsafe { NonNull::new_unchecked(slice_ptr) };
        self.used += layout.size();
        Ok(result)
    }

    #[inline]
    unsafe fn pop_alloc(
        &mut self,
        ptr: NonNull<u8>,
        _: Layout,
    ) -> Result<(), E> {
        let bytes = self.bytes.as_ptr();
        self.used = ptr.as_ptr() as usize - bytes as usize;

        Ok(())
    }
}
