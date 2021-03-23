//! Validation implementations and helper types.

use crate::{
    offset_of, Archive, ArchivePointee, Archived, ArchivedIsize, Fallible, RawRelPtr, RelPtr,
};
use bytecheck::{CheckBytes, Unreachable};
use core::{
    alloc::Layout,
    any::TypeId,
    fmt,
    marker::{PhantomData, PhantomPinned},
};
use ptr_meta::{DynMetadata, Pointee};
use std::{collections::HashMap, error::Error};

impl RawRelPtr {
    /// Checks the bytes of the given raw relative pointer.
    ///
    /// This is done rather than implementing `CheckBytes` to force users to
    /// manually write their `CheckBytes` implementation since they need to also
    /// provide the ownership model of their memory.
    ///
    /// # Safety
    ///
    /// The given pointer must be aligned and point to enough bytes to represent
    /// a `RawRelPtr`.
    pub unsafe fn manual_check_bytes<'a, C: Fallible + ?Sized>(
        value: *const RawRelPtr,
        context: &mut C,
    ) -> Result<&'a Self, Unreachable> {
        let bytes = value.cast::<u8>();
        ArchivedIsize::check_bytes(bytes.add(offset_of!(Self, offset)).cast(), context).unwrap();
        PhantomPinned::check_bytes(bytes.add(offset_of!(Self, _phantom)).cast(), context).unwrap();
        Ok(&*value)
    }
}

impl<T: ArchivePointee + ?Sized> RelPtr<T> {
    /// Checks the bytes of the given relative pointer.
    ///
    /// This is done rather than implementing `CheckBytes` to force users to
    /// manually write their `CheckBytes` implementation since they need to also
    /// provide the ownership model of their memory.
    ///
    /// # Safety
    ///
    /// The given pointer must be aligned and point to enough bytes to represent
    /// a `RelPtr<T>`.
    pub unsafe fn manual_check_bytes<'a, C: Fallible + ?Sized>(
        value: *const RelPtr<T>,
        context: &mut C,
    ) -> Result<&'a Self, <T::ArchivedMetadata as CheckBytes<C>>::Error>
    where
        T: CheckBytes<C>,
        T::ArchivedMetadata: CheckBytes<C>,
    {
        let bytes = value.cast::<u8>();
        RawRelPtr::manual_check_bytes(bytes.add(offset_of!(Self, raw_ptr)).cast(), context)
            .unwrap();
        T::ArchivedMetadata::check_bytes(bytes.add(offset_of!(Self, metadata)).cast(), context)?;
        PhantomData::<T>::check_bytes(bytes.add(offset_of!(Self, _phantom)).cast(), context)
            .unwrap();
        Ok(&*value)
    }
}

/// Gets the layout of a type from its metadata.
pub trait LayoutMetadata<T: ?Sized> {
    /// Gets the layout of the type.
    fn layout(self) -> Layout;
}

impl<T> LayoutMetadata<T> for () {
    fn layout(self) -> Layout {
        Layout::new::<T>()
    }
}

impl<T> LayoutMetadata<[T]> for usize {
    fn layout(self) -> Layout {
        Layout::array::<T>(self).unwrap()
    }
}

impl LayoutMetadata<str> for usize {
    fn layout(self) -> Layout {
        Layout::array::<u8>(self).unwrap()
    }
}

impl<T: ?Sized> LayoutMetadata<T> for DynMetadata<T> {
    fn layout(self) -> Layout {
        self.layout()
    }
}

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

impl Error for ArchiveBoundsError {}

/// A context that can check relative pointers.
pub trait ArchiveBoundsContext: Fallible {
    /// Checks the given parts of a relative pointer for bounds issues
    ///
    /// # Safety
    ///
    /// The base pointer must be inside the archive for this context.
    unsafe fn check_rel_ptr(
        &mut self,
        base: *const u8,
        offset: isize,
    ) -> Result<*const u8, Self::Error>;

    /// Checks the given memory block for bounds issues.
    ///
    /// # Safety
    ///
    /// The pointer must be inside the archive for this context.
    unsafe fn bounds_check_ptr(
        &mut self,
        ptr: *const u8,
        layout: &Layout,
    ) -> Result<(), Self::Error>;
}

/// A validator that can bounds check pointers in an archive.
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

    /// Returns whether the byte range is empty.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl Fallible for ArchiveBoundsValidator {
    type Error = ArchiveBoundsError;
}

impl ArchiveBoundsContext for ArchiveBoundsValidator {
    unsafe fn check_rel_ptr(
        &mut self,
        base: *const u8,
        offset: isize,
    ) -> Result<*const u8, Self::Error> {
        let base_pos = base.offset_from(self.begin);
        if offset < -base_pos || offset > self.len as isize - base_pos {
            Err(ArchiveBoundsError::OutOfBounds {
                base: base_pos as usize,
                offset,
                archive_len: self.len,
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
        if (self.begin as usize) & (layout.align() - 1) != 0 {
            Err(ArchiveBoundsError::Underaligned {
                expected_align: layout.align(),
                actual_align: 1 << (self.begin as usize).trailing_zeros(),
            })
        } else {
            let target_pos = ptr.offset_from(self.begin) as usize;
            if target_pos & (layout.align() - 1) != 0 {
                Err(ArchiveBoundsError::Unaligned {
                    pos: target_pos,
                    align: layout.align(),
                })
            } else if self.len - target_pos < layout.size() {
                Err(ArchiveBoundsError::Overrun {
                    pos: target_pos,
                    size: layout.size(),
                    archive_len: self.len,
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

impl<E: Error + 'static> Error for ArchiveMemoryError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ArchiveMemoryError::Inner(e) => Some(e as &dyn Error),
            ArchiveMemoryError::ClaimOverlap { .. } => None,
        }
    }
}

/// A context that can validate archive memory.
///
/// When implementing archivable containers, an archived type may point to some
/// bytes elsewhere in the archive using a [`RelPtr`]. Before checking those
/// bytes, they must be claimed in the context. This prevents infinite-loop
/// attacks by malicious actors by ensuring that each block of memory has one
/// and only one owner.
pub trait ArchiveMemoryContext: Fallible {
    /// Claims `count` bytes located `offset` bytes away from `base`.
    ///
    /// # Safety
    ///
    /// `base` must be inside the archive this context was created for.
    unsafe fn claim_bytes(&mut self, start: *const u8, len: usize) -> Result<(), Self::Error>;

    /// Claims the memory at the given location as the given type.
    ///
    /// # Safety
    ///
    /// `ptr` must be inside the archive this context was created for.
    unsafe fn claim_owned_ptr<T: ArchivePointee + ?Sized>(
        &mut self,
        ptr: *const T,
    ) -> Result<(), Self::Error>
    where
        Self: ArchiveBoundsContext,
        <T as Pointee>::Metadata: LayoutMetadata<T>,
    {
        let metadata = ptr_meta::metadata(ptr);
        let layout = LayoutMetadata::<T>::layout(metadata);
        self.bounds_check_ptr(ptr.cast(), &layout)?;
        self.claim_bytes(ptr.cast(), layout.size())?;
        Ok(())
    }

    /// Claims the memory referenced by the given relative pointer.
    fn claim_owned_rel_ptr<T: ArchivePointee + ?Sized>(
        &mut self,
        rel_ptr: &RelPtr<T>,
    ) -> Result<*const T, Self::Error>
    where
        Self: ArchiveBoundsContext,
        <T as Pointee>::Metadata: LayoutMetadata<T>,
    {
        unsafe {
            let data = self.check_rel_ptr(rel_ptr.base(), rel_ptr.offset())?;
            let ptr =
                ptr_meta::from_raw_parts::<T>(data.cast(), T::pointer_metadata(rel_ptr.metadata()));
            self.claim_owned_ptr(ptr)?;
            Ok(ptr)
        }
    }
}

/// An adapter that adds memory validation to a context.
pub struct ArchiveValidator<C> {
    inner: C,
    intervals: Vec<Interval>,
}

impl<C> ArchiveValidator<C> {
    /// Wraps the given validator context and adds memory validation.
    pub fn new(inner: C) -> Self {
        Self {
            inner,
            intervals: Vec::new(),
        }
    }

    /// Consumes the adapter and returns the underlying validator.
    pub fn into_inner(self) -> C {
        self.inner
    }
}

impl<C: Fallible> Fallible for ArchiveValidator<C> {
    type Error = ArchiveMemoryError<C::Error>;
}

impl<C: ArchiveBoundsContext> ArchiveBoundsContext for ArchiveValidator<C> {
    unsafe fn check_rel_ptr(
        &mut self,
        base: *const u8,
        offset: isize,
    ) -> Result<*const u8, Self::Error> {
        self.inner
            .check_rel_ptr(base, offset)
            .map_err(ArchiveMemoryError::Inner)
    }

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

impl<E: Error + 'static> Error for SharedArchiveError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            SharedArchiveError::Inner(e) => Some(e as &dyn Error),
            SharedArchiveError::TypeMismatch { .. } => None,
        }
    }
}

/// A context that can validate shared archive memory.
///
/// Shared pointers require this kind of context to validate.
pub trait SharedArchiveContext: Fallible {
    /// Claims `count` shared bytes located `offset` bytes away from `base`.
    ///
    /// Returns whether the bytes need to be checked.
    ///
    /// # Safety
    ///
    /// `base` must be inside the archive this context was created for.
    unsafe fn claim_shared_bytes(
        &mut self,
        start: *const u8,
        len: usize,
        type_id: TypeId,
    ) -> Result<bool, Self::Error>;

    /// Claims the memory referenced by the given relative pointer.
    ///
    /// If the pointer needs to be checked, returns `Some` with the pointer to
    /// check.
    fn claim_shared_ptr<T: ArchivePointee + CheckBytes<Self> + ?Sized>(
        &mut self,
        rel_ptr: &RelPtr<T>,
        type_id: TypeId,
    ) -> Result<Option<*const T>, Self::Error>
    where
        Self: ArchiveBoundsContext,
        <T as Pointee>::Metadata: LayoutMetadata<T>,
    {
        unsafe {
            let data = self.check_rel_ptr(rel_ptr.base(), rel_ptr.offset())?;
            let metadata = T::pointer_metadata(rel_ptr.metadata());
            let ptr = ptr_meta::from_raw_parts::<T>(data.cast(), metadata);
            let layout = LayoutMetadata::<T>::layout(metadata);
            self.bounds_check_ptr(ptr.cast(), &layout)?;
            if self.claim_shared_bytes(ptr.cast(), layout.size(), type_id)? {
                Ok(Some(ptr))
            } else {
                Ok(None)
            }
        }
    }
}

/// An adapter that adds shared memory validation.
pub struct SharedArchiveValidator<C> {
    inner: C,
    shared_blocks: HashMap<*const u8, TypeId>,
}

impl<C> SharedArchiveValidator<C> {
    /// Wraps the given context and adds shared memory validation.
    pub fn new(inner: C) -> Self {
        Self {
            inner,
            shared_blocks: HashMap::new(),
        }
    }

    /// Consumes the adapter and returns the underlying validator.
    pub fn into_inner(self) -> C {
        self.inner
    }
}

impl<C: Fallible> Fallible for SharedArchiveValidator<C> {
    type Error = SharedArchiveError<C::Error>;
}

impl<C: ArchiveBoundsContext> ArchiveBoundsContext for SharedArchiveValidator<C> {
    unsafe fn check_rel_ptr(
        &mut self,
        base: *const u8,
        offset: isize,
    ) -> Result<*const u8, Self::Error> {
        self.inner
            .check_rel_ptr(base, offset)
            .map_err(SharedArchiveError::Inner)
    }

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

impl<C: ArchiveMemoryContext> ArchiveMemoryContext for SharedArchiveValidator<C> {
    unsafe fn claim_bytes(&mut self, start: *const u8, len: usize) -> Result<(), Self::Error> {
        self.inner
            .claim_bytes(start, len)
            .map_err(SharedArchiveError::Inner)
    }
}

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

/// Errors that can occur when checking an archive.
#[derive(Debug)]
pub enum CheckArchiveError<T, C> {
    /// An error that occurred while validating an object
    CheckBytesError(T),
    /// A context error occurred
    ContextError(C),
}

impl<T: fmt::Display, C: fmt::Display> fmt::Display for CheckArchiveError<T, C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CheckArchiveError::CheckBytesError(e) => write!(f, "check bytes error: {}", e),
            CheckArchiveError::ContextError(e) => write!(f, "context error: {}", e),
        }
    }
}

impl<T: Error + 'static, C: Error + 'static> Error for CheckArchiveError<T, C> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            CheckArchiveError::CheckBytesError(e) => Some(e as &dyn Error),
            CheckArchiveError::ContextError(e) => Some(e as &dyn Error),
        }
    }
}

/// A validator that supports all builtin types.
pub type DefaultArchiveValidator = SharedArchiveValidator<ArchiveValidator<ArchiveBoundsValidator>>;

/// The error type that can be produced by checking the given type with the given validator.
pub type CheckTypeError<T, C> =
    CheckArchiveError<<T as CheckBytes<C>>::Error, <C as Fallible>::Error>;

/// Checks the given archive at the given position for an archived version of
/// the given type.
///
/// This is a safe alternative to [`archived_value`](crate::archived_value) for types that implement
/// `CheckBytes`.
///
/// # Example
/// ```
/// use rkyv::{
///     check_archive,
///     ser::{Serializer, serializers::WriteSerializer},
///     AlignedVec,
///     Archive,
///     Serialize,
/// };
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
/// let mut serializer = WriteSerializer::new(AlignedVec::new());
/// let pos = serializer.serialize_value(&value)
///     .expect("failed to archive test");
/// let buf = serializer.into_inner();
/// let archived = check_archive::<Example>(buf.as_ref(), pos).unwrap();
/// ```
pub fn check_archive<T: Archive>(
    buf: &[u8],
    pos: usize,
) -> Result<&T::Archived, CheckTypeError<T::Archived, DefaultArchiveValidator>>
where
    T::Archived: CheckBytes<DefaultArchiveValidator>,
{
    let mut validator =
        SharedArchiveValidator::new(ArchiveValidator::new(ArchiveBoundsValidator::new(buf)));
    check_archive_with_context::<T, DefaultArchiveValidator>(buf, pos, &mut validator)
}

/// Checks the given archive with an additional context.
///
/// See [`check_archive`] for more details.
pub fn check_archive_with_context<
    'a,
    T: Archive,
    C: ArchiveBoundsContext + ArchiveMemoryContext + ?Sized,
>(
    buf: &'a [u8],
    pos: usize,
    context: &mut C,
) -> Result<&'a T::Archived, CheckTypeError<T::Archived, C>>
where
    T::Archived: CheckBytes<C> + Pointee<Metadata = ()>,
{
    unsafe {
        let data = context
            .check_rel_ptr(buf.as_ptr(), pos as isize)
            .map_err(CheckArchiveError::ContextError)?;
        let ptr = ptr_meta::from_raw_parts::<<T as Archive>::Archived>(data.cast(), ());
        let layout = LayoutMetadata::<T::Archived>::layout(());
        context
            .bounds_check_ptr(ptr.cast(), &layout)
            .map_err(CheckArchiveError::ContextError)?;
        context
            .claim_bytes(ptr.cast(), layout.size())
            .map_err(CheckArchiveError::ContextError)?;
        Ok(Archived::<T>::check_bytes(ptr, context).map_err(CheckArchiveError::CheckBytesError)?)
    }
}
