//! Functions for accessing and deserializing buffers safely.

use core::pin::Pin;

use bytecheck::CheckBytes;
use rancor::{Source, Strategy};

use crate::{
    buffer::access_pos_unchecked_mut,
    de::pooling::Pool,
    deserialize,
    validation::{
        buffer::{
            access_pos_with_context, access_with_context,
            check_pos_with_context, root_position,
        },
        validators::DefaultValidator,
    },
    Archive, Deserialize, Portable,
};

/// Accesses an archived value from the given byte slice at the given position
/// after checking its validity.
///
/// This is a safe alternative to
/// [`access_pos_unchecked`](crate::validation::buffer::access_pos_unchecked).
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
/// #[rkyv(check_bytes)]
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
pub fn access<T, E>(bytes: &[u8]) -> Result<&T, E>
where
    T: Portable + for<'a> CheckBytes<Strategy<DefaultValidator<'a>, E>>,
    E: Source,
{
    let mut validator = DefaultValidator::new(bytes);
    access_with_context::<T, DefaultValidator, E>(bytes, &mut validator)
}

// TODO(#516): `Pin` is not technically correct for the return type. `Pin`
// requires the pinned value to be dropped before its memory can be reused, but
// archived types explicitly do not require that. It just wants immovable types.

// TODO: `bytes` may no longer be a fully-initialized `[u8]` after mutable
// operations. We really need some kind of opaque byte container for these
// operations.

/// Mutably accesses an archived value from the given byte slice at the given
/// position after checking its validity.
///
/// This is a safe alternative to
/// [`access_pos_unchecked`](crate::validation::buffer::access_pos_unchecked).
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
