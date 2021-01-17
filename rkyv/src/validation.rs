//! Validation implementations and helper types.

use crate::{Archive, Archived, Offset, RelPtr};
use bytecheck::{CheckBytes, Unreachable};
use core::{fmt, mem};
use std::error;

#[derive(Debug)]
pub enum ArchiveBoundsError {
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

impl error::Error for ArchiveBoundsError {}

/// A context that can ensure that a 
pub trait ArchiveBoundsContext {
    unsafe fn check_rel_ptr(
        &mut self,
        base: *const u8,
        offset: isize,
        count: usize,
        align: usize,
    ) -> Result<*const u8, ArchiveBoundsError>;
}

/// An [`ArchiveContext`] that partially validates an archive.
///
/// It performs only bounds checking, in contrast with [`ArchiveValidator`].
pub struct ArchiveBoundsValidator {
    begin: *const u8,
    len: usize,
}

impl ArchiveBoundsValidator {
    /// Creates a new bounds validator for the given byte range.
    pub fn new(bytes: &[u8]) -> Self {
        Self {
            begin: bytes.as_ptr(),
            len: bytes.len(),
        }
    }

    /// Gets a pointer to the beginning of the validator's byte range.
    pub fn begin(&self) -> *const u8 {
        self.begin
    }

    /// Gets the length of the validator's byte range.
    pub fn len(&self) -> usize {
        self.len
    }
}

impl ArchiveBoundsContext for ArchiveBoundsValidator {
    unsafe fn check_rel_ptr(
        &mut self,
        base: *const u8,
        offset: isize,
        count: usize,
        align: usize,
    ) -> Result<*const u8, ArchiveBoundsError> {
        let base_pos = base.offset_from(self.begin);
        if offset < -base_pos || offset > self.len as isize - base_pos {
            Err(ArchiveBoundsError::OutOfBounds {
                base: base_pos as usize,
                offset,
                archive_len: self.len,
            })
        } else {
            let target_pos = (base_pos + offset) as usize;
            if target_pos & (align - 1) != 0 {
                Err(ArchiveBoundsError::Unaligned {
                    pos: target_pos,
                    align,
                })
            } else if self.len - target_pos < count {
                Err(ArchiveBoundsError::Overrun {
                    pos: target_pos,
                    size: count,
                    archive_len: self.len,
                })
            } else {
                Ok(base.offset(offset))
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
    pub fn overlaps(&self, other: &Self) -> bool {
        self.start < other.end && other.start < self.end
    }
}

/// Errors that can occur related to archive memory.
#[derive(Debug)]
pub enum ArchiveMemoryError {
    /// An error occurred while checking the bounds of the memory region
    BoundsError(ArchiveBoundsError),
    /// Multiple objects claim to own the same memory region
    ClaimOverlap {
        /// A previous interval of bytes claimed by some object
        previous: Interval,
        /// The current interval of bytes being claimed by some object
        current: Interval,
    },
}

impl From<ArchiveBoundsError> for ArchiveMemoryError {
    fn from(e: ArchiveBoundsError) -> Self {
        Self::BoundsError(e)
    }
}

impl fmt::Display for ArchiveMemoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArchiveMemoryError::BoundsError(e) => write!(f, "bounds error: {}", e),
            ArchiveMemoryError::ClaimOverlap { previous, current } => write!(
                f,
                "memory claim overlap: current [{:#?}..{:#?}] overlaps previous [{:#?}..{:#?}]",
                current.start, current.end, previous.start, previous.end
            ),
        }
    }
}

impl error::Error for ArchiveMemoryError {}

/// Context to perform archive validation.
///
/// When implementing archivable containers, an archived type may point to some
/// bytes elsewhere in the archive using a [`RelPtr`]. Before checking those
/// bytes, they must be claimed in the context. This prevents infinite-loop
/// attacks by malicious actors by ensuring that each block of memory has one
/// and only one owner.
///
/// # Example
/// ```
/// use core::{fmt, marker::PhantomData};
/// use std::error::Error;
/// use rkyv::{
///     Archive,
///     ArchiveContext,
///     ArchiveMemoryError,
///     RelPtr,
///     Serialize,
///     Write,
/// };
/// use bytecheck::{CheckBytes, Unreachable};
///
/// pub struct MyBox<T> {
///     value: *mut T,
/// }
///
/// impl<T> MyBox<T> {
///     fn new(value: T) -> Self {
///         Self {
///             value: Box::into_raw(Box::new(value)),
///         }
///     }
///
///     fn value(&self) -> &T {
///         unsafe { &*self.value }
///     }
/// }
///
/// impl<T> Drop for MyBox<T> {
///     fn drop(&mut self) {
///         unsafe {
///             Box::from_raw(self.value);
///         }
///     }
/// }
///
/// // A transparent representation guarantees us the same representation as
/// // a RelPtr
/// #[repr(transparent)]
/// pub struct ArchivedMyBox<T> {
///     value: RelPtr,
///     _phantom: PhantomData<T>,
/// }
///
/// impl<T> ArchivedMyBox<T> {
///     fn value(&self) -> &T {
///         unsafe { &*self.value.as_ptr() }
///     }
/// }
///
/// pub struct MyBoxResolver {
///     value_pos: usize,
/// }
///
/// impl<T: Archive> Archive for MyBox<T> {
///     type Archived = ArchivedMyBox<T::Archived>;
///     type Resolver = MyBoxResolver;
///
///     fn resolve(&self, pos: usize, resolver: Self::Resolver) -> Self::Archived {
///         unsafe {
///             ArchivedMyBox {
///                 value: RelPtr::new(pos, resolver.value_pos),
///                 _phantom: PhantomData,
///             }
///         }
///     }
/// }
///
/// impl<T: Serialize<W>, W: Write + ?Sized> Serialize<W> for MyBox<T> {
///     fn serialize(&self, writer: &mut W) -> Result<Self::Resolver, W::Error> {
///         Ok(MyBoxResolver {
///             value_pos: writer.serialize(self.value())?,
///         })
///     }
/// }
///
/// #[derive(Debug)]
/// pub enum ArchivedMyBoxError<T> {
///     MemoryError(ArchiveMemoryError),
///     CheckValueError(T),
/// }
///
/// impl<T: fmt::Display> fmt::Display for ArchivedMyBoxError<T> {
///     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
///         match self {
///             ArchivedMyBoxError::MemoryError(e) => write!(f, "memory error: {}", e),
///             ArchivedMyBoxError::CheckValueError(e) => write!(f, "check value error: {}", e),
///         }
///     }
/// }
///
/// impl<T: Error> Error for ArchivedMyBoxError<T> {}
///
/// impl<T> From<Unreachable> for ArchivedMyBoxError<T> {
///     fn from(e: Unreachable) -> Self {
///         unreachable!()
///     }
/// }
///
/// impl<T> From<ArchiveMemoryError> for ArchivedMyBoxError<T> {
///     fn from(e: ArchiveMemoryError) -> Self {
///         ArchivedMyBoxError::MemoryError(e)
///     }
/// }
///
/// impl<T: CheckBytes<C>, C: ArchiveContext + ?Sized> CheckBytes<C> for ArchivedMyBox<T> {
///     type Error = ArchivedMyBoxError<T::Error>;
///
///     unsafe fn check_bytes<'a>(
///         bytes: *const u8,
///         context: &mut C
///     ) -> Result<&'a Self, Self::Error> {
///         let rel_ptr = RelPtr::check_bytes(bytes, context)?;
///         let value_bytes = context.claim::<T>(rel_ptr, 1)?;
///         T::check_bytes(value_bytes, context)
///             .map_err(|e| ArchivedMyBoxError::CheckValueError(e))?;
///         Ok(&*bytes.cast())
///     }
/// }
/// ```
pub trait ArchiveContext {
    /// Claims `count` bytes located `offset` bytes away from `base`.
    ///
    /// # Safety
    ///
    /// `base` must be inside the archive this context was created for.
    unsafe fn claim_bytes(&mut self, base: *const u8, offset: isize, count: usize, align: usize) -> Result<*const u8, ArchiveMemoryError>;

    /// Claims `count` items pointed to by the given relative pointer.
    ///
    /// # Safety
    ///
    /// `rel_ptr` must be inside the archive this context was created for.
    unsafe fn claim<T: CheckBytes<Self>>(
        &mut self,
        rel_ptr: &RelPtr,
        count: usize,
    ) -> Result<*const u8, ArchiveMemoryError> {
        let base = (rel_ptr as *const RelPtr).cast::<u8>();
        let offset = rel_ptr.offset();

        self.claim_bytes(
            base,
            offset,
            count * mem::size_of::<T>(),
            mem::align_of::<T>(),
        )
    }
}

/// An [`ArchiveContext`] that completely validates an archive.
///
/// It performs bounds checking and enforces memory ownership.
pub struct ArchiveValidator<B: ArchiveBoundsContext> {
    bounds: B,
    intervals: Vec<Interval>,
}

impl<B: ArchiveBoundsContext> ArchiveValidator<B> {
    /// Creates a new archive context for the given byte slice
    pub fn new(bounds: B) -> Self {
        const DEFAULT_INTERVALS_CAPACITY: usize = 64;

        Self {
            bounds,
            intervals: Vec::with_capacity(DEFAULT_INTERVALS_CAPACITY),
        }
    }

    /// Gets the underlying bounds validator.
    pub fn bounds(&self) -> &B {
        &self.bounds
    }
}

impl<B: ArchiveBoundsContext> ArchiveContext for ArchiveValidator<B> {
    unsafe fn claim_bytes(
        &mut self,
        base: *const u8,
        offset: isize,
        count: usize,
        align: usize,
    ) -> Result<*const u8, ArchiveMemoryError> {
        let ptr = self.bounds.check_rel_ptr(base, offset, count, align)?;
        let interval = Interval {
            start: ptr,
            end: ptr.add(count),
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
                        return Ok(ptr);
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
                        return Ok(ptr);
                    }
                }

                self.intervals.insert(index, interval);
                Ok(ptr)
            }
        }
    }
}

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

impl<C: ArchiveContext + ?Sized> CheckBytes<C> for RelPtr {
    type Error = Unreachable;

    unsafe fn check_bytes<'a>(
        bytes: *const u8,
        context: &mut C,
    ) -> Result<&'a Self, Self::Error> {
        Offset::check_bytes(bytes, context)?;
        Ok(&*bytes.cast())
    }
}

pub trait SharedArchiveContext {
    /// Claims `count` shared bytes located `offset` bytes away from `base`.
    ///
    /// If the bytes need to be checked, returns `Some`. If the bytes have
    /// have already been checked, returns `None`.
    ///
    /// # Safety
    ///
    /// `base` must be inside the archive this context was created for.
    unsafe fn claim_shared_bytes(&mut self, base: *const u8, offset: isize, count: usize, align: usize) -> Result<Option<*const u8>, ArchiveMemoryError>;

    /// Claims `count` shared items pointed to by the given relative pointer.
    ///
    /// If the items need to be checked, returns `Some`. If the bytes have
    /// have already been checked, returns `None`.
    ///
    /// # Safety
    ///
    /// `rel_ptr` must be inside the archive this context was created for.
    unsafe fn claim_shared<T: CheckBytes<Self>>(
        &mut self,
        rel_ptr: &RelPtr,
        count: usize,
    ) -> Result<Option<*const u8>, ArchiveMemoryError> {
        let base = (rel_ptr as *const RelPtr).cast::<u8>();
        let offset = rel_ptr.offset();

        self.claim_shared_bytes(base, offset, count * mem::size_of::<T>(), mem::align_of::<T>())
    }
}

pub type DefaultArchiveValidator = ArchiveValidator<ArchiveBoundsValidator>;

/// Checks the given archive at the given position for an archived version of
/// the given type.
///
/// This is a safe alternative to [`archived_value`](crate::archived_value) for types that implement
/// `CheckBytes`.
///
/// # Example
/// ```
/// use rkyv::{Aligned, Archive, ArchiveBuffer, check_archive, Serialize, Write};
/// use bytecheck::CheckBytes;
///
/// #[derive(Archive, Serialize)]
/// #[archive(derive(CheckBytes))]
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
/// let mut writer = ArchiveBuffer::new(Aligned([0u8; 256]));
/// let pos = writer.serialize(&value)
///     .expect("failed to archive test");
/// let buf = writer.into_inner();
/// let archived = check_archive::<Example>(buf.as_ref(), pos).unwrap();
/// ```
pub fn check_archive<T: Archive>(
    buf: &[u8],
    pos: usize,
) -> Result<&T::Archived, CheckArchiveError<<T::Archived as CheckBytes<DefaultArchiveValidator>>::Error>>
where
    T::Archived: CheckBytes<DefaultArchiveValidator>,
{
    let mut validator = ArchiveValidator::new(ArchiveBoundsValidator::new(buf));
    check_archive_with_context::<T, DefaultArchiveValidator>(buf, pos, &mut validator)
}

/// Checks the given archive with an additional context.
///
/// See [`check_archive`] for more details.
pub fn check_archive_with_context<'a, T: Archive, C: ArchiveContext + ?Sized>(
    buf: &'a [u8],
    pos: usize,
    context: &mut C,
) -> Result<&'a T::Archived, CheckArchiveError<<T::Archived as CheckBytes<C>>::Error>>
where
    T::Archived: CheckBytes<C>,
{
    unsafe {
        let bytes = context.claim_bytes(
            buf.as_ptr(),
            pos as isize,
            mem::size_of::<T::Archived>(),
            mem::align_of::<T::Archived>(),
        )?;
        Ok(Archived::<T>::check_bytes(bytes, context).map_err(CheckArchiveError::CheckBytes)?)
    }
}
