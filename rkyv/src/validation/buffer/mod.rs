//! Functions for accessing and deserializing buffers safely.

#[cfg(feature = "alloc")]
mod alloc;

use core::{mem::size_of, pin::Pin};

use bytecheck::CheckBytes;
use ptr_meta::Pointee;
use rancor::{Source, Strategy};

#[cfg(feature = "alloc")]
pub use self::alloc::*;
use crate::{
    buffer::{access_pos_unchecked, access_pos_unchecked_mut},
    validation::{ArchiveContext, ArchiveContextExt},
    Portable,
};

fn root_position<T: Portable>(bytes: &[u8]) -> usize {
    bytes.len().saturating_sub(size_of::<T>())
}

/// Checks a byte slice for a valid instance of the given archived type at the
/// given position with the given context.
pub fn check_pos_with_context<T, C, E>(
    bytes: &[u8],
    pos: usize,
    context: &mut C,
) -> Result<(), E>
where
    T: CheckBytes<Strategy<C, E>> + Pointee<Metadata = ()>,
    C: ArchiveContext<E> + ?Sized,
    E: Source,
{
    let context = Strategy::<C, E>::wrap(context);
    let ptr = bytes.as_ptr().wrapping_add(pos).cast::<T>();
    context.in_subtree(ptr, |context| {
        // SAFETY: `in_subtree` has guaranteed that `ptr` is properly aligned
        // and points to enough bytes for a `T`.
        unsafe { T::check_bytes(ptr, context) }
    })
}

/// Accesses an archived value from the given byte slice at the given position
/// after checking its validity with the given context.
///
/// This is a safe alternative to [`access_pos_unchecked`].
pub fn access_pos_with_context<'a, T, C, E>(
    bytes: &'a [u8],
    pos: usize,
    context: &mut C,
) -> Result<&'a T, E>
where
    T: Portable + CheckBytes<Strategy<C, E>> + Pointee<Metadata = ()>,
    C: ArchiveContext<E> + ?Sized,
    E: Source,
{
    check_pos_with_context::<T, C, E>(bytes, pos, context)?;
    unsafe { Ok(access_pos_unchecked::<T>(bytes, pos)) }
}

/// Accesses an archived value from the given byte slice by calculating the root
/// position after checking its validity with the given context.
///
/// This is a safe alternative to [`access_unchecked`][unsafe_version].
///
/// [unsafe_version]: crate::access_unchecked
pub fn access_with_context<'a, T, C, E>(
    bytes: &'a [u8],
    context: &mut C,
) -> Result<&'a T, E>
where
    T: Portable + CheckBytes<Strategy<C, E>> + Pointee<Metadata = ()>,
    C: ArchiveContext<E> + ?Sized,
    E: Source,
{
    access_pos_with_context::<T, C, E>(
        bytes,
        root_position::<T>(bytes),
        context,
    )
}

// TODO(#516): `Pin` is not technically correct for the return type. `Pin`
// requires the pinned value to be dropped before its memory can be reused, but
// archived types explicitly do not require that. It just wants immovable types.

// TODO: `bytes` may no longer be a fully-initialized `[u8]` after mutable
// operations. We really need some kind of opaque byte container for these
// operations.

/// Mutably accesses an archived value from the given byte slice at the given
/// position after checking its validity with the given context.
///
/// This is a safe alternative to [`access_pos_unchecked_mut`].
pub fn access_pos_with_context_mut<'a, T, C, E>(
    bytes: &'a mut [u8],
    pos: usize,
    context: &mut C,
) -> Result<Pin<&'a mut T>, E>
where
    T: Portable + CheckBytes<Strategy<C, E>> + Pointee<Metadata = ()>,
    C: ArchiveContext<E> + ?Sized,
    E: Source,
{
    check_pos_with_context::<T, C, E>(bytes, pos, context)?;
    unsafe { Ok(access_pos_unchecked_mut::<T>(bytes, pos)) }
}

/// Mutably accesses an archived value from the given byte slice by calculating
/// the root position after checking its validity with the given context.
///
/// This is a safe alternative to [`access_unchecked_mut`][unsafe_version].
///
/// [unsafe_version]: crate::access_unchecked_mut
pub fn access_with_context_mut<'a, T, C, E>(
    bytes: &'a mut [u8],
    context: &mut C,
) -> Result<Pin<&'a mut T>, E>
where
    T: Portable + CheckBytes<Strategy<C, E>> + Pointee<Metadata = ()>,
    C: ArchiveContext<E> + ?Sized,
    E: Source,
{
    access_pos_with_context_mut::<T, C, E>(
        bytes,
        root_position::<T>(bytes),
        context,
    )
}
