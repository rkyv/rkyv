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
    api::{deserialize_with, serialize_with},
    de::Pool,
    ser::{
        allocator::ArenaHandle, sharing::Share, Allocator, Serializer, Writer,
    },
    traits::Freeze,
    util::{with_arena, AlignedVec},
    Archive, Deserialize, Serialize,
};

/// A high-level serializer.
///
/// This is part of the [high-level API](crate::api::high).
pub type HighSerializer<'a, W, A, E> = Strategy<Serializer<W, A, Share>, E>;

/// A high-level deserializer.
///
/// This is part of the [high-level API](crate::api::high).
pub type HighDeserializer<E> = Strategy<Pool, E>;

/// Serializes the given value and returns the resulting bytes in an
/// [`AlignedVec`].
///
/// This is part of the [high-level API](crate::api::high).
///
/// # Examples
/// ```
/// use rkyv::rancor::Error;
///
/// let value = vec![1, 2, 3, 4];
///
/// let bytes =
///     rkyv::to_bytes::<Error>(&value).expect("failed to serialize vec");
/// // SAFETY:
/// // - The byte slice represents an archived object
/// // - The root of the object is stored at the end of the slice
/// let deserialized = unsafe {
///     rkyv::from_bytes_unchecked::<Vec<i32>, Error>(&bytes)
///         .expect("failed to deserialize vec")
/// };
///
/// assert_eq!(deserialized, value);
/// ```
pub fn to_bytes<E>(
    value: &impl for<'a> Serialize<
        HighSerializer<'a, AlignedVec, ArenaHandle<'a>, E>,
    >,
) -> Result<AlignedVec, E>
where
    E: rancor::Source,
{
    to_bytes_in(value, AlignedVec::new())
}

/// Serializes the given value and writes the bytes to the given `writer`.
///
/// This is part of the [high-level API](crate::api::high).
pub fn to_bytes_in<W, E>(
    value: &impl for<'a> Serialize<HighSerializer<'a, W, ArenaHandle<'a>, E>>,
    writer: W,
) -> Result<W, E>
where
    W: Writer<E>,
    E: rancor::Source,
{
    with_arena(|arena| to_bytes_in_with_alloc(value, writer, arena.acquire()))
}

/// Serializes the given value using the given allocator.
///
/// This is part of the [high-level API](crate::api::high).
pub fn to_bytes_with_alloc<'a, A, E>(
    value: &impl Serialize<HighSerializer<'a, AlignedVec, A, E>>,
    alloc: A,
) -> Result<AlignedVec, E>
where
    A: Allocator<E>,
    E: rancor::Source,
{
    to_bytes_in_with_alloc(value, AlignedVec::new(), alloc)
}

/// Serializes the given value and writes the bytes to the given `writer`, using
/// the given allocator.
///
/// This is part of the [high-level API](crate::api::high).
pub fn to_bytes_in_with_alloc<'a, W, A, E>(
    value: &impl Serialize<HighSerializer<'a, W, A, E>>,
    writer: W,
    alloc: A,
) -> Result<W, E>
where
    W: Writer<E>,
    A: Allocator<E>,
    E: rancor::Source,
{
    let mut serializer = Serializer::new(writer, alloc, Share::new());
    serialize_with(value, &mut serializer)?;
    Ok(serializer.into_writer())
}

/// Deserializes a value from the given bytes.
///
/// This function does not check that the data is valid. Use [`from_bytes`] to
/// validate the data instead.
///
/// This is part of the [high-level API](crate::api::high).
///
/// # Safety
///
/// The given bytes must pass validation when passed to [`from_bytes`].
///
/// # Examples
/// ```
/// use rkyv::rancor::Error;
///
/// let value = vec![1, 2, 3, 4];
///
/// let bytes =
///     rkyv::to_bytes::<Error>(&value).expect("failed to serialize vec");
/// // SAFETY:
/// // - The byte slice represents an archived object
/// // - The root of the object is stored at the end of the slice
/// let deserialized = unsafe {
///     rkyv::from_bytes_unchecked::<Vec<i32>, Error>(&bytes)
///         .expect("failed to deserialize vec")
/// };
///
/// assert_eq!(deserialized, value);
/// ```
pub unsafe fn from_bytes_unchecked<T, E>(bytes: &[u8]) -> Result<T, E>
where
    T: Archive,
    T::Archived: Freeze + Deserialize<T, HighDeserializer<E>>,
{
    // SAFETY: The caller has guaranteed that a valid `T` is located at the root
    // position in the byte slice.
    let archived = unsafe { access_unchecked::<T::Archived>(bytes) };
    deserialize(archived)
}

/// Deserializes a value from the given archived value.
///
/// This is part of the [high-level API](crate::api::high).
pub fn deserialize<T, E>(value: &T::Archived) -> Result<T, E>
where
    T: Archive,
    T::Archived: Deserialize<T, HighDeserializer<E>>,
{
    deserialize_with(value, &mut Pool::new())
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
