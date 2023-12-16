//! Utility methods for accessing and deserializing safely.

use bytecheck::CheckBytes;
use ptr_meta::Pointee;
use rancor::{Error, Strategy};

use crate::{
    de::deserializers::SharedDeserializeMap,
    deserialize,
    validation::{
        validators::DefaultValidator, ArchiveContext, ArchiveContextExt as _,
    },
    Archive, Deserialize,
};

/// Accesses an archived value from the given byte slice at the given position
/// after checking its validity with the given context.
///
/// This is a safe alternative to [`access_pos_unchecked`][unsafe_version].
///
/// [unsafe_version]: crate::util::access_pos_unchecked
#[inline]
pub fn access_pos_with_context<'a, T, C, E>(
    buf: &'a [u8],
    pos: isize,
    context: &mut C,
) -> Result<&'a T::Archived, E>
where
    T: Archive,
    T::Archived: CheckBytes<Strategy<C, E>> + Pointee<Metadata = ()>,
    C: ArchiveContext<E> + ?Sized,
    E: Error,
{
    unsafe {
        let ptr =
            context.bounds_check_subtree_base_offset(buf.as_ptr(), pos, ())?;

        let range = context.push_prefix_subtree(ptr)?;
        CheckBytes::check_bytes(ptr, Strategy::wrap(context))?;
        context.pop_subtree_range(range)?;

        Ok(&*ptr)
    }
}

/// Accesses an archived value from the given byte slice by calculating the root
/// position after checking its validity with the given context.
///
/// This is a safe alternative to [`access_unchecked`][unsafe_version].
///
/// [unsafe_version]: crate::access_unchecked
#[inline]
pub fn access_with_context<'a, T, C, E>(
    buf: &'a [u8],
    context: &mut C,
) -> Result<&'a T::Archived, E>
where
    T: Archive,
    T::Archived: CheckBytes<Strategy<C, E>> + Pointee<Metadata = ()>,
    C: ArchiveContext<E> + ?Sized,
    E: Error,
{
    access_pos_with_context::<T, C, E>(
        buf,
        buf.len() as isize - core::mem::size_of::<T::Archived>() as isize,
        context,
    )
}

/// Accesses an archived value from the given byte slice at the given position
/// after checking its validity.
///
/// This is a safe alternative to [`access_pos_unchecked`][unsafe_version].
///
/// [unsafe_version]: crate::util::access_pos_unchecked
///
/// # Examples
/// ```
/// use rkyv::{
///     check_archived_value,
///     ser::{Serializer, serializers::AlignedSerializer},
///     AlignedVec,
///     Archive,
///     Serialize,
/// };
/// use bytecheck::CheckBytes;
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
/// let mut serializer = AlignedSerializer::new(AlignedVec::new());
/// let pos = serializer.serialize_value(&value)
///     .expect("failed to archive test");
/// let buf = serializer.into_inner();
/// let archived = check_archived_value::<Example>(buf.as_ref(), pos).unwrap();
/// ```
#[inline]
pub fn access_pos<T: Archive, E>(
    bytes: &[u8],
    pos: isize,
) -> Result<&T::Archived, E>
where
    T::Archived: CheckBytes<Strategy<DefaultValidator, E>>,
    E: Error,
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
#[inline]
pub fn access<T: Archive, E>(bytes: &[u8]) -> Result<&T::Archived, E>
where
    T::Archived: CheckBytes<Strategy<DefaultValidator, E>>,
    E: Error,
{
    let mut validator = DefaultValidator::new(bytes);
    access_with_context::<T, DefaultValidator, E>(bytes, &mut validator)
}

// TODO: access_mut/access_mut_*

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
/// let value = vec![1, 2, 3, 4];
///
/// let bytes = rkyv::to_bytes::<_, 1024>(&value)
///     .expect("failed to serialize vec");
/// let deserialized = rkyv::from_bytes::<Vec<i32>>(&bytes)
///     .expect("failed to deserialize vec");
///
/// assert_eq!(deserialized, value);
/// ```
#[inline]
pub fn from_bytes<T, E>(bytes: &[u8]) -> Result<T, E>
where
    T: Archive,
    T::Archived: CheckBytes<Strategy<DefaultValidator, E>>
        + Deserialize<T, Strategy<SharedDeserializeMap, E>>,
    E: Error,
{
    let mut deserializer = SharedDeserializeMap::default();
    deserialize(access::<T, E>(bytes)?, &mut deserializer)
}
