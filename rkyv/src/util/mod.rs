//! Utilities for common archive operations.
//!
//! ## Buffer access
//!
//! Helper functions to get the root object of an archive under certain
//! conditions.
//!
//! ## Alignment
//!
//! Alignment helpers ensure that byte buffers are properly aligned when
//! accessing and deserializing data.

#[cfg(feature = "alloc")]
mod alloc;
mod inline_vec;
mod ser_vec;

use core::{
    mem,
    ops::{Deref, DerefMut},
    pin::Pin,
};

use rancor::Strategy;

#[doc(inline)]
#[cfg(feature = "alloc")]
pub use self::alloc::*;
#[doc(inline)]
pub use self::{inline_vec::InlineVec, ser_vec::SerVec};
use crate::{ser::Writer, Archive, Deserialize, Portable, Serialize};

#[cfg(debug_assertions)]
#[inline]
fn check_alignment<T: Portable>(ptr: *const u8) {
    let expect_align = core::mem::align_of::<T>();
    let actual_align = (ptr as usize) & (expect_align - 1);
    debug_assert_eq!(
        actual_align,
        0,
        concat!(
            "unaligned buffer, expected alignment {} but found alignment {}\n",
            "help: rkyv requires byte buffers to be aligned to access the \
             data inside.\n",
            "      Using an AlignedVec or manually aligning your data with \
             #[align(...)]\n",
            "      may resolve this issue.",
        ),
        expect_align,
        1 << actual_align.trailing_zeros()
    );
}

/// Accesses an archived value from the given byte slice at the given position.
///
/// This function does not check that the data at the given position is valid.
/// Use [`access_pos`](crate::validation::util::access_pos) to validate the data
/// instead.
///
/// # Safety
///
/// A valid `T` must be located at the given position in the byte slice.
#[inline]
pub unsafe fn access_pos_unchecked<T: Portable>(
    bytes: &[u8],
    pos: usize,
) -> &T {
    #[cfg(debug_assertions)]
    check_alignment::<T>(bytes.as_ptr());

    // SAFETY: The caller has guaranteed that a valid `T` is located at `pos` in
    // the byte slice.
    unsafe { &*bytes.as_ptr().add(pos).cast() }
}

/// Accesses a mutable archived value from the given byte slice at the given
/// position.
///
/// This function does not check that the data at the given position is valid.
/// Use [`access_pos_mut`](crate::validation::util::access_pos_mut) to validate
/// the data instead.
///
/// # Safety
///
/// A `T` must be located at the given position in the byte slice.
#[inline]
pub unsafe fn access_pos_unchecked_mut<T: Portable>(
    bytes: &mut [u8],
    pos: usize,
) -> Pin<&mut T> {
    #[cfg(debug_assertions)]
    check_alignment::<T>(bytes.as_ptr());

    // SAFETY: The caller has guaranteed that a valid `T` is located at `pos` in
    // the byte slice. WARNING: This is a technically incorrect use of the
    // pinning API because we do not guarantee that the destructor for `T` will
    // run before the backing memory is reused!
    unsafe { Pin::new_unchecked(&mut *bytes.as_mut_ptr().add(pos).cast()) }
}

/// Accesses an archived value from the given byte slice by calculating the root
/// position.
///
/// This is a wrapper for [`access_pos_unchecked`] that calculates the position
/// of the root object using the length of the byte slice. If your byte slice is
/// not guaranteed to end immediately after the root object, you may need to
/// store the position of the root object returned from
/// [`serialize_and_resolve`](crate::Serialize::serialize_and_resolve).
///
/// # Safety
///
/// - The byte slice must represent an archived object.
/// - The root of the object must be stored at the end of the slice (this is the
///   default behavior).
#[inline]
pub unsafe fn access_unchecked<T: Portable>(bytes: &[u8]) -> &T {
    // SAFETY: The caller has guaranteed that a valid `T` is located at the root
    // position in the byte slice.
    unsafe {
        access_pos_unchecked::<T>(bytes, bytes.len() - mem::size_of::<T>())
    }
}

/// Accesses a mutable archived value from the given byte slice by calculating
/// the root position.
///
/// This is a wrapper for [`access_pos_unchecked_mut`] that calculates the
/// position of the root object using the length of the byte slice. If your byte
/// slice is not guaranteed to end immediately after the root object, you may
/// need to store the position of the root object returned from
/// [`serialize_and_resolve`](crate::Serialize::serialize_and_resolve).
///
/// # Safety
///
/// - The byte slice must represent an archived object.
/// - The root of the object must be stored at the end of the slice (this is the
///   default behavior).
#[inline]
pub unsafe fn access_unchecked_mut<T: Portable>(
    bytes: &mut [u8],
) -> Pin<&mut T> {
    let pos = bytes.len() - mem::size_of::<T>();
    // SAFETY: The caller has guaranteed that a valid `T` is located at the root
    // position in the byte slice.
    unsafe { access_pos_unchecked_mut::<T>(bytes, pos) }
}

/// A wrapper which aligns its inner value to 16 bytes.
#[derive(Clone, Copy, Debug)]
#[repr(C, align(16))]
pub struct Align<T>(
    /// The inner value.
    pub T,
);

impl<T> Deref for Align<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Align<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Serializes the given value into the given serializer and then returns the
/// serializer.
#[inline]
pub fn serialize_into<S, E>(
    value: &impl Serialize<Strategy<S, E>>,
    mut serializer: S,
) -> Result<S, E>
where
    S: Writer<E>,
{
    serialize(value, &mut serializer)?;
    Ok(serializer)
}

/// Serializes the given value into the given serializer.
#[inline]
pub fn serialize<S, E>(
    value: &impl Serialize<Strategy<S, E>>,
    serializer: &mut S,
) -> Result<(), E>
where
    S: Writer<E> + ?Sized,
{
    value.serialize_and_resolve(Strategy::wrap(serializer))?;
    Ok(())
}

/// Deserailizes a value from the given archived value using the provided
/// deserializer.
#[inline]
pub fn deserialize<T, D, E>(
    value: &T::Archived,
    deserializer: &mut D,
) -> Result<T, E>
where
    T: Archive,
    T::Archived: Deserialize<T, Strategy<D, E>>,
{
    value.deserialize(Strategy::wrap(deserializer))
}
