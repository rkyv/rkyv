//! Validation implementations and helper types.

use crate::{Archive, Archived, RelPtr};
use bytecheck::{CheckBytes, Unreachable};
use core::{fmt, mem};
use std::error;

/// A range of bytes in an archive.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Interval {
    start: usize,
    end: usize,
}

impl Interval {
    fn new(start: usize, len: usize) -> Self {
        Self {
            start,
            end: start + len,
        }
    }

    fn overlaps(&self, other: &Self) -> bool {
        self.start < other.end && other.start < self.end
    }
}

/// Errors that can occur related to archive memory.
#[derive(Debug)]
pub enum ArchiveMemoryError {
    /// A pointer pointed outside the bounds of the archive
    OutOfBounds {
        /// The position of the relative pointer
        base: usize,
        /// The offset of the relative pointer
        offset: isize,
        /// The length of the archive
        archive_len: usize,
    },
    /// There wasn't enough space for the desired type at the pointed location
    Overrun {
        /// The position of the type
        pos: usize,
        /// The desired size of the type
        size: usize,
        /// The length of the archive
        archive_len: usize,
    },
    /// The pointer wasn't aligned properly for the desired type
    Unaligned {
        /// The position of the type
        pos: usize,
        /// The required alignment of the type
        align: usize,
    },
    /// Multiple objects claim to own the same memory region
    ClaimOverlap {
        /// A previous interval of bytes claimed by some object
        previous: Interval,
        /// The current interval of bytes being claimed by some object
        current: Interval,
    },
}

impl fmt::Display for ArchiveMemoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArchiveMemoryError::OutOfBounds {
                base,
                offset,
                archive_len,
            } => write!(
                f,
                "out of bounds pointer: base {} offset {} in archive len {}",
                base, offset, archive_len
            ),
            ArchiveMemoryError::Overrun {
                pos,
                size,
                archive_len,
            } => write!(
                f,
                "archive overrun: pos {} size {} in archive len {}",
                pos, size, archive_len
            ),
            ArchiveMemoryError::Unaligned { pos, align } => write!(
                f,
                "unaligned pointer: pos {} unaligned for alignment {}",
                pos, align
            ),
            ArchiveMemoryError::ClaimOverlap { previous, current } => write!(
                f,
                "memory claim overlap: current [{}..{}] overlaps previous [{}..{}]",
                current.start, current.end, previous.start, previous.end
            ),
        }
    }
}

impl error::Error for ArchiveMemoryError {}

/// Errors that can occur when checking an archive.
#[derive(Debug)]
pub enum CheckArchiveError<T> {
    /// A memory error
    MemoryError(ArchiveMemoryError),
    /// An error that occurred while validating an object
    CheckBytes(T),
}

impl<T> From<ArchiveMemoryError> for CheckArchiveError<T> {
    fn from(e: ArchiveMemoryError) -> Self {
        Self::MemoryError(e)
    }
}

impl<T: fmt::Display> fmt::Display for CheckArchiveError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CheckArchiveError::MemoryError(e) => write!(f, "archive memory error: {}", e),
            CheckArchiveError::CheckBytes(e) => write!(f, "check bytes error: {}", e),
        }
    }
}

impl<T: fmt::Debug + fmt::Display> error::Error for CheckArchiveError<T> {}

/// Context to perform archive validation.
pub struct ArchiveContext {
    begin: *const u8,
    len: usize,
    intervals: Vec<Interval>,
}

impl ArchiveContext {
    /// Creates a new archive context for the given byte slice
    pub fn new(bytes: &[u8]) -> Self {
        Self {
            begin: bytes.as_ptr(),
            len: bytes.len(),
            intervals: Vec::new(),
        }
    }

    /// Checks the relative pointer with given `base` and `offset`, then claims
    /// `count` items at the target location.
    ///
    /// # Safety
    ///
    /// `base` must be inside the archive this context was created for.
    pub unsafe fn claim_memory<T: CheckBytes<ArchiveContext>>(
        &mut self,
        base: *const u8,
        offset: isize,
        count: usize,
    ) -> Result<*const u8, ArchiveMemoryError> {
        let base_pos = base.offset_from(self.begin);
        if offset < -base_pos || offset > self.len as isize - base_pos {
            Err(ArchiveMemoryError::OutOfBounds {
                base: base_pos as usize,
                offset,
                archive_len: self.len,
            })
        } else {
            let target_pos = (base_pos + offset) as usize;
            let size = count * mem::size_of::<T>();
            if self.len - target_pos < size {
                Err(ArchiveMemoryError::Overrun {
                    pos: target_pos,
                    size,
                    archive_len: self.len,
                })
            } else {
                let align = mem::align_of::<T>();
                if target_pos & (align - 1) != 0 {
                    Err(ArchiveMemoryError::Unaligned {
                        pos: target_pos,
                        align,
                    })
                } else {
                    let interval = Interval::new(target_pos, size);
                    match self.intervals.binary_search(&interval) {
                        Ok(index) => Err(ArchiveMemoryError::ClaimOverlap {
                            previous: self.intervals[index],
                            current: interval,
                        }),
                        Err(index) => {
                            if index < self.intervals.len()
                                && self.intervals[index].overlaps(&interval)
                            {
                                Err(ArchiveMemoryError::ClaimOverlap {
                                    previous: self.intervals[index],
                                    current: interval,
                                })
                            } else if index > 0 && self.intervals[index - 1].overlaps(&interval) {
                                Err(ArchiveMemoryError::ClaimOverlap {
                                    previous: self.intervals[index - 1],
                                    current: interval,
                                })
                            } else {
                                self.intervals.insert(index, interval);
                                Ok(base.offset(offset))
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Checks the given archive at the given position for an archived version of
/// the given type.
pub fn check_archive<'a, T: Archive>(
    buf: &[u8],
    pos: usize,
) -> Result<&'a Archived<T>, CheckArchiveError<<Archived<T> as CheckBytes<ArchiveContext>>::Error>>
where
    T::Archived: CheckBytes<ArchiveContext>,
{
    let mut context = ArchiveContext::new(buf);
    unsafe {
        let bytes = context.claim_memory::<Archived<T>>(buf.as_ptr(), pos as isize, 1)?;
        Archived::<T>::check_bytes(bytes, &mut context).map_err(CheckArchiveError::CheckBytes)?;
        Ok(&*bytes.cast())
    }
}

impl CheckBytes<ArchiveContext> for RelPtr {
    type Error = Unreachable;

    unsafe fn check_bytes<'a>(
        bytes: *const u8,
        context: &mut ArchiveContext,
    ) -> Result<&'a Self, Self::Error> {
        i32::check_bytes(bytes, context)?;
        Ok(&*bytes.cast())
    }
}
