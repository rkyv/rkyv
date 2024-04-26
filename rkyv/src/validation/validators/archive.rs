//! The provided implementation for `ArchiveContext`.

use core::{alloc::Layout, fmt, num::NonZeroUsize, ops::Range};

use rancor::{fail, OptionExt, Source};

use crate::{fmt::Pointer, validation::ArchiveContext};

#[derive(Debug)]
struct UnalignedPointer {
    address: usize,
    align: usize,
}

impl fmt::Display for UnalignedPointer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "unaligned pointer: ptr {} unaligned for alignment {}",
            Pointer(self.address),
            self.align,
        )
    }
}

#[cfg(feature = "std")]
impl std::error::Error for UnalignedPointer {}

#[derive(Debug)]
struct InvalidSubtreePointer {
    address: usize,
    size: usize,
    subtree_range: Range<usize>,
}

impl fmt::Display for InvalidSubtreePointer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "subtree pointer overran range: ptr {} size {} in range {}..{}",
            Pointer(self.address),
            self.size,
            Pointer(self.subtree_range.start),
            Pointer(self.subtree_range.end),
        )
    }
}

#[cfg(feature = "std")]
impl std::error::Error for InvalidSubtreePointer {}

#[derive(Debug)]
struct ExceededMaximumSubtreeDepth;

impl fmt::Display for ExceededMaximumSubtreeDepth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "pushed a subtree range that exceeded the maximum subtree depth",
        )
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ExceededMaximumSubtreeDepth {}

#[derive(Debug)]
struct RangePoppedTooManyTimes;

impl fmt::Display for RangePoppedTooManyTimes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "subtree range popped too many times")
    }
}

#[cfg(feature = "std")]
impl std::error::Error for RangePoppedTooManyTimes {}

#[derive(Debug)]
struct RangePoppedOutOfOrder;

impl fmt::Display for RangePoppedOutOfOrder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "subtree range popped out of order")
    }
}

#[cfg(feature = "std")]
impl std::error::Error for RangePoppedOutOfOrder {}

/// A validator that can verify archives with nonlocal memory.
#[derive(Debug)]
pub struct ArchiveValidator {
    subtree_range: Range<usize>,
    max_subtree_depth: Option<NonZeroUsize>,
}

// SAFETY: `ArchiveValidator` is safe to send between threads because the
// pointers it contains are only ever used to compare addresses, never to
// dereference.
unsafe impl Send for ArchiveValidator {}

// SAFETY: `ArchiveValidator` is safe to share between threads because the
// pointers it contains are only ever used to compare addresses, never to
// dereference.
unsafe impl Sync for ArchiveValidator {}

impl ArchiveValidator {
    /// Creates a new bounds validator for the given bytes.
    #[inline]
    pub fn new(bytes: &[u8]) -> Self {
        Self::with_max_depth(bytes, None)
    }

    /// Crates a new bounds validator for the given bytes with a maximum
    /// validation depth.
    #[inline]
    pub fn with_max_depth(
        bytes: &[u8],
        max_subtree_depth: Option<NonZeroUsize>,
    ) -> Self {
        let Range { start, end } = bytes.as_ptr_range();
        Self {
            subtree_range: Range {
                start: start as usize,
                end: end as usize,
            },
            max_subtree_depth,
        }
    }
}

unsafe impl<E: Source> ArchiveContext<E> for ArchiveValidator {
    #[inline]
    fn check_subtree_ptr(
        &mut self,
        ptr: *const u8,
        layout: &Layout,
    ) -> Result<(), E> {
        let start = ptr as usize;
        let end = ptr.wrapping_add(layout.size()) as usize;
        if start < self.subtree_range.start || end > self.subtree_range.end {
            fail!(InvalidSubtreePointer {
                address: start,
                size: layout.size(),
                subtree_range: self.subtree_range.clone(),
            });
        } else if start & (layout.align() - 1) != 0 {
            fail!(UnalignedPointer {
                address: ptr as usize,
                align: layout.align(),
            });
        } else {
            Ok(())
        }
    }

    #[inline]
    unsafe fn push_subtree_range(
        &mut self,
        root: *const u8,
        end: *const u8,
    ) -> Result<Range<usize>, E> {
        if let Some(max_subtree_depth) = &mut self.max_subtree_depth {
            *max_subtree_depth = NonZeroUsize::new(max_subtree_depth.get() - 1)
                .into_trace(ExceededMaximumSubtreeDepth)?;
        }

        let result = Range {
            start: end as usize,
            end: self.subtree_range.end,
        };
        self.subtree_range.end = root as usize;
        Ok(result)
    }

    #[inline]
    unsafe fn pop_subtree_range(
        &mut self,
        range: Range<usize>,
    ) -> Result<(), E> {
        if range.start < self.subtree_range.end {
            fail!(RangePoppedOutOfOrder);
        }
        self.subtree_range = range;
        if let Some(max_subtree_depth) = &mut self.max_subtree_depth {
            *max_subtree_depth = max_subtree_depth
                .checked_add(1)
                .into_trace(RangePoppedTooManyTimes)?;
        }
        Ok(())
    }
}
