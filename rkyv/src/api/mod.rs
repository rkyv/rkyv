//! APIs for producing and using archived data.

#[cfg(feature = "bytecheck")]
mod checked;
#[cfg(feature = "alloc")]
pub mod high;
pub mod low;
#[cfg(test)]
pub mod test;

use core::mem::size_of;

use rancor::Strategy;

#[cfg(feature = "bytecheck")]
pub use self::checked::*;
use crate::{seal::Seal, ser::Writer, Deserialize, Portable, SerializeUnsized};

#[cfg(debug_assertions)]
fn sanity_check_buffer<T: Portable>(ptr: *const u8, pos: usize, size: usize) {
    use core::mem::{align_of, size_of};

    let root_size = size_of::<T>();
    let min_size = pos + root_size;
    debug_assert!(
        min_size <= size,
        concat!(
            "buffer too small, expected at least {} bytes but found {} bytes\n",
            "help: the root type at offset {} requires at least {} bytes",
        ),
        min_size,
        size,
        pos,
        root_size,
    );
    let expect_align = align_of::<T>();
    let actual_align = (ptr as usize) & (expect_align - 1);
    debug_assert_eq!(
        actual_align,
        0,
        concat!(
            "unaligned buffer, expected alignment {} but found alignment {}\n",
            "help: rkyv requires byte buffers to be aligned to access the \
             data inside.\n",
            "      Using an AlignedVec or manually aligning your data with \
             `#[align(...)]` may resolve this issue.\n",
            "      Alternatively, you may enable the `unaligned` feature to \
             relax the alignment requirements for your archived data.\n",
            "      `unaligned` is a format control feature, and enabling it \
             may change the format of your serialized data)",
        ),
        expect_align,
        1 << actual_align.trailing_zeros()
    );
}

/// Returns the position of the root within a buffer of `length` bytes.
///
/// If the buffer size is too small to accomodate a root of the given type, then
/// the root position will be zero.
///
/// This is called by [`access_unchecked`] to calculate the root position.
pub fn root_position<T: Portable>(size: usize) -> usize {
    size.saturating_sub(size_of::<T>())
}

/// Accesses an archived value from the given byte slice at the given position.
///
/// This function does not check that the data at the given position is valid.
/// Use [`access_pos`] to validate the data instead.
///
/// # Safety
///
/// The given bytes must pass validation at the given position when passed to
/// [`access_pos`].
///
/// [`access_pos`]: crate::api::high::access_pos
pub unsafe fn access_pos_unchecked<T: Portable>(
    bytes: &[u8],
    pos: usize,
) -> &T {
    #[cfg(debug_assertions)]
    sanity_check_buffer::<T>(bytes.as_ptr(), pos, bytes.len());

    // SAFETY: The caller has guaranteed that a valid `T` is located at `pos` in
    // the byte slice.
    unsafe { &*bytes.as_ptr().add(pos).cast() }
}

/// Accesses a mutable archived value from the given byte slice at the given
/// position.
///
/// This function does not check that the data at the given position is valid.
/// Use [`access_pos_mut`] to validate the data instead.
///
/// # Safety
///
/// The given bytes must pass validation at the given position when passed to
/// [`access_pos_mut`].
///
/// [`access_pos_mut`]: crate::api::high::access_pos_mut
pub unsafe fn access_pos_unchecked_mut<T: Portable>(
    bytes: &mut [u8],
    pos: usize,
) -> Seal<'_, T> {
    #[cfg(debug_assertions)]
    sanity_check_buffer::<T>(bytes.as_ptr(), pos, bytes.len());

    // SAFETY: The caller has guaranteed that the data at the given position
    // passes validation when passed to `access_pos_mut`.
    unsafe { Seal::new(&mut *bytes.as_mut_ptr().add(pos).cast()) }
}

/// Accesses an archived value from the given byte slice by calculating the root
/// position.
///
/// This function does not check that the data is valid. Use [`access`] to
/// validate the data instead.
///
/// This is a wrapper for [`access_pos_unchecked`] that calculates the position
/// of the root object using the length of the byte slice. If your byte slice is
/// not guaranteed to end immediately after the root object, you may need to
/// store the position of the root object and call [`access_pos_unchecked`]
/// directly.
///
/// # Safety
///
/// The given bytes must pass validation when passed to [`access`].
///
/// [`access`]: crate::api::high::access
pub unsafe fn access_unchecked<T: Portable>(bytes: &[u8]) -> &T {
    // SAFETY: The caller has guaranteed that a valid `T` is located at the root
    // position in the byte slice.
    unsafe { access_pos_unchecked::<T>(bytes, root_position::<T>(bytes.len())) }
}

/// Accesses a mutable archived value from the given byte slice by calculating
/// the root position.
///
/// This function does not check that the data is valid. Use [`access_mut`] to
/// validate the data instead.
///
/// This is a wrapper for [`access_pos_unchecked_mut`] that calculates the
/// position of the root object using the length of the byte slice. If your byte
/// slice is not guaranteed to end immediately after the root object, you may
/// need to store the position of the root object and call
/// [`access_pos_unchecked_mut`] directly.
///
/// # Safety
///
/// The given bytes must pass validation when passed to [`access_mut`].
///
/// [`access_mut`]: crate::api::high::access_mut
pub unsafe fn access_unchecked_mut<T: Portable>(
    bytes: &mut [u8],
) -> Seal<'_, T> {
    // SAFETY: The caller has guaranteed that the given bytes pass validation
    // when passed to `access_mut`.
    unsafe {
        access_pos_unchecked_mut::<T>(bytes, root_position::<T>(bytes.len()))
    }
}

/// Serializes the given value using the given serializer.
pub fn serialize_using<S, E>(
    value: &impl SerializeUnsized<Strategy<S, E>>,
    serializer: &mut S,
) -> Result<usize, E>
where
    S: Writer<E> + ?Sized,
{
    value.serialize_unsized(Strategy::wrap(serializer))
}

/// Deserializes a value from the given archived value using the provided
/// deserializer.
pub fn deserialize_using<T, D, E>(
    value: &impl Deserialize<T, Strategy<D, E>>,
    deserializer: &mut D,
) -> Result<T, E> {
    value.deserialize(Strategy::wrap(deserializer))
}
