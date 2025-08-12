use core::{
    alloc::Layout, error::Error, fmt, marker::PhantomData, num::NonZeroUsize,
    ops::Range,
};

use rancor::{fail, OptionExt, Source};

use crate::validation::ArchiveContext;

const PTR_WIDTH: usize = (usize::BITS / 4 + 2) as usize;

struct Pointer(pub usize);

impl fmt::Display for Pointer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:#0w$x}", self.0, w = PTR_WIDTH)
    }
}

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

impl Error for UnalignedPointer {}

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

impl Error for InvalidSubtreePointer {}

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

impl Error for ExceededMaximumSubtreeDepth {}

#[derive(Debug)]
struct RangePoppedTooManyTimes;

impl fmt::Display for RangePoppedTooManyTimes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "subtree range popped too many times")
    }
}

impl Error for RangePoppedTooManyTimes {}

#[derive(Debug)]
struct RangePoppedOutOfOrder;

impl fmt::Display for RangePoppedOutOfOrder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "subtree range popped out of order")
    }
}

impl Error for RangePoppedOutOfOrder {}

/// A validator that can verify archives with nonlocal memory.
#[derive(Debug)]
pub struct ArchiveValidator<'a> {
    subtree_range: Range<usize>,
    max_subtree_depth: Option<NonZeroUsize>,
    _phantom: PhantomData<&'a [u8]>,
}

impl<'a> ArchiveValidator<'a> {
    /// Creates a new bounds validator for the given bytes.
    #[inline]
    pub fn new(bytes: &'a [u8]) -> Self {
        Self::with_max_depth(bytes, None)
    }

    /// Crates a new bounds validator for the given bytes with a maximum
    /// validation depth.
    #[inline]
    pub fn with_max_depth(
        bytes: &'a [u8],
        max_subtree_depth: Option<NonZeroUsize>,
    ) -> Self {
        let Range { start, end } = bytes.as_ptr_range();
        Self {
            subtree_range: Range {
                start: start as usize,
                end: end as usize,
            },
            max_subtree_depth,
            _phantom: PhantomData,
        }
    }
}

unsafe impl<E: Source> ArchiveContext<E> for ArchiveValidator<'_> {
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
