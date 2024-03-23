//! The provided implementation for `ArchiveContext`.

use core::{
    alloc::{Layout, LayoutError},
    fmt,
    num::NonZeroUsize,
    ops::Range,
};

use bytecheck::rancor::Error;
use rancor::{fail, OptionExt};

use crate::validation::ArchiveContext;

/// Errors that can occur when checking archive memory.
#[derive(Debug)]
pub enum ArchiveError {
    /// The pointer wasn't aligned properly for the desired type
    Unaligned {
        /// The pointer to the type
        address: usize,
        /// The required alignment of the type
        align: usize,
    },
    /// The pointer wasn't within the subtree range
    InvalidSubtreePointer {
        /// The address of the subtree pointer
        address: usize,
        /// The desired size of the subtree value
        size: usize,
        /// The subtree range
        subtree_range: Range<usize>,
    },
    /// A subtree range was popped too many times.
    RangePoppedTooManyTimes,
    /// The maximum subtree depth was reached or exceeded.
    ExceededMaximumSubtreeDepth,
    /// A layout error occurred
    LayoutError {
        /// A layout error
        layout_error: LayoutError,
    },
}

impl fmt::Display for ArchiveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        const W: usize = (usize::BITS / 4 + 2) as usize;

        match self {
            ArchiveError::Unaligned { address, align } => write!(
                f,
                "unaligned pointer: ptr {address:#0w$x} unaligned for \
                 alignment {align}",
                w = W,
            ),
            ArchiveError::InvalidSubtreePointer {
                address,
                size,
                subtree_range,
            } => write!(
                f,
                "subtree pointer overran range: ptr {address:#0w$x} size \
                 {size} in range {:#0w$x}..{:#0w$x}",
                subtree_range.start,
                subtree_range.end,
                w = W,
            ),
            ArchiveError::RangePoppedTooManyTimes => {
                write!(f, "subtree range popped too many times",)
            }
            ArchiveError::ExceededMaximumSubtreeDepth => write!(
                f,
                "pushed a subtree range that exceeded the maximum subtree \
                 depth",
            ),
            ArchiveError::LayoutError { layout_error } => {
                write!(f, "a layout error occurred: {}", layout_error)
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ArchiveError {}

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

unsafe impl<E: Error> ArchiveContext<E> for ArchiveValidator {
    #[inline]
    fn check_subtree_ptr(
        &mut self,
        ptr: *const u8,
        layout: &Layout,
    ) -> Result<(), E> {
        let start = ptr as usize;
        let end = ptr.wrapping_add(layout.size()) as usize;
        if start < self.subtree_range.start || end > self.subtree_range.end {
            fail!(ArchiveError::InvalidSubtreePointer {
                address: start,
                size: layout.size(),
                subtree_range: self.subtree_range.clone(),
            });
        } else if start & (layout.align() - 1) != 0 {
            fail!(ArchiveError::Unaligned {
                address: ptr as usize,
                align: layout.align(),
            });
        } else {
            Ok(())
        }
    }

    #[inline]
    unsafe fn push_prefix_subtree_range(
        &mut self,
        root: *const u8,
        end: *const u8,
    ) -> Result<Range<usize>, E> {
        if let Some(max_subtree_depth) = &mut self.max_subtree_depth {
            *max_subtree_depth = NonZeroUsize::new(max_subtree_depth.get() - 1)
                .into_trace(ArchiveError::ExceededMaximumSubtreeDepth)?;
        }

        let result = Range {
            start: end as usize,
            end: self.subtree_range.end,
        };
        self.subtree_range.end = root as usize;
        Ok(result)
    }

    #[inline]
    unsafe fn push_suffix_subtree_range(
        &mut self,
        start: *const u8,
        root: *const u8,
    ) -> Result<Range<usize>, E> {
        if let Some(max_subtree_depth) = &mut self.max_subtree_depth {
            *max_subtree_depth = NonZeroUsize::new(max_subtree_depth.get() - 1)
                .into_trace(ArchiveError::ExceededMaximumSubtreeDepth)?;
        }

        let result = Range {
            start: self.subtree_range.start,
            end: start as usize,
        };
        self.subtree_range.start = start as usize;
        self.subtree_range.end = root as usize;
        Ok(result)
    }

    #[inline]
    unsafe fn pop_subtree_range(
        &mut self,
        range: Range<usize>,
    ) -> Result<(), E> {
        self.subtree_range = range;
        if let Some(max_subtree_depth) = &mut self.max_subtree_depth {
            *max_subtree_depth = max_subtree_depth
                .checked_add(1)
                .into_trace(ArchiveError::RangePoppedTooManyTimes)?;
        }
        Ok(())
    }
}
