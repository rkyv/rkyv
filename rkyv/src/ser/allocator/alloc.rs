#[cfg(not(feature = "std"))]
use alloc::{
    alloc::{alloc, alloc_zeroed, dealloc},
    boxed::Box,
    vec::Vec,
};
use core::{
    alloc::Layout,
    marker::PhantomData,
    mem::{align_of, size_of, ManuallyDrop},
    ptr::{slice_from_raw_parts_mut, NonNull},
};
use std::alloc::handle_alloc_error;
#[cfg(feature = "std")]
use std::alloc::{alloc, dealloc};

use crate::ser::Allocator;

struct Block {
    next_ptr: NonNull<Block>,
    next_size: usize,
}

impl Block {
    fn alloc(size: usize) -> NonNull<Self> {
        debug_assert!(size >= size_of::<Self>());
        let layout = Layout::from_size_align(size, align_of::<Self>()).unwrap();
        let ptr = unsafe { alloc(layout).cast::<Self>() };
        let Some(ptr) = NonNull::new(ptr) else {
            handle_alloc_error(layout)
        };

        unsafe {
            ptr.as_ptr().write(Self {
                next_ptr: ptr,
                next_size: layout.size(),
            });
        }

        ptr
    }

    unsafe fn dealloc(ptr: NonNull<Self>, size: usize) {
        let layout = unsafe {
            Layout::from_size_align(size, align_of::<Self>()).unwrap_unchecked()
        };
        unsafe {
            dealloc(ptr.as_ptr().cast(), layout);
        }
    }

    /// # Safety
    ///
    /// `tail_ptr` and `new_ptr` must point to valid `Block`s and `new_ptr` must
    /// be the only block in its loop.
    unsafe fn push_next(
        mut tail_ptr: NonNull<Self>,
        mut new_ptr: NonNull<Self>,
    ) {
        let tail = unsafe { tail_ptr.as_mut() };
        let new = unsafe { new_ptr.as_mut() };

        debug_assert!(new.next_ptr == new_ptr);

        let head = tail.next_ptr;
        let head_cap = tail.next_size;
        tail.next_ptr = new_ptr;
        tail.next_size = new.next_size;
        new.next_ptr = head;
        new.next_size = head_cap;
    }
}

/// An arena allocator for allocations.
///
/// Reusing the same arena for multiple serializations will reduce the number of
/// global allocations, which can save a considerable amount of time.
pub struct Arena {
    head_ptr: NonNull<Block>,
}

impl Drop for Arena {
    fn drop(&mut self) {
        self.shrink();
        let head_size = unsafe { self.head_ptr.as_ref().next_size };
        unsafe {
            Block::dealloc(self.head_ptr, head_size);
        }
    }
}

impl Arena {
    /// The default capacity for arenas.
    pub const DEFAULT_CAPACITY: usize = 1024;

    /// Creates a new `Arena` with the default capacity.
    pub fn new() -> Self {
        Self::with_capacity(Self::DEFAULT_CAPACITY)
    }

    /// Creates a new `Arena` with at least the requested capacity.
    pub fn with_capacity(cap: usize) -> Self {
        let head_size = (cap + size_of::<Block>()).next_power_of_two();
        let head_ptr = Block::alloc(head_size);
        Self { head_ptr }
    }

    /// Cleans up allocated blocks which are no longer in use.
    ///
    /// The arena is automatically shrunk by [`acquire`](Self::acquire).
    pub fn shrink(&mut self) -> usize {
        let (mut current_ptr, mut current_size) = {
            let head = unsafe { self.head_ptr.as_ref() };
            (head.next_ptr, head.next_size)
        };

        loop {
            let current = unsafe { current_ptr.as_mut() };

            if current.next_ptr == current_ptr {
                // There was only one block in the loop. No deallocating needed.
                break;
            }

            let next_ptr = current.next_ptr;
            let next_size = current.next_size;

            if next_ptr == self.head_ptr {
                // End of the loop. Free the head block.
                unsafe {
                    Block::dealloc(next_ptr, next_size);
                }

                // Loop the head back on itself.
                current.next_ptr = current_ptr;
                current.next_size = current_size;
                self.head_ptr = current_ptr;

                break;
            }

            unsafe {
                Block::dealloc(current_ptr, current_size);
            }

            current_ptr = next_ptr;
            current_size = next_size;
        }

        current_size - size_of::<Block>()
    }

    /// Returns the available capacity of the arena.
    pub fn capacity(&self) -> usize {
        let mut current_ptr = self.head_ptr;
        loop {
            let current = unsafe { current_ptr.as_ref() };
            if current.next_ptr == self.head_ptr {
                break current.next_size - size_of::<Block>();
            }
            current_ptr = current.next_ptr;
        }
    }

    /// Acquires a handle to the arena.
    ///
    /// The returned handle has exclusive allocation rights in the arena.
    pub fn acquire(&mut self) -> ArenaHandle<'_> {
        self.shrink();

        ArenaHandle {
            tail_ptr: self.head_ptr,
            tail_size: unsafe { self.head_ptr.as_ref().next_size },
            used: size_of::<Block>(),
            _phantom: PhantomData,
        }
    }

    /// Consumes the `Arena`, returning a raw pointer.
    pub fn into_raw(self) -> NonNull<()> {
        let this = ManuallyDrop::new(self);
        this.head_ptr.cast()
    }

    /// Constructs an arena from a raw pointer.
    ///
    /// # Safety
    ///
    /// `raw` must have been returned from `into_raw`. `from_raw` takes
    /// ownership over the pointer, and so `from_raw` must not be called on the
    /// same pointer more than once.
    pub unsafe fn from_raw(raw: NonNull<()>) -> Self {
        Self {
            head_ptr: raw.cast(),
        }
    }
}

impl Default for Arena {
    fn default() -> Self {
        Self::new()
    }
}

/// A handle which can allocate within an arena.
pub struct ArenaHandle<'a> {
    tail_ptr: NonNull<Block>,
    tail_size: usize,
    used: usize,
    _phantom: PhantomData<&'a mut Arena>,
}

unsafe impl<E> Allocator<E> for ArenaHandle<'_> {
    unsafe fn push_alloc(
        &mut self,
        layout: Layout,
    ) -> Result<NonNull<[u8]>, E> {
        let pos = self.tail_ptr.as_ptr() as usize + self.used;
        let pad = 0usize.wrapping_sub(pos) % layout.align();
        if pad + layout.size() <= self.tail_size - self.used {
            self.used += pad;
        } else {
            // Allocation request is too large, allocate a new block
            let size = usize::max(
                2 * self.tail_size,
                (size_of::<Block>() + layout.size() + layout.align())
                    .next_power_of_two(),
            );
            let next = Block::alloc(size);
            unsafe {
                Block::push_next(self.tail_ptr, next);
            }
            self.tail_ptr = next;
            self.tail_size = size;
            let pos = self.tail_ptr.as_ptr() as usize + size_of::<Block>();
            let pad = 0usize.wrapping_sub(pos) % layout.align();
            self.used = size_of::<Block>() + pad;
        }

        // SAFETY: `self.used` is always less than the length of the allocated
        // block that `tail_ptr` points to.
        let ptr = unsafe { self.tail_ptr.as_ptr().cast::<u8>().add(self.used) };
        let slice_ptr = slice_from_raw_parts_mut(ptr, layout.size());
        // SAFETY: `slice_ptr` is guaranteed not to be null because it is offset
        // from `self.tail_ptr` which is always non-null.
        let result = unsafe { NonNull::new_unchecked(slice_ptr) };
        self.used += layout.size();
        Ok(result)
    }

    unsafe fn pop_alloc(
        &mut self,
        ptr: NonNull<u8>,
        _: Layout,
    ) -> Result<(), E> {
        let bytes = self.tail_ptr.as_ptr().cast::<u8>();
        self.used = ptr.as_ptr() as usize - bytes as usize;

        Ok(())
    }
}
