//! Utility methods for accessing and deserializing safely.

use core::{mem::size_of, pin::Pin};

use bytecheck::CheckBytes;
use ptr_meta::Pointee;
use rancor::{Source, Strategy};

use crate::{
    de::pooling::Pool,
    deserialize,
    util::{access_pos_unchecked, access_pos_unchecked_mut},
    validation::{
        validators::DefaultValidator, ArchiveContext, ArchiveContextExt,
    },
    Archive, Deserialize, Portable,
};

#[inline]
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

// TODO: Either this should be unsafe or there must be some invariant that
// `check_pos_with_context` verifies that the position is dereferenceable
// regardless of what context was used to verify it.

/// Accesses an archived value from the given byte slice at the given position
/// after checking its validity with the given context.
///
/// This is a safe alternative to [`access_pos_unchecked`].
#[inline]
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
#[inline]
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

/// Accesses an archived value from the given byte slice at the given position
/// after checking its validity.
///
/// This is a safe alternative to [`access_pos_unchecked`].
#[inline]
pub fn access_pos<T, E>(bytes: &[u8], pos: usize) -> Result<&T, E>
where
    T: Portable + for<'a> CheckBytes<Strategy<DefaultValidator<'a>, E>>,
    E: Source,
{
    let mut validator = DefaultValidator::new(bytes);
    access_pos_with_context::<T, DefaultValidator, E>(
        bytes,
        pos,
        &mut validator,
    )
}

/// Accesses an archived value from the given byte slice by calculating the root
/// position after checking its validity.
///
/// This is a safe alternative to [`access_unchecked`][unsafe_version].
///
/// [unsafe_version]: crate::access_unchecked
///
/// # Examples
/// ```
/// use rkyv::{
///     access, bytecheck::CheckBytes, rancor::Error, to_bytes, Archive,
///     Archived, Serialize,
/// };
///
/// #[derive(Archive, Serialize)]
/// #[archive_attr(derive(CheckBytes))]
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
/// let bytes = to_bytes::<Error>(&value).unwrap();
/// let archived = access::<Archived<Example>, Error>(&bytes).unwrap();
///
/// assert_eq!(archived.name, "pi");
/// assert_eq!(archived.value, 31415926);
/// ```
#[inline]
pub fn access<T, E>(bytes: &[u8]) -> Result<&T, E>
where
    T: Portable + for<'a> CheckBytes<Strategy<DefaultValidator<'a>, E>>,
    E: Source,
{
    let mut validator = DefaultValidator::new(bytes);
    access_with_context::<T, DefaultValidator, E>(bytes, &mut validator)
}

// TODO: `Pin` is not technically correct for the return type. `Pin` requires
// the pinned value to be dropped before its memory can be reused, but archived
// types explicitly do not require that. It just wants immovable types.

// TODO: `bytes` may no longer be a fully-initialized `[u8]` after mutable
// operations. We really need some kind of opaque byte container for these
// operations.

/// Mutably accesses an archived value from the given byte slice at the given
/// position after checking its validity with the given context.
///
/// This is a safe alternative to [`access_pos_unchecked_mut`].
#[inline]
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
#[inline]
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

/// Mutably accesses an archived value from the given byte slice at the given
/// position after checking its validity.
///
/// This is a safe alternative to [`access_pos_unchecked`].
#[inline]
pub fn access_pos_mut<T, E>(
    bytes: &mut [u8],
    pos: usize,
) -> Result<Pin<&mut T>, E>
where
    T: Portable + for<'a> CheckBytes<Strategy<DefaultValidator<'a>, E>>,
    E: Source,
{
    let mut context = DefaultValidator::new(bytes);
    check_pos_with_context::<T, _, E>(bytes, pos, &mut context)?;
    unsafe { Ok(access_pos_unchecked_mut::<T>(bytes, pos)) }
}

/// Mutably accesses an archived value from the given byte slice by calculating
/// the root position after checking its validity.
///
/// This is a safe alternative to [`access_unchecked`][unsafe_version].
///
/// [unsafe_version]: crate::access_unchecked
#[inline]
pub fn access_mut<T, E>(bytes: &mut [u8]) -> Result<Pin<&mut T>, E>
where
    T: Portable + for<'a> CheckBytes<Strategy<DefaultValidator<'a>, E>>,
    E: Source,
{
    let mut context = DefaultValidator::new(bytes);
    let pos = root_position::<T>(bytes);
    check_pos_with_context::<T, _, E>(bytes, pos, &mut context)?;
    unsafe { Ok(access_pos_unchecked_mut::<T>(bytes, pos)) }
}

/// Checks and deserializes a value from the given bytes.
///
/// This function is only available with the `alloc` and `validation` features
/// because it uses a general-purpose deserializer and performs validation on
/// the data before deserializing. In no-alloc and high-performance
/// environments, the deserializer should be customized for the specific
/// situation.
///
/// This is a safe alternative to [`from_bytes_unchecked`][unsafe_version].
///
/// [unsafe_version]: crate::from_bytes_unchecked
///
/// # Examples
/// ```
/// use rkyv::rancor::Error;
///
/// let value = vec![1, 2, 3, 4];
///
/// let bytes =
///     rkyv::to_bytes::<Error>(&value).expect("failed to serialize vec");
/// let deserialized = rkyv::from_bytes::<Vec<i32>, Error>(&bytes)
///     .expect("failed to deserialize vec");
///
/// assert_eq!(deserialized, value);
/// ```
#[inline]
pub fn from_bytes<T, E>(bytes: &[u8]) -> Result<T, E>
where
    T: Archive,
    T::Archived: for<'a> CheckBytes<Strategy<DefaultValidator<'a>, E>>
        + Deserialize<T, Strategy<Pool, E>>,
    E: Source,
{
    let mut deserializer = Pool::default();
    deserialize(access::<T::Archived, E>(bytes)?, &mut deserializer)
}
