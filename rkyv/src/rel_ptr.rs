//! Relative pointer implementations and options.

use core::{
    fmt,
    marker::{PhantomData, PhantomPinned},
    ptr::{self, addr_of_mut},
};

use rancor::{fail, Error, Panic, ResultExt as _};

use crate::{
    primitive::{
        ArchivedI16, ArchivedI32, ArchivedI64, ArchivedU16, ArchivedU32,
        ArchivedU64,
    },
    ArchivePointee, Portable,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct IsizeOverflow;

impl fmt::Display for IsizeOverflow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "the offset overflowed the range of `isize`")
    }
}

#[cfg(feature = "std")]
impl std::error::Error for IsizeOverflow {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ExceedsStorageRange;

impl fmt::Display for ExceedsStorageRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "the offset is too far for the offset type of the relative pointer",
        )
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ExceedsStorageRange {}

/// A offset that can be used with [`RawRelPtr`].
pub trait Offset: Copy {
    /// Creates a new offset between a `from` position and a `to` position.
    fn from_isize<E: Error>(value: isize) -> Result<Self, E>;

    /// Gets the offset as an `isize`.
    fn to_isize(self) -> isize;
}

macro_rules! impl_offset_single_byte {
    ($ty:ty) => {
        impl Offset for $ty {
            #[inline]
            fn from_isize<E: Error>(value: isize) -> Result<Self, E> {
                // `pointer::add`` and `pointer::offset` require that the
                // computed offsets cannot overflow an isize, which is why we're
                // using signed_offset instead of `checked_sub` for unsized
                // types.
                Self::try_from(value).into_error()
            }

            #[inline]
            fn to_isize(self) -> isize {
                // We're guaranteed that our offset will not exceed the
                // capacity of an `isize`
                self as isize
            }
        }
    };
}

impl_offset_single_byte!(i8);
impl_offset_single_byte!(u8);

macro_rules! impl_offset_multi_byte {
    ($ty:ty, $archived:ty) => {
        impl Offset for $archived {
            #[inline]
            fn from_isize<E: Error>(value: isize) -> Result<Self, E> {
                // `pointer::add`` and `pointer::offset` require that the
                // computed offsets cannot overflow an isize, which is why we're
                // using signed_offset instead of `checked_sub` for unsized
                // types.
                Ok(<$archived>::from_native(
                    <$ty>::try_from(value).into_error()?,
                ))
            }

            #[inline]
            fn to_isize(self) -> isize {
                // We're guaranteed that our offset will not exceed the
                // capacity of an `isize`.
                self.to_native() as isize
            }
        }
    };
}

impl_offset_multi_byte!(i16, ArchivedI16);
#[cfg(any(target_pointer_width = "32", target_pointer_width = "64"))]
impl_offset_multi_byte!(i32, ArchivedI32);
#[cfg(target_pointer_width = "64")]
impl_offset_multi_byte!(i64, ArchivedI64);

impl_offset_multi_byte!(u16, ArchivedU16);
#[cfg(any(target_pointer_width = "32", target_pointer_width = "64"))]
impl_offset_multi_byte!(u32, ArchivedU32);
#[cfg(target_pointer_width = "64")]
impl_offset_multi_byte!(u64, ArchivedU64);

/// Errors that can occur while creating raw relative pointers.
#[derive(Debug)]
pub enum RelPtrError {
    /// The given `from` and `to` positions for the relative pointer failed to
    /// form a valid offset.
    ///
    /// This is probably because the distance between them could not be
    /// represented by the offset type.
    OffsetError,
}

/// An untyped pointer which resolves relative to its position in memory.
///
/// This is the most fundamental building block in rkyv. It allows the
/// construction and use of pointers that can be safely relocated as long as the
/// source and target are moved together. This is what allows memory to be moved
/// from disk into memory and accessed without decoding.
///
/// Regular pointers are *absolute*, meaning that the pointer can be moved
/// without being invalidated. However, the pointee **cannot** be moved,
/// otherwise the pointer is invalidated.
///
/// Relative pointers are *relative*, meaning that the **pointer** can be moved
/// with the **pointee** without invalidating the pointer. However, if either
/// the **pointer** or the **pointee** move independently, the pointer will be
/// invalidated.
// TODO: should RawRelPtr not implement Portable? We're missing a check that the
// type it gets a pointer to is Portable.
#[derive(Portable)]
#[archive(crate)]
#[cfg_attr(feature = "bytecheck", derive(bytecheck::CheckBytes))]
#[repr(transparent)]
pub struct RawRelPtr<O> {
    offset: O,
    _phantom: PhantomPinned,
}

/// Calculates the offset between two positions as an `isize`.
///
/// This function exists solely to get the distance between two `usizes` as an
/// `isize` with a full range of values.
///
/// # Examples
///
/// ```
/// # use rkyv::rel_ptr::signed_offset;
/// # use rancor::Failure;
///
/// assert_eq!(signed_offset::<Failure>(0, 1), Ok(1));
/// assert_eq!(signed_offset::<Failure>(1, 0), Ok(-1));
/// assert_eq!(
///     signed_offset::<Failure>(0, isize::MAX as usize),
///     Ok(isize::MAX)
/// );
/// assert_eq!(
///     signed_offset::<Failure>(isize::MAX as usize, 0),
///     Ok(-isize::MAX)
/// );
/// assert!(signed_offset::<Failure>(0, isize::MAX as usize + 1).is_err());
/// assert_eq!(
///     signed_offset::<Failure>(isize::MAX as usize + 1, 0),
///     Ok(isize::MIN)
/// );
/// assert!(signed_offset::<Failure>(0, isize::MAX as usize + 2).is_err());
/// assert!(signed_offset::<Failure>(isize::MAX as usize + 2, 0).is_err());
/// ```
#[inline]
pub fn signed_offset<E: Error>(from: usize, to: usize) -> Result<isize, E> {
    let (result, overflow) = to.overflowing_sub(from);
    if (!overflow && result <= (isize::MAX as usize))
        || (overflow && result >= (isize::MIN as usize))
    {
        Ok(result as isize)
    } else {
        fail!(IsizeOverflow);
    }
}

impl<O: Offset> RawRelPtr<O> {
    /// Attempts to create a new `RawRelPtr` in-place between the given `from`
    /// and `to` positions.
    ///
    /// # Safety
    ///
    /// `out` must point to a `Self` that is valid for reads and writes.
    #[inline]
    pub unsafe fn try_emplace<E: Error>(
        from: usize,
        to: usize,
        out: *mut Self,
    ) -> Result<(), E> {
        let offset = O::from_isize(signed_offset(from, to)?)?;
        ptr::addr_of_mut!((*out).offset).write(offset);
        Ok(())
    }

    /// Creates a new `RawRelPtr` in-place between the given `from` and `to`
    /// positions.
    ///
    /// # Panics
    ///
    /// - If the offset between `from` and `to` does not fit in an `isize`
    /// - If the offset between `from` and `to` exceeds the offset storage
    ///
    /// # Safety
    ///
    /// `out` must point to a `Self` that is valid for reads and writes.
    #[inline]
    pub unsafe fn emplace(from: usize, to: usize, out: *mut Self) {
        Self::try_emplace::<Panic>(from, to, out).always_ok()
    }

    /// Gets the base pointer for the relative pointer.
    #[inline]
    pub fn base(&self) -> *mut u8 {
        (self as *const Self).cast_mut().cast::<u8>()
    }

    /// Gets the offset of the relative pointer from its base.
    #[inline]
    pub fn offset(&self) -> isize {
        self.offset.to_isize()
    }

    /// Gets whether the offset of the relative pointer is 0.
    #[inline]
    pub fn is_null(&self) -> bool {
        self.offset() == 0
    }

    /// Calculates the memory address being pointed to by this relative pointer.
    ///
    /// # Safety
    ///
    /// The offset of this relative pointer, when added to its base, must be
    /// located in the same allocated object as it.
    #[inline]
    pub unsafe fn as_ptr(&self) -> *mut () {
        unsafe { self.base().offset(self.offset()).cast() }
    }

    /// Calculates the memory address being pointed to by this relative pointer
    /// using wrapping methods.
    ///
    /// This method is a safer but potentially slower version of `as_ptr`.
    #[inline]
    pub fn as_ptr_wrapping(&self) -> *mut () {
        self.base().wrapping_offset(self.offset()).cast()
    }
}

impl<O: fmt::Debug> fmt::Debug for RawRelPtr<O> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RawRelPtr")
            .field("offset", &self.offset)
            .finish()
    }
}

impl<O: Offset> fmt::Pointer for RawRelPtr<O> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Pointer::fmt(&self.as_ptr_wrapping(), f)
    }
}

/// A raw relative pointer that uses an archived `i8` as the underlying offset.
pub type RawRelPtrI8 = RawRelPtr<i8>;
/// A raw relative pointer that uses an archived `i16` as the underlying offset.
pub type RawRelPtrI16 = RawRelPtr<ArchivedI16>;
/// A raw relative pointer that uses an archived `i32` as the underlying offset.
#[cfg(any(target_pointer_width = "32", target_pointer_width = "64"))]
pub type RawRelPtrI32 = RawRelPtr<ArchivedI32>;
/// A raw relative pointer that uses an archived `i64` as the underlying offset.
#[cfg(target_pointer_width = "64")]
pub type RawRelPtrI64 = RawRelPtr<ArchivedI64>;

/// A raw relative pointer that uses an archived `u8` as the underlying offset.
pub type RawRelPtrU8 = RawRelPtr<u8>;
/// A raw relative pointer that uses an archived `u16` as the underlying offset.
pub type RawRelPtrU16 = RawRelPtr<ArchivedU16>;
/// A raw relative pointer that uses an archived `u32` as the underlying offset.
#[cfg(any(target_pointer_width = "32", target_pointer_width = "64"))]
pub type RawRelPtrU32 = RawRelPtr<ArchivedU32>;
/// A raw relative pointer that uses an archived `u64` as the underlying offset.
#[cfg(target_pointer_width = "64")]
pub type RawRelPtrU64 = RawRelPtr<ArchivedU64>;

/// A pointer which resolves to relative to its position in memory.
///
/// This is a strongly-typed version of [`RawRelPtr`].
///
/// See [`Archive`](crate::Archive) for an example of creating one.
#[cfg_attr(feature = "bytecheck", derive(bytecheck::CheckBytes))]
#[repr(C)]
pub struct RelPtr<T: ArchivePointee + ?Sized, O> {
    raw_ptr: RawRelPtr<O>,
    metadata: T::ArchivedMetadata,
    _phantom: PhantomData<T>,
}

// SAFETY: `RelPtr<T, O>` is portable if all of its fields are portable _and_
// the target type is also portable.
unsafe impl<T, O> Portable for RelPtr<T, O>
where
    T: ArchivePointee + Portable + ?Sized,
    RawRelPtr<O>: Portable,
    T::ArchivedMetadata: Portable,
    PhantomData<T>: Portable,
{
}

impl<T, O: Offset> RelPtr<T, O> {
    /// Attempts to create a relative pointer from one position to another.
    ///
    /// # Safety
    ///
    /// `out` must point to a `Self` that is valid for reads and writes.
    #[inline]
    pub unsafe fn try_emplace<E: Error>(
        from: usize,
        to: usize,
        out: *mut Self,
    ) -> Result<(), E> {
        let (fp, fo) = out_field!(out.raw_ptr);
        // Skip metadata since sized T is guaranteed to be ()
        RawRelPtr::try_emplace(from + fp, to, fo)
    }

    /// Creates a relative pointer from one position to another.
    ///
    /// # Panics
    ///
    /// - If the offset between `from` and `to` does not fit in an `isize`
    /// - If the offset between `from` and `to` exceeds the offset storage
    ///
    /// # Safety
    ///
    /// `out` must point to a `Self` that is valid for reads and writes.
    #[inline]
    pub unsafe fn emplace(from: usize, to: usize, out: *mut Self) {
        Self::try_emplace::<Panic>(from, to, out).always_ok()
    }
}

impl<T: ArchivePointee + ?Sized, O: Offset> RelPtr<T, O>
where
    T::ArchivedMetadata: Default,
{
    /// Attempts to create a null relative pointer with default metadata.
    ///
    /// # Safety
    ///
    /// `out` must point to a `Self` that is valid for reads and writes.
    #[inline]
    pub unsafe fn try_emplace_null<E: Error>(
        pos: usize,
        out: *mut Self,
    ) -> Result<(), E> {
        let (fp, fo) = out_field!(out.raw_ptr);
        RawRelPtr::try_emplace(pos + fp, pos, fo)?;
        let (_, fo) = out_field!(out.metadata);
        fo.write(Default::default());
        Ok(())
    }

    /// Creates a null relative pointer with default metadata.
    ///
    /// # Panics
    ///
    /// - If an offset of `0` does not fit in an `isize`
    /// - If an offset of `0` exceeds the offset storage
    ///
    /// # Safety
    ///
    /// `out` must point to a `Self` that is valid for reads and writes.
    #[inline]
    pub unsafe fn emplace_null(pos: usize, out: *mut Self) {
        Self::try_emplace_null::<Panic>(pos, out).always_ok()
    }
}

impl<T: ArchivePointee + ?Sized, O: Offset> RelPtr<T, O> {
    /// Attempts to create a relative pointer from one position to another.
    ///
    /// # Safety
    ///
    /// `out` must point to a `Self` that is valid for reads and writes.
    #[inline]
    pub unsafe fn try_emplace_unsized<E: Error>(
        from: usize,
        to: usize,
        metadata: T::ArchivedMetadata,
        out: *mut Self,
    ) -> Result<(), E> {
        let (fp, fo) = out_field!(out.raw_ptr);
        RawRelPtr::try_emplace(from + fp, to, fo)?;
        addr_of_mut!((*out).metadata).write(metadata);
        Ok(())
    }

    /// Creates a relative pointer from one position to another.
    ///
    /// # Panics
    ///
    /// - If the offset between `from` and `to` does not fit in an `isize`
    /// - If the offset between `from` and `to` exceeds the offset storage
    ///
    /// # Safety
    ///
    /// `out` must point to a `Self` that is valid for reads and writes.
    #[inline]
    pub unsafe fn emplace_unsized(
        from: usize,
        to: usize,
        metadata: T::ArchivedMetadata,
        out: *mut Self,
    ) {
        Self::try_emplace_unsized::<Panic>(from, to, metadata, out).always_ok()
    }

    /// Gets the base pointer for the relative pointer.
    #[inline]
    pub fn base(&self) -> *mut u8 {
        self.raw_ptr.base()
    }

    /// Gets the offset of the relative pointer from its base.
    #[inline]
    pub fn offset(&self) -> isize {
        self.raw_ptr.offset()
    }

    /// Gets whether the offset of the relative pointer is 0.
    #[inline]
    pub fn is_null(&self) -> bool {
        self.raw_ptr.is_null()
    }

    /// Gets the metadata of the relative pointer.
    #[inline]
    pub fn metadata(&self) -> &T::ArchivedMetadata {
        &self.metadata
    }

    /// Calculates the memory address being pointed to by this relative pointer.
    ///
    /// # Safety
    ///
    /// The offset of this relative pointer, when added to its base, must be
    /// located in the same allocated object as it.
    #[inline]
    pub unsafe fn as_ptr(&self) -> *mut T {
        ptr_meta::from_raw_parts_mut(
            self.raw_ptr.as_ptr(),
            T::pointer_metadata(&self.metadata),
        )
    }

    /// Calculates the memory address being pointed to by this relative pointer
    /// using wrapping methods.
    ///
    /// This method is a safer but potentially slower version of `as_ptr`.
    #[inline]
    pub fn as_ptr_wrapping(&self) -> *mut T {
        ptr_meta::from_raw_parts_mut(
            self.raw_ptr.as_ptr_wrapping(),
            T::pointer_metadata(&self.metadata),
        )
    }
}

impl<T: ArchivePointee + ?Sized, O: fmt::Debug> fmt::Debug for RelPtr<T, O>
where
    T::ArchivedMetadata: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RelPtr")
            .field("raw_ptr", &self.raw_ptr)
            .field("metadata", &self.metadata)
            .finish()
    }
}

impl<T: ArchivePointee + ?Sized, O: Offset> fmt::Pointer for RelPtr<T, O> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Pointer::fmt(&self.as_ptr_wrapping(), f)
    }
}
