use core::{alloc::Layout, fmt, ops::DerefMut, ptr::NonNull};

use rancor::{fail, Error};

use crate::ser::Allocator;

#[derive(Debug)]
enum BufferAllocError {
    OutOfSpace(Layout),
    NotPoppedInReverseOrder {
        pos: usize,
        popped_pos: usize,
        popped_size: usize,
    },
    DoesNotContainAllocation,
}

impl fmt::Display for BufferAllocError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutOfSpace(layout) => write!(
                f,
                "not enough space to allocate request of size {} and align {}",
                layout.size(),
                layout.align()
            ),
            Self::NotPoppedInReverseOrder {
                pos,
                popped_pos,
                popped_size,
            } => write!(
                f,
                "allocation popped at {} with length {} runs past buffer allocator start {}",
                popped_pos, popped_size, pos,
            ),
            Self::DoesNotContainAllocation => {
                write!(f, "allocator does not contain popped allocation")
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for BufferAllocError {}

/// An allocator that allocates within a buffer.
///
/// Pairing this fixed-size allocator with a fallback allocator can help prevent
/// running out of space unexpectedly.
#[derive(Debug, Default)]
pub struct BufferAllocator<T> {
    buffer: T,
    pos: usize,
    // TODO: This used to store a pointer which was a nightmare. Did removing
    // that cause a perf regression?
}

impl<T> BufferAllocator<T> {
    /// Creates a new buffer allocator.
    pub fn new(buffer: T) -> Self {
        Self { buffer, pos: 0 }
    }

    /// Resets the allocator to its initial state.
    pub fn clear(&mut self) {
        self.pos = 0;
    }

    /// Consumes the buffer allocator, returning the underlying buffer.
    pub fn into_inner(self) -> T {
        self.buffer
    }
}

impl<T: DerefMut, E> Allocator<E> for BufferAllocator<T>
where
    T::Target: AsMut<[u8]>,
    E: Error,
{
    #[inline]
    unsafe fn push_alloc(
        &mut self,
        layout: Layout,
    ) -> Result<NonNull<[u8]>, E> {
        let bytes = self.buffer.as_mut();

        let pos = bytes.as_ptr() as usize + self.pos;
        let pad = 0usize.wrapping_sub(pos) % layout.align();
        if pad + layout.size() <= bytes.len() - self.pos {
            self.pos += pad;
            let result_slice = ptr_meta::from_raw_parts_mut(
                bytes.as_mut_ptr().add(self.pos).cast(),
                layout.size(),
            );
            let result = NonNull::new_unchecked(result_slice);
            self.pos += layout.size();
            Ok(result)
        } else {
            fail!(BufferAllocError::OutOfSpace(layout));
        }
    }

    #[inline]
    unsafe fn pop_alloc(
        &mut self,
        ptr: NonNull<u8>,
        layout: Layout,
    ) -> Result<(), E> {
        let bytes = self.buffer.as_mut();
        let ptr = ptr.as_ptr();

        if bytes.as_mut_ptr_range().contains(&ptr) {
            let popped_pos = ptr.offset_from(bytes.as_mut_ptr()) as usize;
            if popped_pos + layout.size() <= self.pos {
                self.pos = popped_pos;
                Ok(())
            } else {
                fail!(BufferAllocError::NotPoppedInReverseOrder {
                    pos: self.pos,
                    popped_pos,
                    popped_size: layout.size(),
                });
            }
        } else {
            fail!(BufferAllocError::DoesNotContainAllocation);
        }
    }
}
