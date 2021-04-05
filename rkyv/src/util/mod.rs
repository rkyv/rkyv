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
#[doc(hidden)]
pub mod std;

use crate::{Archive, ArchiveUnsized, RelPtr};
use core::{
    mem,
    ops::{Deref, DerefMut},
    pin::Pin,
};

#[cfg(feature = "std")]
#[doc(inline)]
pub use self::std::*;

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
    crate::archived_value::<T>(bytes, bytes.len() - mem::size_of::<T::Archived>())
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
    crate::archived_value_mut::<T>(bytes, pos)
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
    crate::archived_unsized_value::<T>(bytes, bytes.len() - mem::size_of::<RelPtr<T::Archived>>())
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
    crate::archived_unsized_value_mut::<T>(bytes, pos)
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
