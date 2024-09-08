//! High-level checked APIs.
//!
//! These APIs support shared pointers.

use bytecheck::CheckBytes;
use rancor::{Source, Strategy};

use crate::{
    api::{
        access_pos_unchecked_mut, access_pos_with_context, access_with_context,
        check_pos_with_context, deserialize_with, root_position,
    },
    de::pooling::Pool,
    seal::Seal,
    validation::{
        archive::ArchiveValidator, shared::SharedValidator, Validator,
    },
    Archive, Deserialize, Portable,
};

/// A high-level validator.
///
/// This is part of the [high-level API](crate::api::high).
pub type HighValidator<'a, E> =
    Strategy<Validator<ArchiveValidator<'a>, SharedValidator>, E>;

fn validator(bytes: &[u8]) -> Validator<ArchiveValidator<'_>, SharedValidator> {
    Validator::new(ArchiveValidator::new(bytes), SharedValidator::new())
}

/// Accesses an archived value from the given byte slice at the given position
/// after checking its validity.
///
/// This is a safe alternative to
/// [`access_pos_unchecked`](crate::api::access_pos_unchecked) and is part of
/// the [high-level API](crate::api::high).
pub fn access_pos<T, E>(bytes: &[u8], pos: usize) -> Result<&T, E>
where
    T: Portable + for<'a> CheckBytes<HighValidator<'a, E>>,
    E: Source,
{
    access_pos_with_context::<_, _, E>(bytes, pos, &mut validator(bytes))
}

/// Accesses an archived value from the given byte slice by calculating the root
/// position after checking its validity.
///
/// This is a safe alternative to
/// [`access_unchecked`](crate::api::access_unchecked) and is part of the
/// [high-level API](crate::api::high).
///
/// # Examples
/// ```
/// use rkyv::{
///     access, bytecheck::CheckBytes, rancor::Error, to_bytes, Archive,
///     Archived, Serialize,
/// };
///
/// #[derive(Archive, Serialize)]
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
    T: Portable + for<'a> CheckBytes<HighValidator<'a, E>>,
    E: Source,
{
    access_with_context::<_, _, E>(bytes, &mut validator(bytes))
}

/// Mutably accesses an archived value from the given byte slice at the given
/// position after checking its validity.
///
/// This is a safe alternative to
/// [`access_pos_unchecked`](crate::api::access_pos_unchecked) and is part of
/// the [high-level API](crate::api::high).
pub fn access_pos_mut<T, E>(
    bytes: &mut [u8],
    pos: usize,
) -> Result<Seal<'_, T>, E>
where
    T: Portable + for<'a> CheckBytes<HighValidator<'a, E>>,
    E: Source,
{
    let mut context = validator(bytes);
    check_pos_with_context::<T, _, E>(bytes, pos, &mut context)?;
    unsafe { Ok(access_pos_unchecked_mut::<T>(bytes, pos)) }
}

/// Mutably accesses an archived value from the given byte slice by calculating
/// the root position after checking its validity.
///
/// This is a safe alternative to
/// [`access_unchecked`](crate::api::access_unchecked) and is part of the
/// [high-level API](crate::api::high).
pub fn access_mut<T, E>(bytes: &mut [u8]) -> Result<Seal<'_, T>, E>
where
    T: Portable + for<'a> CheckBytes<HighValidator<'a, E>>,
    E: Source,
{
    let mut context = validator(bytes);
    let pos = root_position::<T>(bytes.len());
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
/// This is a safe alternative to
/// [`from_bytes_unchecked`](crate::api::high::from_bytes_unchecked) and is part
/// of the [high-level API](crate::api::high).
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
    T::Archived: for<'a> CheckBytes<HighValidator<'a, E>>
        + Deserialize<T, Strategy<Pool, E>>,
    E: Source,
{
    let mut deserializer = Pool::default();
    deserialize_with(access::<T::Archived, E>(bytes)?, &mut deserializer)
}
