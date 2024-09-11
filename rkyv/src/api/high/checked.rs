//! High-level checked APIs.
//!
//! These APIs have default writers, automatically manage allocators, and
//! support shared pointers.

use bytecheck::CheckBytes;
use rancor::{Source, Strategy};

use crate::{
    api::{
        access_pos_unchecked_mut, access_pos_with_context, access_with_context,
        check_pos_with_context, deserialize_using, root_position,
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

/// Access a byte slice with a given root position.
///
/// This is a safe alternative to [`access_pos_unchecked`] and is part of the
/// [high-level API](crate::api::high).
///
/// [`access_pos_unchecked`]: crate::api::access_pos_unchecked
///
/// # Example
///
/// ```
/// use rkyv::{
///     api::{high::access_pos, root_position},
///     bytecheck::CheckBytes,
///     rancor::Error,
///     to_bytes, Archive, Archived, Serialize,
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
/// let archived = access_pos::<ArchivedExample, Error>(
///     &bytes,
///     root_position::<ArchivedExample>(bytes.len()),
/// )
/// .unwrap();
///
/// assert_eq!(archived.name, "pi");
/// assert_eq!(archived.value, 31415926);
/// ```
pub fn access_pos<T, E>(bytes: &[u8], pos: usize) -> Result<&T, E>
where
    T: Portable + for<'a> CheckBytes<HighValidator<'a, E>>,
    E: Source,
{
    access_pos_with_context::<_, _, E>(bytes, pos, &mut validator(bytes))
}

/// Access a byte slice.
///
/// This is a safe alternative to [`access_unchecked`] and is part of the
/// [high-level API](crate::api::high).
///
/// [`access_unchecked`]: crate::access_unchecked
///
/// # Example
///
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
/// let archived = access::<ArchivedExample, Error>(&bytes).unwrap();
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

/// Mutably access a byte slice with a given root position.
///
/// This is a safe alternative to [`access_pos_unchecked_mut`] and is part of
/// the [high-level API](crate::api::high).
///
/// # Example
///
/// ```
/// use rkyv::{
///     api::{high::access_pos_mut, root_position},
///     bytecheck::CheckBytes,
///     rancor::Error, munge::munge,
///     to_bytes, Archive, Archived, Serialize,
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
/// let mut bytes = to_bytes::<Error>(&value).unwrap();
/// let root_pos = root_position::<ArchivedExample>(bytes.len());
///
/// let mut archived =
///     access_pos_mut::<ArchivedExample, Error>(&mut bytes, root_pos).unwrap();
///
/// // Because the access is mutable, we can mutate the archived data
/// munge!(let ArchivedExample { mut value, .. } = archived);
/// assert_eq!(*value, 31415926);
/// *value = 12345.into();
/// assert_eq!(*value, 12345);
/// ```
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

/// Mutably access a byte slice.
///
/// This is a safe alternative to [`access_unchecked_mut`] and is part of the
/// [high-level API](crate::api::high).
///
/// [`access_unchecked_mut`]: crate::api::access_unchecked_mut
///
/// # Example
///
/// ```
/// use rkyv::{
///     access_mut,
///     bytecheck::CheckBytes,
///     rancor::Error, munge::munge,
///     to_bytes, Archive, Archived, Serialize,
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
/// let mut bytes = to_bytes::<Error>(&value).unwrap();
///
/// let mut archived = access_mut::<ArchivedExample, Error>(&mut bytes)
///     .unwrap();
///
/// // Because the access is mutable, we can mutate the archived data
/// munge!(let ArchivedExample { mut value, .. } = archived);
/// assert_eq!(*value, 31415926);
/// *value = 12345.into();
/// assert_eq!(*value, 12345);
/// ```
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

/// Deserialize a value from the given bytes.
///
/// This is a safe alternative to [`from_bytes_unchecked`] and is part of the
/// [high-level API](crate::api::high).
///
/// [`from_bytes_unchecked`]: crate::api::high::from_bytes_unchecked
///
/// # Example
///
/// ```
/// use rkyv::{
///     from_bytes, rancor::Error, to_bytes, Archive, Deserialize, Serialize,
/// };
///
/// #[derive(Archive, Serialize, Deserialize, Debug, PartialEq)]
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
/// let deserialized = from_bytes::<Example, Error>(&bytes).unwrap();
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
    deserialize_using(access::<T::Archived, E>(bytes)?, &mut deserializer)
}
