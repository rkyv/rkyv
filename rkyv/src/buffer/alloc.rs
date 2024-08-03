use rancor::Strategy;

use crate::{
    access_unchecked,
    buffer::serialize_into,
    de::Pool,
    deserialize,
    ser::{sharing::Share, DefaultSerializer, Serializer, Writer},
    util::{with_arena, AlignedVec},
    Archive, Deserialize, Serialize,
};

/// Serializes the given value and returns the resulting bytes in an
/// [`AlignedVec`].
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
    value: &impl for<'a> Serialize<DefaultSerializer<'a, AlignedVec, E>>,
) -> Result<AlignedVec, E>
where
    E: rancor::Source,
{
    to_bytes_in(value, AlignedVec::new())
}

/// Serializes the given value and writes the bytes to the given `writer`.
pub fn to_bytes_in<W, E>(
    value: &impl for<'a> Serialize<DefaultSerializer<'a, W, E>>,
    writer: W,
) -> Result<W, E>
where
    W: Writer<E>,
    E: rancor::Source,
{
    with_arena(|arena| {
        Ok(serialize_into(
            value,
            Serializer::new(writer, arena.acquire(), Share::new()),
        )?
        .into_writer())
    })
}

/// Deserializes a value from the given bytes.
///
/// This function is only available with the `alloc` feature because it uses a
/// general-purpose deserializer. In no-alloc and high-performance environments,
/// the deserializer should be customized for the specific situation.
///
/// # Safety
///
/// - The byte slice must represent an archived object.
/// - The root of the object must be stored at the end of the slice (this is the
///   default behavior).
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
    T::Archived: Deserialize<T, Strategy<Pool, E>>,
{
    // SAFETY: The caller has guaranteed that a valid `T` is located at the root
    // position in the byte slice.
    let archived = unsafe { access_unchecked::<T::Archived>(bytes) };
    deserialize(archived, &mut Pool::new())
}

#[cfg(test)]
mod tests {
    use rancor::Panic;

    use crate::{
        alloc::{string::ToString, vec::Vec},
        buffer::to_bytes_in,
    };

    #[test]
    fn to_bytes_in_vec() {
        let value = "hello world".to_string();
        let bytes = to_bytes_in::<_, Panic>(&value, Vec::new()).unwrap();
        assert!(!bytes.is_empty());
    }
}
