//! Validators add validation capabilities by wrapping and extending basic validators.

use crate::{validation::ArchiveBoundsContext, Fallible};
#[cfg(feature = "std")]
use crate::{
    validation::{
        check_archived_root_with_context, check_archived_value_with_context, ArchiveMemoryContext,
        CheckTypeError, SharedArchiveContext,
    },
    Archive,
};
#[cfg(feature = "std")]
use bytecheck::CheckBytes;
use core::{alloc::Layout, any::TypeId, fmt};
#[cfg(feature = "std")]
use std::{collections::HashMap, error::Error};

/// Errors that can occur when checking a relative pointer
#[derive(Debug)]
pub enum ArchiveBoundsError {
    /// The archive is under-aligned for one of the types inside
    Underaligned {
        /// The expected alignment of the archive
        expected_align: usize,
        /// The actual alignment of the archive
        actual_align: usize,
    },
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
}

impl fmt::Display for ArchiveBoundsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArchiveBoundsError::Underaligned {
                expected_align,
                actual_align,
            } => write!(
                f,
                "archive underaligned: need alignment {} but have alignment {}",
                expected_align, actual_align
            ),
            ArchiveBoundsError::OutOfBounds {
                base,
                offset,
                archive_len,
            } => write!(
                f,
                "out of bounds pointer: base {} offset {} in archive len {}",
                base, offset, archive_len
            ),
            ArchiveBoundsError::Overrun {
                pos,
                size,
                archive_len,
            } => write!(
                f,
                "archive overrun: pos {} size {} in archive len {}",
                pos, size, archive_len
            ),
            ArchiveBoundsError::Unaligned { pos, align } => write!(
                f,
                "unaligned pointer: pos {} unaligned for alignment {}",
                pos, align
            ),
        }
    }
}

#[cfg(feature = "std")]
impl Error for ArchiveBoundsError {}

/// A validator that can bounds check pointers in an archive.
pub struct ArchiveBoundsValidator<'a> {
    bytes: &'a [u8],
}

impl<'a> ArchiveBoundsValidator<'a> {
    /// Creates a new bounds validator for the given bytes.
    #[inline]
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes }
    }

    /// Gets a reference to the bytes being validated.
    #[inline]
    pub fn bytes(&self) -> &'a [u8] {
        self.bytes
    }

    /// Gets a pointer to the beginning of the validator's byte range.
    #[inline]
    pub fn begin(&self) -> *const u8 {
        self.bytes.as_ptr()
    }

    /// Gets the length of the validator's byte range.
    #[inline]
    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    /// Returns whether the byte range is empty.
    pub fn is_empty(&self) -> bool {
        self.bytes.len() == 0
    }
}

impl<'a> From<&'a [u8]> for ArchiveBoundsValidator<'a> {
    fn from(bytes: &'a [u8]) -> Self {
        Self::new(bytes)
    }
}

impl<'a> Fallible for ArchiveBoundsValidator<'a> {
    type Error = ArchiveBoundsError;
}

impl<'a> ArchiveBoundsContext for ArchiveBoundsValidator<'a> {
    unsafe fn check_rel_ptr(
        &mut self,
        base: *const u8,
        offset: isize,
    ) -> Result<*const u8, Self::Error> {
        let base_pos = base.offset_from(self.begin());
        if offset < -base_pos || offset > self.len() as isize - base_pos {
            Err(ArchiveBoundsError::OutOfBounds {
                base: base_pos as usize,
                offset,
                archive_len: self.len(),
            })
        } else {
            Ok(base.offset(offset))
        }
    }

    unsafe fn bounds_check_ptr(
        &mut self,
        ptr: *const u8,
        layout: &Layout,
    ) -> Result<(), Self::Error> {
        if (self.begin() as usize) & (layout.align() - 1) != 0 {
            Err(ArchiveBoundsError::Underaligned {
                expected_align: layout.align(),
                actual_align: 1 << (self.begin() as usize).trailing_zeros(),
            })
        } else {
            let target_pos = ptr.offset_from(self.begin()) as usize;
            if target_pos & (layout.align() - 1) != 0 {
                Err(ArchiveBoundsError::Unaligned {
                    pos: target_pos,
                    align: layout.align(),
                })
            } else if self.len() - target_pos < layout.size() {
                Err(ArchiveBoundsError::Overrun {
                    pos: target_pos,
                    size: layout.size(),
                    archive_len: self.len(),
                })
            } else {
                Ok(())
            }
        }
    }
}

/// A range of bytes in an archive.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Interval {
    /// The start of the byte range
    pub start: *const u8,
    /// The end of the byte range
    pub end: *const u8,
}

impl Interval {
    /// Returns whether the interval overlaps with another.
    #[inline]
    pub fn overlaps(&self, other: &Self) -> bool {
        self.start < other.end && other.start < self.end
    }
}

/// Errors that can occur related to archive memory.
#[derive(Debug)]
pub enum ArchiveMemoryError<E> {
    /// An error from the wrapped validator
    Inner(E),
    /// Multiple objects claim to own the same memory region
    ClaimOverlap {
        /// A previous interval of bytes claimed by some object
        previous: Interval,
        /// The current interval of bytes being claimed by some object
        current: Interval,
    },
}

impl<E: fmt::Display> fmt::Display for ArchiveMemoryError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArchiveMemoryError::Inner(e) => e.fmt(f),
            ArchiveMemoryError::ClaimOverlap { previous, current } => write!(
                f,
                "memory claim overlap: current [{:#?}..{:#?}] overlaps previous [{:#?}..{:#?}]",
                current.start, current.end, previous.start, previous.end
            ),
        }
    }
}

#[cfg(feature = "std")]
impl<E: Error + 'static> Error for ArchiveMemoryError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ArchiveMemoryError::Inner(e) => Some(e as &dyn Error),
            ArchiveMemoryError::ClaimOverlap { .. } => None,
        }
    }
}

#[cfg(feature = "std")]
/// An adapter that adds memory validation to a context.
pub struct ArchiveValidator<C> {
    inner: C,
    intervals: Vec<Interval>,
}

#[cfg(feature = "std")]
impl<C> ArchiveValidator<C> {
    /// Wraps the given validator context and adds memory validation.
    #[inline]
    pub fn new(inner: C) -> Self {
        Self {
            inner,
            intervals: Vec::new(),
        }
    }

    /// Consumes the adapter and returns the underlying validator.
    #[inline]
    pub fn into_inner(self) -> C {
        self.inner
    }
}

#[cfg(feature = "std")]
impl<'a, C: From<&'a [u8]>> From<&'a [u8]> for ArchiveValidator<C> {
    fn from(bytes: &'a [u8]) -> Self {
        Self::new(C::from(bytes))
    }
}

#[cfg(feature = "std")]
impl<C: Fallible> Fallible for ArchiveValidator<C> {
    type Error = ArchiveMemoryError<C::Error>;
}

#[cfg(feature = "std")]
impl<C: ArchiveBoundsContext> ArchiveBoundsContext for ArchiveValidator<C> {
    #[inline]
    unsafe fn check_rel_ptr(
        &mut self,
        base: *const u8,
        offset: isize,
    ) -> Result<*const u8, Self::Error> {
        self.inner
            .check_rel_ptr(base, offset)
            .map_err(ArchiveMemoryError::Inner)
    }

    #[inline]
    unsafe fn bounds_check_ptr(
        &mut self,
        ptr: *const u8,
        layout: &Layout,
    ) -> Result<(), Self::Error> {
        self.inner
            .bounds_check_ptr(ptr, layout)
            .map_err(ArchiveMemoryError::Inner)
    }
}

#[cfg(feature = "std")]
impl<C: ArchiveBoundsContext> ArchiveMemoryContext for ArchiveValidator<C> {
    unsafe fn claim_bytes(&mut self, start: *const u8, len: usize) -> Result<(), Self::Error> {
        let interval = Interval {
            start,
            end: start.add(len),
        };
        match self.intervals.binary_search(&interval) {
            Ok(index) => Err(ArchiveMemoryError::ClaimOverlap {
                previous: self.intervals[index],
                current: interval,
            }),
            Err(index) => {
                if index < self.intervals.len() {
                    if self.intervals[index].overlaps(&interval) {
                        return Err(ArchiveMemoryError::ClaimOverlap {
                            previous: self.intervals[index],
                            current: interval,
                        });
                    } else if self.intervals[index].start == interval.end {
                        self.intervals[index].start = interval.start;
                        return Ok(());
                    }
                }

                if index > 0 {
                    if self.intervals[index - 1].overlaps(&interval) {
                        return Err(ArchiveMemoryError::ClaimOverlap {
                            previous: self.intervals[index - 1],
                            current: interval,
                        });
                    } else if self.intervals[index - 1].end == interval.start {
                        self.intervals[index - 1].end = interval.end;
                        return Ok(());
                    }
                }

                self.intervals.insert(index, interval);
                Ok(())
            }
        }
    }
}

/// Errors that can occur when checking shared memory.
#[derive(Debug)]
pub enum SharedArchiveError<E> {
    /// An error occurred while checking the memory of the archive
    Inner(E),
    /// Multiple pointers exist to the same location with different types
    TypeMismatch {
        /// A previous type that the location was checked as
        previous: TypeId,
        /// The current type that the location is checked as
        current: TypeId,
    },
}

impl<E: fmt::Display> fmt::Display for SharedArchiveError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SharedArchiveError::Inner(e) => e.fmt(f),
            SharedArchiveError::TypeMismatch { previous, current } => write!(
                f,
                "the same memory region has been claimed as two different types ({:?} and {:?})",
                previous, current
            ),
        }
    }
}

#[cfg(feature = "std")]
impl<E: Error + 'static> Error for SharedArchiveError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            SharedArchiveError::Inner(e) => Some(e as &dyn Error),
            SharedArchiveError::TypeMismatch { .. } => None,
        }
    }
}

/// An adapter that adds shared memory validation.
#[cfg(feature = "std")]
pub struct SharedArchiveValidator<C> {
    inner: C,
    shared_blocks: HashMap<*const u8, TypeId>,
}

#[cfg(feature = "std")]
impl<C> SharedArchiveValidator<C> {
    /// Wraps the given context and adds shared memory validation.
    #[inline]
    pub fn new(inner: C) -> Self {
        Self {
            inner,
            shared_blocks: HashMap::new(),
        }
    }

    /// Consumes the adapter and returns the underlying validator.
    #[inline]
    pub fn into_inner(self) -> C {
        self.inner
    }
}

#[cfg(feature = "std")]
impl<'a, C: From<&'a [u8]>> From<&'a [u8]> for SharedArchiveValidator<C> {
    fn from(bytes: &'a [u8]) -> Self {
        Self::new(C::from(bytes))
    }
}

#[cfg(feature = "std")]
impl<C: Fallible> Fallible for SharedArchiveValidator<C> {
    type Error = SharedArchiveError<C::Error>;
}

#[cfg(feature = "std")]
impl<C: ArchiveBoundsContext> ArchiveBoundsContext for SharedArchiveValidator<C> {
    #[inline]
    unsafe fn check_rel_ptr(
        &mut self,
        base: *const u8,
        offset: isize,
    ) -> Result<*const u8, Self::Error> {
        self.inner
            .check_rel_ptr(base, offset)
            .map_err(SharedArchiveError::Inner)
    }

    #[inline]
    unsafe fn bounds_check_ptr(
        &mut self,
        ptr: *const u8,
        layout: &Layout,
    ) -> Result<(), Self::Error> {
        self.inner
            .bounds_check_ptr(ptr, layout)
            .map_err(SharedArchiveError::Inner)
    }
}

#[cfg(feature = "std")]
impl<C: ArchiveMemoryContext> ArchiveMemoryContext for SharedArchiveValidator<C> {
    #[inline]
    unsafe fn claim_bytes(&mut self, start: *const u8, len: usize) -> Result<(), Self::Error> {
        self.inner
            .claim_bytes(start, len)
            .map_err(SharedArchiveError::Inner)
    }
}

#[cfg(feature = "std")]
impl<C: ArchiveMemoryContext> SharedArchiveContext for SharedArchiveValidator<C> {
    unsafe fn claim_shared_bytes(
        &mut self,
        start: *const u8,
        len: usize,
        type_id: TypeId,
    ) -> Result<bool, Self::Error> {
        if let Some(previous_type_id) = self.shared_blocks.get(&start) {
            if previous_type_id != &type_id {
                Err(SharedArchiveError::TypeMismatch {
                    previous: *previous_type_id,
                    current: type_id,
                })
            } else {
                Ok(false)
            }
        } else {
            self.shared_blocks.insert(start, type_id);
            self.inner
                .claim_bytes(start, len)
                .map_err(SharedArchiveError::Inner)?;
            Ok(true)
        }
    }
}

/// A validator that supports all builtin types.
#[cfg(feature = "std")]
pub type DefaultArchiveValidator<'a> =
    SharedArchiveValidator<ArchiveValidator<ArchiveBoundsValidator<'a>>>;

/// Checks the given archive at the given position for an archived version of the given type.
///
/// This is a safe alternative to [`archived_value`](crate::archived_value) for types that implement
/// `CheckBytes`.
///
/// # Example
/// ```
/// use rkyv::{
///     check_archived_value,
///     ser::{Serializer, serializers::AlignedSerializer},
///     AlignedVec,
///     Archive,
///     Serialize,
/// };
/// use bytecheck::CheckBytes;
///
/// #[derive(Archive, Serialize)]
/// #[archive_attr(derive(CheckBytes))]
/// struct Example {
///     name: String,
///     value: i32,
/// }
///
/// let value = Example {
///     name: "pi".to_string(),
///     value: 31415926,
/// };
///
/// let mut serializer = AlignedSerializer::new(AlignedVec::new());
/// let pos = serializer.serialize_value(&value)
///     .expect("failed to archive test");
/// let buf = serializer.into_inner();
/// let archived = check_archived_value::<Example>(buf.as_ref(), pos).unwrap();
/// ```
#[cfg(feature = "std")]
#[inline]
pub fn check_archived_value<'a, T: Archive>(
    bytes: &'a [u8],
    pos: usize,
) -> Result<&T::Archived, CheckTypeError<T::Archived, DefaultArchiveValidator<'a>>>
where
    T::Archived: CheckBytes<DefaultArchiveValidator<'a>>,
{
    let mut validator = DefaultArchiveValidator::from(bytes);
    check_archived_value_with_context::<T, DefaultArchiveValidator>(bytes, pos, &mut validator)
}

/// Checks the given archive at the given position for an archived version of the given type.
///
/// This is a safe alternative to [`archived_value`](crate::archived_value) for types that implement
/// `CheckBytes`.
///
/// See [`check_archived_value`] for more details.
#[cfg(feature = "std")]
#[inline]
pub fn check_archived_root<'a, T: Archive>(
    bytes: &'a [u8],
) -> Result<&T::Archived, CheckTypeError<T::Archived, DefaultArchiveValidator<'a>>>
where
    T::Archived: CheckBytes<DefaultArchiveValidator<'a>>,
{
    let mut validator = DefaultArchiveValidator::from(bytes);
    check_archived_root_with_context::<T, DefaultArchiveValidator>(bytes, &mut validator)
}
