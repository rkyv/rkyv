//! APIs for environments where allocations cannot be made.
//!
//! These APIs require user-provided writers and allocators, and do not support
//! shared pointers.

#[cfg(feature = "bytecheck")]
mod checked;

use rancor::Strategy;

#[cfg(feature = "bytecheck")]
pub use self::checked::*;
use crate::{
    access_unchecked,
    api::{deserialize_using, serialize_using},
    ser::{Allocator, Serializer, Writer},
    Archive, Deserialize, Serialize,
};

/// A general-purpose serializer suitable for environments where allocations
/// cannot be made.
///
/// This is part of the [low-level API](crate::api::low).
pub type LowSerializer<W, A, E> = Strategy<Serializer<W, A, ()>, E>;

/// A general-purpose deserializer suitable for environments where allocations
/// cannot be made.
///
/// This is part of the [low-level API](crate::api::low).
pub type LowDeserializer<E> = Strategy<(), E>;

/// Serialize a value using the given allocator and write the bytes to the given
/// writer.
///
/// This is part of the [low-level API](crate::api::low).
///
/// # Example
///
/// ```
/// use core::mem::MaybeUninit;
///
/// use rkyv::{
///     access_unchecked,
///     api::low::to_bytes_in_with_alloc,
///     rancor::Failure,
///     ser::{allocator::SubAllocator, writer::Buffer},
///     util::Align,
///     with::InlineAsBox,
///     Archive, Serialize,
/// };
///
/// let mut output = Align([MaybeUninit::<u8>::uninit(); 256]);
/// let mut alloc = [MaybeUninit::<u8>::uninit(); 256];
///
/// #[derive(Archive, Serialize)]
/// struct Example<'a> {
///     #[rkyv(with = InlineAsBox)]
///     inner: &'a i32,
/// }
///
/// let forty_two = 42;
/// let value = Example { inner: &forty_two };
///
/// let bytes = to_bytes_in_with_alloc::<_, _, Failure>(
///     &value,
///     Buffer::from(&mut *output),
///     SubAllocator::new(&mut alloc),
/// )
/// .unwrap();
///
/// let archived = unsafe { access_unchecked::<ArchivedExample<'_>>(&*bytes) };
/// assert_eq!(*archived.inner, 42);
/// ```
pub fn to_bytes_in_with_alloc<W, A, E>(
    value: &impl Serialize<LowSerializer<W, A, E>>,
    writer: W,
    alloc: A,
) -> Result<W, E>
where
    W: Writer<E>,
    A: Allocator<E>,
    E: rancor::Source,
{
    let mut serializer = Serializer::new(writer, alloc, ());
    serialize_using(value, &mut serializer)?;
    Ok(serializer.into_writer())
}

/// Deserialize a value from the given bytes.
///
/// This function does not check that the data is valid. Use [`from_bytes`] to
/// validate the data instead.
///
/// This is part of the [low-level API](crate::api::low).
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
/// use core::mem::MaybeUninit;
///
/// use rkyv::{
///     access_unchecked,
///     api::low::{from_bytes_unchecked, to_bytes_in_with_alloc},
///     rancor::Failure,
///     ser::{allocator::SubAllocator, writer::Buffer},
///     util::Align,
///     with::InlineAsBox,
///     Archive, Deserialize, Serialize,
/// };
///
/// let mut output = Align([MaybeUninit::<u8>::uninit(); 256]);
/// let mut alloc = [MaybeUninit::<u8>::uninit(); 256];
///
/// #[derive(Archive, Serialize, Deserialize, Debug, PartialEq)]
/// struct Example {
///     inner: i32,
/// }
///
/// let value = Example { inner: 42 };
///
/// let bytes = to_bytes_in_with_alloc::<_, _, Failure>(
///     &value,
///     Buffer::from(&mut *output),
///     SubAllocator::new(&mut alloc),
/// )
/// .unwrap();
///
/// let deserialized =
///     unsafe { from_bytes_unchecked::<Example, Failure>(&*bytes).unwrap() };
/// assert_eq!(value, deserialized);
/// ```
pub unsafe fn from_bytes_unchecked<T, E>(bytes: &[u8]) -> Result<T, E>
where
    T: Archive,
    T::Archived: Deserialize<T, LowDeserializer<E>>,
{
    // SAFETY: The caller has guaranteed that a valid `T` is located at the root
    // position in the byte slice.
    let archived = unsafe { access_unchecked::<T::Archived>(bytes) };
    deserialize(archived)
}

/// Deserialize a value from the given archived value.
///
/// This is part of the [low-level API](crate::api::low).
///
/// # Example
///
/// ```
/// use core::mem::MaybeUninit;
///
/// use rkyv::{
///     access_unchecked,
///     api::low::{deserialize, to_bytes_in_with_alloc},
///     rancor::Failure,
///     ser::{allocator::SubAllocator, writer::Buffer},
///     util::Align,
///     with::InlineAsBox,
///     Archive, Deserialize, Serialize,
/// };
///
/// let mut output = Align([MaybeUninit::<u8>::uninit(); 256]);
/// let mut alloc = [MaybeUninit::<u8>::uninit(); 256];
///
/// #[derive(Archive, Serialize, Deserialize, Debug, PartialEq)]
/// struct Example {
///     inner: i32,
/// }
///
/// let value = Example { inner: 42 };
///
/// let bytes = to_bytes_in_with_alloc::<_, _, Failure>(
///     &value,
///     Buffer::from(&mut *output),
///     SubAllocator::new(&mut alloc),
/// )
/// .unwrap();
///
/// let archived = unsafe { access_unchecked::<ArchivedExample>(&*bytes) };
/// let deserialized = deserialize::<Example, Failure>(archived).unwrap();
/// assert_eq!(value, deserialized);
/// ```
pub fn deserialize<T, E>(
    value: &impl Deserialize<T, LowDeserializer<E>>,
) -> Result<T, E> {
    deserialize_using(value, &mut ())
}
