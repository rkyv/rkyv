//! APIs for producing and using archived data safely.

use bytecheck::CheckBytes;
use ptr_meta::Pointee;
use rancor::{Source, Strategy};

use crate::{
    api::{access_pos_unchecked, access_pos_unchecked_mut, root_position},
    seal::Seal,
    validation::{ArchiveContext, ArchiveContextExt},
    Portable,
};

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
        root_position::<T>(bytes.len()),
        context,
    )
}

/// Mutably accesses an archived value from the given byte slice at the given
/// position after checking its validity with the given context.
///
/// This is a safe alternative to [`access_pos_unchecked_mut`].
pub fn access_pos_with_context_mut<'a, T, C, E>(
    bytes: &'a mut [u8],
    pos: usize,
    context: &mut C,
) -> Result<Seal<'a, T>, E>
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
) -> Result<Seal<'a, T>, E>
where
    T: Portable + CheckBytes<Strategy<C, E>> + Pointee<Metadata = ()>,
    C: ArchiveContext<E> + ?Sized,
    E: Source,
{
    access_pos_with_context_mut::<T, C, E>(
        bytes,
        root_position::<T>(bytes.len()),
        context,
    )
}
