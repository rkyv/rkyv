//! Low-level checked APIs.
//!
//! These APIs require user-provided writers and allocators, and do not support
//! shared pointers.

use bytecheck::CheckBytes;
use rancor::{Source, Strategy};

use crate::{
    api::{
        access_pos_unchecked_mut, access_pos_with_context, access_with_context,
        check_pos_with_context, deserialize_using, root_position,
    },
    de::pooling::Unpool,
    seal::Seal,
    validation::{archive::ArchiveValidator, Validator},
    Archive, Deserialize, Portable,
};

/// A low-level validator.
///
/// This is part of the [low-level API](crate::api::low).
pub type LowValidator<'a, E> = Strategy<Validator<ArchiveValidator<'a>, ()>, E>;

fn validator(bytes: &[u8]) -> Validator<ArchiveValidator<'_>, ()> {
    Validator::new(ArchiveValidator::new(bytes), ())
}

/// Access a byte slice with a given root position.
///
/// This is a safe alternative to [`access_pos_unchecked`] and is part of the
/// [low-level API](crate::api::low).
///
/// [`access_pos_unchecked`]: crate::api::access_pos_unchecked
///
/// # Example
///
/// ```
/// use core::mem::MaybeUninit;
///
/// use rkyv::{
///     api::{
///         low::{access_pos, to_bytes_in_with_alloc},
///         root_position,
///     },
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
/// let archived = access_pos::<ArchivedExample<'_>, Failure>(
///     &*bytes,
///     root_position::<ArchivedExample<'_>>(bytes.len()),
/// )
/// .unwrap();
/// assert_eq!(*archived.inner, 42);
/// ```
pub fn access_pos<T, E>(bytes: &[u8], pos: usize) -> Result<&T, E>
where
    T: Portable + for<'a> CheckBytes<LowValidator<'a, E>>,
    E: Source,
{
    access_pos_with_context::<_, _, E>(bytes, pos, &mut validator(bytes))
}

/// Access a byte slice.
///
/// This is a safe alternative to [`access_unchecked`] and is part of the
/// [low-level API](crate::api::low).
///
/// [`access_unchecked`]: crate::api::access_unchecked
///
/// # Example
///
/// ```
/// use core::mem::MaybeUninit;
///
/// use rkyv::{
///     api::{
///         low::{access, to_bytes_in_with_alloc},
///         root_position,
///     },
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
/// let archived = access::<ArchivedExample<'_>, Failure>(&*bytes).unwrap();
/// assert_eq!(*archived.inner, 42);
/// ```
pub fn access<T, E>(bytes: &[u8]) -> Result<&T, E>
where
    T: Portable + for<'a> CheckBytes<LowValidator<'a, E>>,
    E: Source,
{
    access_with_context::<_, _, E>(bytes, &mut validator(bytes))
}

/// Mutably access a byte slice with a given root position.
///
/// This is a safe alternative to [`access_pos_unchecked_mut`] and is part of
/// the [low-level API](crate::api::low).
///
/// [`access_pos_unchecked_mut`]: crate::api::access_pos_unchecked_mut
///
/// # Example
///
/// ```
/// use core::mem::MaybeUninit;
///
/// use rkyv::{
///     api::{root_position, low::{to_bytes_in_with_alloc, access_pos_mut}},
///     rancor::Failure,
///     ser::{allocator::SubAllocator, writer::Buffer},
///     util::Align,
///     with::InlineAsBox,
///     Archive, Serialize,
///     munge::munge,
/// };
///
/// let mut output = Align([MaybeUninit::<u8>::uninit(); 256]);
/// let mut alloc = [MaybeUninit::<u8>::uninit(); 256];
///
/// #[derive(Archive, Serialize)]
/// struct Example {
///     inner: i32,
/// }
///
/// let value = Example { inner: 42 };
///
/// let mut bytes = to_bytes_in_with_alloc::<_, _, Failure>(
///     &value,
///     Buffer::from(&mut *output),
///     SubAllocator::new(&mut alloc),
/// )
/// .unwrap();
///
/// let root_pos = root_position::<ArchivedExample>(bytes.len());
/// let mut archived = access_pos_mut::<ArchivedExample, Failure>(
///     &mut *bytes,
///     root_pos,
/// ).unwrap();
///
/// // Because the access is mutable, we can mutate the archived data
/// munge!(let ArchivedExample { mut inner, .. } = archived);
/// assert_eq!(*inner, 42);
/// *inner = 12345.into();
/// assert_eq!(*inner, 12345);
/// ```
pub fn access_pos_mut<T, E>(
    bytes: &mut [u8],
    pos: usize,
) -> Result<Seal<'_, T>, E>
where
    T: Portable + for<'a> CheckBytes<LowValidator<'a, E>>,
    E: Source,
{
    let mut context = validator(bytes);
    check_pos_with_context::<T, _, E>(bytes, pos, &mut context)?;
    unsafe { Ok(access_pos_unchecked_mut::<T>(bytes, pos)) }
}

/// Mutably accesses a byte slice.
///
/// This is a safe alternative to [`access_unchecked_mut`] and is part of the
/// [low-level API](crate::api::low).
///
/// [`access_unchecked_mut`]: crate::api::access_unchecked_mut
///
/// # Example
///
/// ```
/// use core::mem::MaybeUninit;
///
/// use rkyv::{
///     api::low::{to_bytes_in_with_alloc, access_mut},
///     rancor::Failure,
///     ser::{allocator::SubAllocator, writer::Buffer},
///     util::Align,
///     with::InlineAsBox,
///     Archive, Serialize,
///     munge::munge,
/// };
///
/// let mut output = Align([MaybeUninit::<u8>::uninit(); 256]);
/// let mut alloc = [MaybeUninit::<u8>::uninit(); 256];
///
/// #[derive(Archive, Serialize)]
/// struct Example {
///     inner: i32,
/// }
///
/// let value = Example { inner: 42 };
///
/// let mut bytes = to_bytes_in_with_alloc::<_, _, Failure>(
///     &value,
///     Buffer::from(&mut *output),
///     SubAllocator::new(&mut alloc),
/// )
/// .unwrap();
///
/// let mut archived = access_mut::<ArchivedExample, Failure>(
///     &mut *bytes,
/// ).unwrap();
///
/// // Because the access is mutable, we can mutate the archived data
/// munge!(let ArchivedExample { mut inner, .. } = archived);
/// assert_eq!(*inner, 42);
/// *inner = 12345.into();
/// assert_eq!(*inner, 12345);
/// ```
pub fn access_mut<T, E>(bytes: &mut [u8]) -> Result<Seal<'_, T>, E>
where
    T: Portable + for<'a> CheckBytes<LowValidator<'a, E>>,
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
/// [low-level API](crate::api::low).
///
/// [`from_bytes_unchecked`]: crate::api::low::from_bytes_unchecked
///
/// # Example
///
/// ```
/// use core::mem::MaybeUninit;
///
/// use rkyv::{
///     api::low::{from_bytes, to_bytes_in_with_alloc},
///     rancor::Failure,
///     ser::{allocator::SubAllocator, writer::Buffer},
///     util::Align,
///     Archive, Deserialize, Serialize,
/// };
///
/// let mut output = Align([MaybeUninit::<u8>::uninit(); 256]);
/// let mut alloc = [MaybeUninit::<u8>::uninit(); 256];
///
/// #[derive(Archive, Serialize, Deserialize, PartialEq, Debug)]
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
/// let deserialized = from_bytes::<Example, Failure>(&*bytes).unwrap();
/// assert_eq!(value, deserialized);
/// ```
pub fn from_bytes<T, E>(bytes: &[u8]) -> Result<T, E>
where
    T: Archive,
    T::Archived: for<'a> CheckBytes<LowValidator<'a, E>>
        + Deserialize<T, Strategy<Unpool, E>>,
    E: Source,
{
    deserialize_using(access::<T::Archived, E>(bytes)?, &mut Unpool)
}
