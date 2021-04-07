//! Utilities for common archive operations.
//!
//! ## Buffer access
//!
//! Helper functions to get the root object of an archive under certain conditions.
//!
//! ## Alignment
//!
//! Alignment helpers ensure that byte buffers are properly aligned when accessing and deserializing
//! data.

#[cfg(feature = "std")]
mod std;

use crate::{Archive, ArchiveUnsized, RelPtr};
use core::{
    mem,
    ops::{Deref, DerefMut},
    pin::Pin,
};

#[doc(inline)]
#[cfg(feature = "std")]
pub use self::std::*;

#[cfg(debug_assertions)]
#[inline]
fn check_alignment<T>(ptr: *const u8) {
    let expect_align = core::mem::align_of::<T>();
    let actual_align = (ptr as usize) & (expect_align - 1);
    debug_assert_eq!(
        actual_align,
        0,
        "unaligned buffer, expected alignment {} but found alignment {}",
        expect_align,
        1 << actual_align
    );
}

/// Casts an archived value from the given byte slice at the given position.
///
/// This helps avoid situations where lifetimes get inappropriately assigned and allow buffer
/// mutation after getting archived value references.
///
/// # Safety
///
/// The caller must guarantee that a value is archived at the given position in the byte slice.
#[inline]
pub unsafe fn archived_value<T: Archive + ?Sized>(bytes: &[u8], pos: usize) -> &T::Archived {
    #[cfg(debug_assertions)]
    check_alignment::<T::Archived>(bytes.as_ptr());

    &*bytes.as_ptr().add(pos).cast()
}

/// Casts a mutable archived value from the given byte slice at the given position.
///
/// This helps avoid situations where lifetimes get inappropriately assigned and allow buffer
/// mutation after getting archived value references.
///
/// # Safety
///
/// The caller must guarantee that a value is archived at the given position in the byte slice.
#[inline]
pub unsafe fn archived_value_mut<T: Archive + ?Sized>(
    bytes: Pin<&mut [u8]>,
    pos: usize,
) -> Pin<&mut T::Archived> {
    #[cfg(debug_assertions)]
    check_alignment::<T::Archived>(bytes.as_ptr());

    Pin::new_unchecked(&mut *bytes.get_unchecked_mut().as_mut_ptr().add(pos).cast())
}

/// Casts a [`RelPtr`] to the given unsized type from the given byte slice at the given position and
/// returns the value it points to.
///
/// This helps avoid situations where lifetimes get inappropriately assigned and allow buffer
/// mutation after getting archived value references.
///
/// # Safety
///
/// The caller must guarantee that a reference is archived at the given position in the byte slice.
#[inline]
pub unsafe fn archived_unsized_value<T: ArchiveUnsized + ?Sized>(
    bytes: &[u8],
    pos: usize,
) -> &T::Archived {
    #[cfg(debug_assertions)]
    check_alignment::<RelPtr<T::Archived>>(bytes.as_ptr());

    let rel_ptr = &*bytes.as_ptr().add(pos).cast::<RelPtr<T::Archived>>();
    &*rel_ptr.as_ptr()
}

/// Casts a mutable [`RelPtr`] to the given unsized type from the given byte slice at the given
/// position and returns the value it points to.
///
/// This helps avoid situations where lifetimes get inappropriately assigned and allow buffer
/// mutation after getting archived value references.
///
/// # Safety
///
/// The caller must guarantee that a reference is archived at the given position in the byte slice.
#[inline]
pub unsafe fn archived_unsized_value_mut<T: ArchiveUnsized + ?Sized>(
    bytes: Pin<&mut [u8]>,
    pos: usize,
) -> Pin<&mut T::Archived> {
    #[cfg(debug_assertions)]
    check_alignment::<RelPtr<T::Archived>>(bytes.as_ptr());

    let rel_ptr = &mut *bytes
        .get_unchecked_mut()
        .as_mut_ptr()
        .add(pos)
        .cast::<RelPtr<T::Archived>>();
    Pin::new_unchecked(&mut *rel_ptr.as_mut_ptr())
}

/// Casts an archived value from the given byte slice by calculating the root position.
///
/// This is a wrapper for [`archived_value`](crate::archived_value) that calculates the correct
/// position of the root using the length of the byte slice. If your byte slice is not guaranteed to
/// end immediately after the root object, you may need to store the position of the root object or
/// use [`serialize_front`](crate::ser::SeekSerializer::serialize_front) instead.
///
/// # Safety
///
/// The caller must guarantee that the byte slice represents an archived object and that the root
/// object is stored at the end of the byte slice.
#[inline]
pub unsafe fn archived_root<T: Archive + ?Sized>(bytes: &[u8]) -> &T::Archived {
    archived_value::<T>(bytes, bytes.len() - mem::size_of::<T::Archived>())
}

/// Casts a mutable archived value from the given byte slice by calculating the root position.
///
/// This is a wrapper for [`archived_value_mut`](crate::archived_value_mut) that calculates the
/// correct position of the root using the length of the byte slice. If your byte slice is not
/// guaranteed to end immediately after the root object, you may need to store the position of the
/// root object or use [`serialize_front`](crate::ser::SeekSerializer::serialize_front) instead.
///
/// # Safety
///
/// The caller must guarantee that the byte slice represents an archived object and that the root
/// object is stored at the end of the byte slice.
#[inline]
pub unsafe fn archived_root_mut<T: Archive + ?Sized>(
    bytes: Pin<&mut [u8]>,
) -> Pin<&mut T::Archived> {
    let pos = bytes.len() - mem::size_of::<T::Archived>();
    archived_value_mut::<T>(bytes, pos)
}

/// Casts a [`RelPtr`] to the given unsized type from the given byte slice by calculating the root
/// position.
///
/// This is a wrapper for [`archived_unsized_value`](crate::archived_unsized_value) that calculates
/// the correct position of the root using the length of the byte slice. If your byte slice is not
/// guaranteed to end immediately after the root object, you may need to store the position of the
/// root object or use [`serialize_front`](crate::ser::SeekSerializer::serialize_front) instead.
///
/// # Safety
///
/// The caller must guarantee that the byte slice represents an archived object and that the root
/// object is stored at the end of the byte slice.
#[inline]
pub unsafe fn archived_unsized_root<T: ArchiveUnsized + ?Sized>(bytes: &[u8]) -> &T::Archived {
    archived_unsized_value::<T>(bytes, bytes.len() - mem::size_of::<RelPtr<T::Archived>>())
}

/// Casts a [`RelPtr`] to the given unsized type from the given byte slice by calculating the root
/// position.
///
/// This is a wrapper for [`archived_unsized_value_mut`](crate::archived_unsized_value_mut) that
/// calculates the correct position of the root using the length of the byte slice. If your byte
/// slice is not guaranteed to end immediately after the root object, you may need to store the
/// position of the root object or use
/// [`serialize_front`](crate::ser::SeekSerializer::serialize_front) instead.
///
/// # Safety
///
/// The caller must guarantee that the byte slice represents an archived object and that the root
/// object is stored at the end of the byte slice.
#[inline]
pub unsafe fn archived_unsized_root_mut<T: ArchiveUnsized + ?Sized>(
    bytes: Pin<&mut [u8]>,
) -> Pin<&mut T::Archived> {
    let pos = bytes.len() - mem::size_of::<RelPtr<T::Archived>>();
    archived_unsized_value_mut::<T>(bytes, pos)
}

/// Wraps a type and aligns it to 16 bytes.
///
/// ## Examples
/// ```
/// use core::mem;
/// use rkyv::Aligned;
///
/// assert_eq!(mem::align_of::<u8>(), 1);
/// assert_eq!(mem::align_of::<Aligned<u8>>(), 16);
/// ```
#[derive(Clone, Copy)]
#[repr(align(16))]
pub struct Aligned<T>(pub T);

impl<T: Deref> Deref for Aligned<T> {
    type Target = T::Target;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl<T: DerefMut> DerefMut for Aligned<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.0
    }
}

impl<T: AsRef<[U]>, U> AsRef<[U]> for Aligned<T> {
    #[inline]
    fn as_ref(&self) -> &[U] {
        self.0.as_ref()
    }
}

impl<T: AsMut<[U]>, U> AsMut<[U]> for Aligned<T> {
    #[inline]
    fn as_mut(&mut self) -> &mut [U] {
        self.0.as_mut()
    }
}
