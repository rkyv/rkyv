//! APIs for environments where allocations can be made.
//!
//! These APIs have default writers, automatically manage allocators, and
//! support shared pointers.

#[cfg(feature = "bytecheck")]
mod checked;

use rancor::Strategy;

#[cfg(feature = "bytecheck")]
pub use self::checked::*;
use crate::{
    access_unchecked,
    api::{deserialize_using, serialize_using},
    de::Pool,
    ser::{
        allocator::ArenaHandle, sharing::Share, Allocator, Serializer, Writer,
    },
    util::{with_arena, AlignedVec},
    Archive, Deserialize, Serialize,
};

/// A high-level serializer.
///
/// This is part of the [high-level API](crate::api::high).
pub type HighSerializer<W, A, E> = Strategy<Serializer<W, A, Share>, E>;

/// A high-level deserializer.
///
/// This is part of the [high-level API](crate::api::high).
pub type HighDeserializer<E> = Strategy<Pool, E>;

/// Serialize a value to bytes.
///
/// Returns the serialized bytes in an [`AlignedVec`].
///
/// This is part of the [high-level API](crate::api::high).
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
pub fn to_bytes<E>(
    // rustfmt insists on inlining this parameter even though it exceeds the
    // max line length
    #[rustfmt::skip] value: &impl for<'a> Serialize<
        HighSerializer<AlignedVec, ArenaHandle<'a>, E>,
    >,
) -> Result<AlignedVec, E>
where
    E: rancor::Source,
{
    to_bytes_in(value, AlignedVec::new())
}

/// Serialize a value and write the bytes to the given writer.
///
/// This is part of the [high-level API](crate::api::high).
///
/// # Example
///
/// ```
/// use rkyv::{
///     api::high::to_bytes_in, from_bytes, rancor::Error, util::AlignedVec,
///     Archive, Deserialize, Serialize,
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
/// let bytes =
///     to_bytes_in::<_, Error>(&value, AlignedVec::<8>::new()).unwrap();
/// let deserialized = from_bytes::<Example, Error>(&bytes).unwrap();
///
/// assert_eq!(deserialized, value);
/// ```
pub fn to_bytes_in<W, E>(
    value: &impl for<'a> Serialize<HighSerializer<W, ArenaHandle<'a>, E>>,
    writer: W,
) -> Result<W, E>
where
    W: Writer<E>,
    E: rancor::Source,
{
    with_arena(|arena| to_bytes_in_with_alloc(value, writer, arena.acquire()))
}

/// Serialize a value using the given allocator.
///
/// This is part of the [high-level API](crate::api::high).
///
/// # Example
///
/// ```
/// use rkyv::{
///     api::high::to_bytes_with_alloc, from_bytes, rancor::Error,
///     util::with_arena, Archive, Deserialize, Serialize,
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
/// with_arena(|arena| {
///     let bytes =
///         to_bytes_with_alloc::<_, Error>(&value, arena.acquire()).unwrap();
///     let deserialized = from_bytes::<Example, Error>(&bytes).unwrap();
///
///     assert_eq!(deserialized, value);
/// });
/// ```
pub fn to_bytes_with_alloc<A, E>(
    value: &impl Serialize<HighSerializer<AlignedVec, A, E>>,
    alloc: A,
) -> Result<AlignedVec, E>
where
    A: Allocator<E>,
    E: rancor::Source,
{
    to_bytes_in_with_alloc(value, AlignedVec::new(), alloc)
}

/// Serialize a value using the given allocator and write the bytes to the given
/// writer.
///
/// This is part of the [high-level API](crate::api::high).
///
/// # Example
///
/// ```
/// use rkyv::{
///     api::high::to_bytes_in_with_alloc,
///     from_bytes,
///     rancor::Error,
///     util::{with_arena, AlignedVec},
///     Archive, Deserialize, Serialize,
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
/// with_arena(|arena| {
///     let bytes = to_bytes_in_with_alloc::<_, _, Error>(
///         &value,
///         AlignedVec::<8>::new(),
///         arena.acquire(),
///     )
///     .expect("failed to serialize vec");
///
///     let deserialized = from_bytes::<Example, Error>(&bytes)
///         .expect("failed to deserialize vec");
///
///     assert_eq!(deserialized, value);
/// });
/// ```
pub fn to_bytes_in_with_alloc<W, A, E>(
    value: &impl Serialize<HighSerializer<W, A, E>>,
    writer: W,
    alloc: A,
) -> Result<W, E>
where
    W: Writer<E>,
    A: Allocator<E>,
    E: rancor::Source,
{
    let mut serializer = Serializer::new(writer, alloc, Share::new());
    serialize_using(value, &mut serializer)?;
    Ok(serializer.into_writer())
}

/// Deserialize a value from the given bytes.
///
/// This function does not check that the data is valid. Use [`from_bytes`] to
/// validate the data instead.
///
/// This is part of the [high-level API](crate::api::high).
///
/// # Safety
///
/// The byte slice must represent a valid archived type when accessed at the
/// default root position. See the [module docs](crate::api) for more
/// information.
///
/// # Example
///
/// ```
/// use rkyv::{
///     from_bytes_unchecked, rancor::Error, to_bytes, Archive, Deserialize,
///     Serialize,
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
/// let deserialized =
///     unsafe { from_bytes_unchecked::<Example, Error>(&bytes).unwrap() };
///
/// assert_eq!(deserialized, value);
/// ```
pub unsafe fn from_bytes_unchecked<T, E>(bytes: &[u8]) -> Result<T, E>
where
    T: Archive,
    T::Archived: Deserialize<T, HighDeserializer<E>>,
{
    // SAFETY: The caller has guaranteed that a valid `T` is located at the root
    // position in the byte slice.
    let archived = unsafe { access_unchecked::<T::Archived>(bytes) };
    deserialize(archived)
}

/// Deserialize a value from the given archived value.
///
/// This is part of the [high-level API](crate::api::high).
///
/// # Example
///
/// ```
/// use rkyv::{
///     access, deserialize, rancor::Error, to_bytes, Archive, Deserialize,
///     Serialize,
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
/// let archived = access::<ArchivedExample, Error>(&*bytes).unwrap();
/// let deserialized = deserialize::<Example, Error>(archived).unwrap();
///
/// assert_eq!(deserialized, value);
/// ```
pub fn deserialize<T, E>(
    value: &impl Deserialize<T, HighDeserializer<E>>,
) -> Result<T, E> {
    deserialize_using(value, &mut Pool::new())
}

#[cfg(test)]
mod tests {
    use rancor::Panic;

    use crate::{
        alloc::{string::ToString, vec::Vec},
        api::high::to_bytes_in,
    };

    #[test]
    fn to_bytes_in_vec() {
        let value = "hello world".to_string();
        let bytes = to_bytes_in::<_, Panic>(&value, Vec::new()).unwrap();
        assert!(!bytes.is_empty());
    }
}
