//! Relative pointer implementations and options.

use core::{
    error::Error,
    fmt,
    marker::{PhantomData, PhantomPinned},
    ptr::addr_of_mut,
};

use munge::munge;
use rancor::{fail, Panic, ResultExt as _, Source};

use crate::{
    primitive::{
        ArchivedI16, ArchivedI32, ArchivedI64, ArchivedU16, ArchivedU32,
        ArchivedU64,
    },
    seal::Seal,
    traits::{ArchivePointee, NoUndef},
    Place, Portable,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct IsizeOverflow;

impl fmt::Display for IsizeOverflow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "the offset overflowed the range of `isize`")
    }
}

impl Error for IsizeOverflow {}

/// A offset that can be used with [`RawRelPtr`].
pub trait Offset: Copy + NoUndef {
    /// Creates a new offset between a `from` position and a `to` position.
    fn from_isize<E: Source>(value: isize) -> Result<Self, E>;

    /// Gets the offset as an `isize`.
    fn to_isize(self) -> isize;
}

macro_rules! impl_offset_single_byte {
    ($ty:ty) => {
        impl Offset for $ty {
            fn from_isize<E: Source>(value: isize) -> Result<Self, E> {
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
            fn from_isize<E: Source>(value: isize) -> Result<Self, E> {
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
impl_offset_multi_byte!(i32, ArchivedI32);
impl_offset_multi_byte!(i64, ArchivedI64);

impl_offset_multi_byte!(u16, ArchivedU16);
impl_offset_multi_byte!(u32, ArchivedU32);
impl_offset_multi_byte!(u64, ArchivedU64);

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
#[cfg_attr(feature = "bytecheck", derive(bytecheck::CheckBytes))]
#[derive(Portable)]
#[rkyv(crate)]
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
/// # use rancor::Error;
/// assert!(signed_offset::<Error>(0, 1).is_ok_and(|x| x == 1));
/// assert!(signed_offset::<Error>(1, 0).is_ok_and(|x| x == -1));
/// assert!(signed_offset::<Error>(0, isize::MAX as usize)
///     .is_ok_and(|x| x == isize::MAX));
/// assert!(signed_offset::<Error>(isize::MAX as usize, 0)
///     .is_ok_and(|x| x == -isize::MAX));
/// assert!(signed_offset::<Error>(0, isize::MAX as usize + 1).is_err());
/// assert!(signed_offset::<Error>(isize::MAX as usize + 1, 0)
///     .is_ok_and(|x| x == isize::MIN));
/// assert!(signed_offset::<Error>(0, isize::MAX as usize + 2).is_err());
/// assert!(signed_offset::<Error>(isize::MAX as usize + 2, 0).is_err());
/// ```
pub fn signed_offset<E: Source>(from: usize, to: usize) -> Result<isize, E> {
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
    /// Attempts to create an invalid `RawRelPtr` in-place.
    pub fn try_emplace_invalid<E: Source>(out: Place<Self>) -> Result<(), E> {
        Self::try_emplace::<E>(out.pos() + 1, out)
    }

    /// Creates an invalid `RawRelPtr` in-place.
    ///
    /// # Panics
    ///
    /// - If an offset of `1` does not fit in an `isize`
    /// - If an offset of `1` exceeds the offset storage
    pub fn emplace_invalid(out: Place<Self>) {
        Self::try_emplace_invalid::<Panic>(out).always_ok();
    }

    /// Attempts to create a new `RawRelPtr` in-place between the given `from`
    /// and `to` positions.
    pub fn try_emplace<E: Source>(
        to: usize,
        out: Place<Self>,
    ) -> Result<(), E> {
        let offset = O::from_isize(signed_offset(out.pos(), to)?)?;
        munge!(let Self { offset: out_offset, _phantom: _ } = out);
        out_offset.write(offset);
        Ok(())
    }

    /// Creates a new `RawRelPtr` in-place between the given `from` and `to`
    /// positions.
    ///
    /// # Panics
    ///
    /// - If the offset between `out` and `to` does not fit in an `isize`
    /// - If the offset between `out` and `to` exceeds the offset storage
    pub fn emplace(to: usize, out: Place<Self>) {
        Self::try_emplace::<Panic>(to, out).always_ok()
    }

    /// Gets the base pointer for the pointed-to relative pointer.
    pub fn base_raw(this: *mut Self) -> *mut u8 {
        this.cast()
    }

    /// Gets the offset of the pointed-to relative pointer from its base.
    ///
    /// # Safety
    ///
    /// `this` must be non-null, properly-aligned, and point to a valid
    /// `RawRelPtr`.
    pub unsafe fn offset_raw(this: *mut Self) -> isize {
        // SAFETY: The caller has guaranteed that `this` is safe to dereference
        unsafe { addr_of_mut!((*this).offset).read().to_isize() }
    }

    /// Calculates the memory address being pointed to by the pointed-to
    /// relative pointer.
    ///
    /// # Safety
    ///
    /// - `this` must be non-null, properly-aligned, and point to a valid
    ///   `RawRelPtr`.
    /// - The offset of this relative pointer, when added to its base, must be
    ///   located in the same allocated object as it.
    pub unsafe fn as_ptr_raw(this: *mut Self) -> *mut () {
        // SAFETY:
        // - The caller has guaranteed that `this` is safe to dereference.
        // - The caller has guaranteed that offsetting the base pointer by its
        //   offset will yield a pointer in the same allocated object.
        unsafe { Self::base_raw(this).offset(Self::offset_raw(this)).cast() }
    }

    /// Calculates the memory address being pointed to by the pointed-to
    /// relative pointer using wrapping methods.
    ///
    /// This method is a safer but potentially slower version of `as_ptr_raw`.
    ///
    /// # Safety
    ///
    /// `this` must be non-null, properly-aligned, and point to a valid
    /// `RawRelPtr`.
    pub unsafe fn as_ptr_wrapping_raw(this: *mut Self) -> *mut () {
        // SAFETY: The safety requirements of `offset_raw` are the same as the
        // safety requirements for `as_ptr_wrapping_raw`.
        let offset = unsafe { Self::offset_raw(this) };
        Self::base_raw(this).wrapping_offset(offset).cast()
    }

    /// Gets whether the offset of the pointed-to relative pointer is invalid.
    ///
    /// # Safety
    ///
    /// `this` must be non-null, properly-aligned, and point to a valid
    /// `RawRelPtr`.
    pub unsafe fn is_invalid_raw(this: *mut Self) -> bool {
        // SAFETY: The safety requirements of `offset_raw` are the same as the
        // safety requirements for `is_invalid_raw`.
        unsafe { Self::offset_raw(this) == 1 }
    }

    /// Gets the base pointer for the relative pointer.
    pub fn base(&self) -> *const u8 {
        Self::base_raw((self as *const Self).cast_mut()).cast_const()
    }

    /// Gets the mutable base pointer for the relative pointer.
    pub fn base_mut(this: Seal<'_, Self>) -> *mut u8 {
        // SAFETY: The value pointed to by `this` is not moved and no bytes are
        // written through it.
        let this = unsafe { Seal::unseal_unchecked(this) };
        Self::base_raw(this as *mut Self)
    }

    /// Gets the offset of the relative pointer from its base.
    pub fn offset(&self) -> isize {
        let this = self as *const Self;
        // SAFETY: `self` is a reference, so it's guaranteed to be non-null,
        // properly-aligned, and point to a valid `RawRelPtr`.
        unsafe { Self::offset_raw(this.cast_mut()) }
    }

    /// Gets whether the offset of the relative pointer is invalid.
    pub fn is_invalid(&self) -> bool {
        let this = self as *const Self;
        // SAFETY: `self` is a reference, so it's guaranteed to be non-null,
        // properly-aligned, and point to a valid `RawRelPtr`.
        unsafe { Self::is_invalid_raw(this.cast_mut()) }
    }

    /// Calculates the memory address being pointed to by this relative pointer.
    ///
    /// # Safety
    ///
    /// The offset of this relative pointer, when added to its base, must be
    /// located in the same allocated object as it.
    pub unsafe fn as_ptr(&self) -> *const () {
        let this = self as *const Self;
        // SAFETY:
        // - `self` is a reference, so it's guaranteed to be non-null,
        //   properly-aligned, and point to a valid `RawRelPtr`.
        // - The caller has guaranteed that the offset of this relative pointer,
        //   when added to its base, is located in the same allocated object as
        //   it.
        unsafe { Self::as_ptr_raw(this.cast_mut()).cast_const() }
    }

    /// Calculates the mutable memory address being pointed to by this relative
    /// pointer.
    ///
    /// # Safety
    ///
    /// The offset of this relative pointer, when added to its base, must be
    /// located in the same allocated object as it.
    pub unsafe fn as_mut_ptr(this: Seal<'_, Self>) -> *mut () {
        // SAFETY: The value pointed to by `this` is not moved and no bytes are
        // written through it.
        let this = unsafe { Seal::unseal_unchecked(this) };
        // SAFETY:
        // - `this` is a reference, so it's guaranteed to be non-null,
        //   properly-aligned, and point to a valid `RawRelPtr`.
        // - The caller has guaranteed that the offset of this relative pointer,
        //   when added to its base, is located in the same allocated object as
        //   it.
        unsafe { Self::as_ptr_raw(this as *mut Self) }
    }

    /// Calculates the memory address being pointed to by this relative pointer
    /// using wrapping methods.
    ///
    /// This method is a safer but potentially slower version of `as_ptr`.
    pub fn as_ptr_wrapping(&self) -> *const () {
        let this = self as *const Self;
        // SAFETY: `self` is a reference, so it's guaranteed to be non-null,
        // properly-aligned, and point to a valid `RawRelPtr`.
        unsafe { Self::as_ptr_wrapping_raw(this.cast_mut()).cast_const() }
    }

    /// Calculates the mutable memory address being pointed to by this relative
    /// pointer using wrapping methods.
    ///
    /// This method is a safer but potentially slower version of `as_mut_ptr`.
    pub fn as_mut_ptr_wrapping(this: Seal<'_, Self>) -> *mut () {
        // SAFETY: The value pointed to by `this` is not moved and no bytes are
        // written through it.
        let this = unsafe { Seal::unseal_unchecked(this) };
        // SAFETY: `this` is a reference, so it's guaranteed to be non-null,
        // properly-aligned, and point to a valid `RawRelPtr`.
        unsafe { Self::as_ptr_wrapping_raw(this as *mut Self) }
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
pub type RawRelPtrI32 = RawRelPtr<ArchivedI32>;
/// A raw relative pointer that uses an archived `i64` as the underlying offset.
pub type RawRelPtrI64 = RawRelPtr<ArchivedI64>;

/// A raw relative pointer that uses an archived `u8` as the underlying offset.
pub type RawRelPtrU8 = RawRelPtr<u8>;
/// A raw relative pointer that uses an archived `u16` as the underlying offset.
pub type RawRelPtrU16 = RawRelPtr<ArchivedU16>;
/// A raw relative pointer that uses an archived `u32` as the underlying offset.
pub type RawRelPtrU32 = RawRelPtr<ArchivedU32>;
/// A raw relative pointer that uses an archived `u64` as the underlying offset.
pub type RawRelPtrU64 = RawRelPtr<ArchivedU64>;

/// A pointer which resolves to relative to its position in memory.
///
/// This is a strongly-typed version of [`RawRelPtr`].
///
/// See [`Archive`](crate::Archive) for an example of creating one.
#[derive(Portable)]
#[cfg_attr(feature = "bytecheck", derive(bytecheck::CheckBytes))]
#[rkyv(crate)]
#[repr(C)]
pub struct RelPtr<T: ArchivePointee + ?Sized, O> {
    raw_ptr: RawRelPtr<O>,
    metadata: T::ArchivedMetadata,
    _phantom: PhantomData<T>,
}

impl<T, O: Offset> RelPtr<T, O> {
    /// Attempts to create a relative pointer from one position to another.
    pub fn try_emplace<E: Source>(
        to: usize,
        out: Place<Self>,
    ) -> Result<(), E> {
        munge!(let RelPtr { raw_ptr, metadata: _, _phantom: _ } = out);
        // Skip metadata since sized T is guaranteed to be ()
        RawRelPtr::try_emplace(to, raw_ptr)
    }

    /// Creates a relative pointer from one position to another.
    ///
    /// # Panics
    ///
    /// - If the offset between `from` and `to` does not fit in an `isize`
    /// - If the offset between `from` and `to` exceeds the offset storage
    pub fn emplace(to: usize, out: Place<Self>) {
        Self::try_emplace::<Panic>(to, out).always_ok()
    }
}

impl<T: ArchivePointee + ?Sized, O: Offset> RelPtr<T, O> {
    /// Attempts to create an invalid relative pointer with default metadata.
    pub fn try_emplace_invalid<E: Source>(out: Place<Self>) -> Result<(), E> {
        munge!(let RelPtr { raw_ptr, metadata, _phantom: _ } = out);
        RawRelPtr::try_emplace_invalid(raw_ptr)?;
        metadata.write(Default::default());
        Ok(())
    }

    /// Creates an invalid relative pointer with default metadata.
    ///
    /// # Panics
    ///
    /// - If an offset of `1` does not fit in an `isize`
    /// - If an offset of `1` exceeds the offset storage
    pub fn emplace_invalid(out: Place<Self>) {
        Self::try_emplace_invalid::<Panic>(out).always_ok()
    }

    /// Attempts to create a relative pointer from one position to another.
    pub fn try_emplace_unsized<E: Source>(
        to: usize,
        metadata: T::ArchivedMetadata,
        out: Place<Self>,
    ) -> Result<(), E> {
        munge!(let RelPtr { raw_ptr, metadata: out_meta, _phantom: _ } = out);
        RawRelPtr::try_emplace(to, raw_ptr)?;
        out_meta.write(metadata);
        Ok(())
    }

    /// Creates a relative pointer from one position to another.
    ///
    /// # Panics
    ///
    /// - If the offset between `from` and `to` does not fit in an `isize`
    /// - If the offset between `from` and `to` exceeds the offset storage
    pub fn emplace_unsized(
        to: usize,
        metadata: T::ArchivedMetadata,
        out: Place<Self>,
    ) {
        Self::try_emplace_unsized::<Panic>(to, metadata, out).always_ok()
    }

    /// Gets the base pointer for the pointed-to relative pointer.
    pub fn base_raw(this: *mut Self) -> *mut u8 {
        RawRelPtr::<O>::base_raw(this.cast())
    }

    /// Gets the offset of the pointed-to relative pointer from its base.
    ///
    /// # Safety
    ///
    /// `this` must be non-null, properly-aligned, and point to a valid
    /// `RelPtr`.
    pub unsafe fn offset_raw(this: *mut Self) -> isize {
        // SAFETY: `RelPtr` is `#[repr(C)]`, so the `RawRelPtr` member of the
        // `RelPtr` will have the same address as the `RelPtr`. Because `this`
        // is non-null, properly-aligned, and points to a valid `RelPtr`, a
        // pointer to its first field will also be non-null, properly-aligned,
        // and point to a valid `RawRelPtr`.
        unsafe { RawRelPtr::<O>::offset_raw(this.cast()) }
    }

    /// Calculates the memory address being pointed to by the pointed-to
    /// relative pointer.
    ///
    /// # Safety
    ///
    /// - `this` must be non-null, properly-aligned, and point to a valid
    ///   `RelPtr`.
    /// - The offset of this relative pointer, when added to its base, must be
    ///   located in the same allocated object as it.
    pub unsafe fn as_ptr_raw(this: *mut Self) -> *mut T {
        // SAFETY:
        // - `RelPtr` is `#[repr(C)]`, so the `RawRelPtr` member of the `RelPtr`
        //   will have the same address as the `RelPtr`. Because `this` is
        //   non-null, properly-aligned, and points to a valid `RelPtr`, a
        //   pointer to its first field will also be non-null, properly-aligned,
        //   and point to a valid `RawRelPtr`.
        // - The base and offset of the `RawRelPtr` are guaranteed to be the
        //   same as the base and offset of the `RelPtr`.
        let data_address = unsafe { RawRelPtr::<O>::as_ptr_raw(this.cast()) };
        // SAFETY: The caller has guaranteed that `this` points to a valid
        // `RelPtr`.
        let metadata = unsafe { T::pointer_metadata(&(*this).metadata) };
        ptr_meta::from_raw_parts_mut(data_address, metadata)
    }

    /// Calculates the memory address being pointed to by the pointed-to
    /// relative pointer using wrapping methods.
    ///
    /// This method is a safer but potentially slower version of `as_ptr_raw`.
    ///
    /// # Safety
    ///
    /// `this` must be non-null, properly-aligned, and point to a valid
    /// `RelPtr`.
    pub unsafe fn as_ptr_wrapping_raw(this: *mut Self) -> *mut T {
        // SAFETY: `RelPtr` is `#[repr(C)]`, so the `RawRelPtr` member of the
        // `RelPtr` will have the same address as the `RelPtr`. Because `this`
        // is non-null, properly-aligned, and points to a valid `RelPtr`, a
        // pointer to its first field will also be non-null, properly-aligned,
        // and point to a valid `RawRelPtr`.
        let data_address =
            unsafe { RawRelPtr::<O>::as_ptr_wrapping_raw(this.cast()) };
        // SAFETY: The caller has guaranteed that `this` points to a valid
        // `RelPtr`.
        let metadata = unsafe { T::pointer_metadata(&(*this).metadata) };
        ptr_meta::from_raw_parts_mut(data_address, metadata)
    }

    /// Gets whether the offset of the pointed-to relative pointer is invalid.
    ///
    /// # Safety
    ///
    /// `this` must be non-null, properly-aligned, and point to a valid
    /// `RawRelPtr`.
    pub unsafe fn is_invalid_raw(this: *mut Self) -> bool {
        // SAFETY: `RelPtr` is `#[repr(C)]`, so the `RawRelPtr` member of the
        // `RelPtr` will have the same address as the `RelPtr`. Because `this`
        // is non-null, properly-aligned, and points to a valid `RelPtr`, a
        // pointer to its first field will also be non-null, properly-aligned,
        // and point to a valid `RawRelPtr`.
        unsafe { RawRelPtr::<O>::is_invalid_raw(this.cast()) }
    }

    /// Gets the base pointer for the relative pointer.
    pub fn base(&self) -> *const u8 {
        self.raw_ptr.base()
    }

    /// Gets the mutable base pointer for this relative pointer.
    pub fn base_mut(this: Seal<'_, Self>) -> *mut u8 {
        munge!(let Self { raw_ptr, .. } = this);
        RawRelPtr::base_mut(raw_ptr)
    }

    /// Gets the offset of the relative pointer from its base.
    pub fn offset(&self) -> isize {
        self.raw_ptr.offset()
    }

    /// Gets whether the offset of the relative pointer is 0.
    pub fn is_invalid(&self) -> bool {
        self.raw_ptr.is_invalid()
    }

    /// Gets the metadata of the relative pointer.
    pub fn metadata(&self) -> &T::ArchivedMetadata {
        &self.metadata
    }

    /// Calculates the memory address being pointed to by this relative pointer.
    ///
    /// # Safety
    ///
    /// The offset of this relative pointer, when added to its base, must be
    /// located in the same allocated object as it.
    pub unsafe fn as_ptr(&self) -> *const T {
        ptr_meta::from_raw_parts(
            // SAFETY: The safety requirements for `RawRelPtr::as_ptr` are the
            // same as those for `RelPtr::as_ptr``.
            unsafe { self.raw_ptr.as_ptr() },
            T::pointer_metadata(&self.metadata),
        )
    }

    /// Calculates the mutable memory address being pointed to by this relative
    /// pointer.
    ///
    /// # Safety
    ///
    /// The offset of this relative pointer, when added to its base, must be
    /// located in the same allocated object as it.
    pub unsafe fn as_mut_ptr(this: Seal<'_, Self>) -> *mut T {
        munge!(let Self { raw_ptr, metadata, _phantom: _ } = this);
        let metadata = T::pointer_metadata(&*metadata);
        ptr_meta::from_raw_parts_mut(
            // SAFETY: The safety requirements for `RawRelPtr::as_mut_ptr` are
            // the same as those for `RelPtr::as_mut_ptr``.
            unsafe { RawRelPtr::as_mut_ptr(raw_ptr) },
            metadata,
        )
    }

    /// Calculates the memory address being pointed to by this relative pointer
    /// using wrapping methods.
    ///
    /// This method is a safer but potentially slower version of `as_ptr`.
    pub fn as_ptr_wrapping(&self) -> *const T {
        ptr_meta::from_raw_parts(
            self.raw_ptr.as_ptr_wrapping(),
            T::pointer_metadata(&self.metadata),
        )
    }

    /// Calculates the mutable memory address being pointed to by this relative
    /// pointer using wrapping methods.
    ///
    /// This method is a safer but potentially slower version of `as_ptr`.
    pub fn as_mut_ptr_wrapping(this: Seal<'_, Self>) -> *mut T {
        munge!(let Self { raw_ptr, metadata, _phantom: _ } = this);
        let metadata = T::pointer_metadata(&*metadata);
        ptr_meta::from_raw_parts_mut(
            RawRelPtr::as_mut_ptr_wrapping(raw_ptr),
            metadata,
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
