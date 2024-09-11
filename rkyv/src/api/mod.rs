//! APIs for producing and using archived data.
//!
//! # Accessing byte slices
//!
//! The safety requirements for accessing a byte slice will often state that a
//! byte slice must "represent a valid archived type". The specific validity
//! requirements may vary widely depending on the types being accessed, and so
//! in general the only way to guarantee that this call is safe is to have
//! previously validated the byte slice.
//!
//! Using techniques such as cryptographic signing can provide a more performant
//! way to verify data integrity from trusted sources.
//!
//! It is generally safe to assume that unchanged and properly-aligned
//! serialized bytes are always safe to access without validation. By contrast,
//! bytes from a potentially-malicious source should always be validated prior
//! to access.

#[cfg(feature = "bytecheck")]
mod checked;
#[cfg(feature = "alloc")]
pub mod high;
pub mod low;
#[cfg(test)]
pub mod test;

use core::mem::size_of;

use rancor::Strategy;

#[cfg(feature = "bytecheck")]
pub use self::checked::*;
use crate::{seal::Seal, ser::Writer, Deserialize, Portable, SerializeUnsized};

#[cfg(debug_assertions)]
fn sanity_check_buffer<T: Portable>(ptr: *const u8, pos: usize, size: usize) {
    use core::mem::{align_of, size_of};

    let root_size = size_of::<T>();
    let min_size = pos + root_size;
    debug_assert!(
        min_size <= size,
        concat!(
            "buffer too small, expected at least {} bytes but found {} bytes\n",
            "help: the root type at offset {} requires at least {} bytes",
        ),
        min_size,
        size,
        pos,
        root_size,
    );
    let expect_align = align_of::<T>();
    let actual_align = (ptr as usize) & (expect_align - 1);
    debug_assert_eq!(
        actual_align,
        0,
        concat!(
            "unaligned buffer, expected alignment {} but found alignment {}\n",
            "help: rkyv requires byte buffers to be aligned to access the \
             data inside.\n",
            "      Using an AlignedVec or manually aligning your data with \
             `#[align(...)]` may resolve this issue.\n",
            "      Alternatively, you may enable the `unaligned` feature to \
             relax the alignment requirements for your archived data.\n",
            "      `unaligned` is a format control feature, and enabling it \
             may change the format of your serialized data)",
        ),
        expect_align,
        1 << actual_align.trailing_zeros()
    );
}

/// Return the position of the root within a buffer of `length` bytes.
///
/// Most accessing functions have a variant which automatically calculates this
/// value for you. For example, prefer to call [`access_unchecked`] over
/// [`access_pos_unchecked`].
///
/// The root position of a buffer is calculated by subtracing the size of the
/// root object from the end of the buffer. If the buffer size is too small to
/// accomodate a root of the given type, then this function will return zero.
///
/// # Example
///
/// ```
/// use rkyv::{api::root_position, Archive};
///
/// #[derive(Archive)]
/// pub struct MyData {
///     inner: u32,
/// }
///
/// assert_eq!(size_of::<ArchivedMyData>(), 4);
///
/// // This is too small, and so returns 0
/// assert_eq!(root_position::<ArchivedMyData>(3), 0);
/// assert_eq!(root_position::<ArchivedMyData>(4), 0);
/// assert_eq!(root_position::<ArchivedMyData>(5), 1);
/// ```
pub fn root_position<T: Portable>(size: usize) -> usize {
    size.saturating_sub(size_of::<T>())
}

/// Access a byte slice with a given root position.
///
/// Most of the time, the root position should be calculated using the root type
/// and size of the buffer. Prefer [`access_unchecked`] whenever possible.
///
/// While the root of the archived data is located at the given position, the
/// reachable data may be located throughout the byte slice.
///
/// This function does not check that the bytes are valid to access. Use
/// [`access_pos`](high::access_pos) to safely access the buffer using
/// validation.
///
/// # Safety
///
/// The byte slice must represent a valid archived type when accessed with the
/// given root position. See the [module docs](crate::api) for more information.
///
/// # Example
///
/// ```
/// use rkyv::{
///     api::{access_pos_unchecked, root_position},
///     rancor::Error,
///     to_bytes, Archive, Deserialize, Serialize,
/// };
///
/// #[derive(Archive, Serialize, Deserialize)]
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
///
/// let archived = unsafe {
///     access_pos_unchecked::<ArchivedExample>(
///         &*bytes,
///         root_position::<ArchivedExample>(bytes.len()),
///     )
/// };
/// assert_eq!(archived.name, "pi");
/// assert_eq!(archived.value, 31415926);
/// ```
pub unsafe fn access_pos_unchecked<T: Portable>(
    bytes: &[u8],
    pos: usize,
) -> &T {
    #[cfg(debug_assertions)]
    sanity_check_buffer::<T>(bytes.as_ptr(), pos, bytes.len());

    // SAFETY: The caller has guaranteed that a valid `T` is located at `pos` in
    // the byte slice.
    unsafe { &*bytes.as_ptr().add(pos).cast() }
}

/// Mutably access a byte slice with a given root position.
///
/// Most of the time, the root position should be calculated using the root type
/// and size of the buffer. Prefer [`access_unchecked_mut`] whenever possible.
///
/// While the root of the archived data is located at the given position, the
/// reachable data may be located throughout the byte slice.
///
/// This function does not check that the bytes are valid to access. Use
/// [`access_pos_mut`](high::access_pos_mut) to safely access the buffer using
/// validation.
///
/// The returned `Seal` restricts the mutating operations that may be safely
/// performed on the returned reference. See [`Seal`] for more information.
///
/// # Safety
///
/// The byte slice must represent a valid archived type when accessed with the
/// given root position. See the [module docs](crate::api) for more information.
///
/// # Example
///
/// ```
/// use rkyv::{
///     to_bytes, api::{root_position, access_pos_unchecked_mut}, util::Align,
///     Archive, Serialize, Deserialize, munge::munge, rancor::Error,
/// };
///
/// #[derive(Archive, Serialize, Deserialize)]
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
/// let mut archived = unsafe {
///     access_pos_unchecked_mut::<ArchivedExample>(&mut *bytes, root_pos)
/// };
/// assert_eq!(archived.name, "pi");
/// assert_eq!(archived.value, 31415926);
///
/// // Because the access is mutable, we can mutate the archived data
/// munge!(let ArchivedExample { mut value, .. } = archived);
/// assert_eq!(*value, 31415926);
/// *value = 12345.into();
/// assert_eq!(*value, 12345);
/// ```
pub unsafe fn access_pos_unchecked_mut<T: Portable>(
    bytes: &mut [u8],
    pos: usize,
) -> Seal<'_, T> {
    #[cfg(debug_assertions)]
    sanity_check_buffer::<T>(bytes.as_ptr(), pos, bytes.len());

    // SAFETY: The caller has guaranteed that the data at the given position
    // passes validation when passed to `access_pos_mut`.
    unsafe { Seal::new(&mut *bytes.as_mut_ptr().add(pos).cast()) }
}

/// Access a byte slice.
///
/// This function does not check that the bytes are valid to access. Use
/// [`access`](high::access) to safely access the buffer using validation.
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
///     access_unchecked, rancor::Error, to_bytes, Archive, Deserialize,
///     Serialize,
/// };
///
/// #[derive(Archive, Serialize, Deserialize)]
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
///
/// let archived = unsafe { access_unchecked::<ArchivedExample>(&*bytes) };
/// assert_eq!(archived.name, "pi");
/// assert_eq!(archived.value, 31415926);
/// ```
pub unsafe fn access_unchecked<T: Portable>(bytes: &[u8]) -> &T {
    // SAFETY: The caller has guaranteed that a valid `T` is located at the root
    // position in the byte slice.
    unsafe { access_pos_unchecked::<T>(bytes, root_position::<T>(bytes.len())) }
}

/// Mutably access a byte slice.
///
/// This function does not check that the bytes are valid to access. Use
/// [`access_mut`](high::access_mut) to safely access the buffer using
/// validation.
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
///     to_bytes, access_unchecked_mut, util::Align, Archive,
///     munge::munge, Serialize, Deserialize, rancor::Error,
/// };
///
/// #[derive(Archive, Serialize, Deserialize)]
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
/// let mut archived = unsafe {
///     access_unchecked_mut::<ArchivedExample>(&mut *bytes)
/// };
/// assert_eq!(archived.name, "pi");
/// assert_eq!(archived.value, 31415926);
///
/// // Because the access is mutable, we can mutate the archived data
/// munge!(let ArchivedExample { mut value, .. } = archived);
/// assert_eq!(*value, 31415926);
/// *value = 12345.into();
/// assert_eq!(*value, 12345);
/// ```
pub unsafe fn access_unchecked_mut<T: Portable>(
    bytes: &mut [u8],
) -> Seal<'_, T> {
    // SAFETY: The caller has guaranteed that the given bytes pass validation
    // when passed to `access_mut`.
    unsafe {
        access_pos_unchecked_mut::<T>(bytes, root_position::<T>(bytes.len()))
    }
}

/// Serialize a value using the given serializer.
///
/// Returns the position of the serialized value.
///
/// Most of the time, [`to_bytes`](high::to_bytes) is a more ergonomic way to
/// serialize a value to bytes.
///
/// # Example
///
/// ```
/// use rkyv::{
///     access,
///     api::serialize_using,
///     rancor::Error,
///     ser::{sharing::Share, Serializer},
///     util::{with_arena, AlignedVec},
///     Archive, Deserialize, Serialize,
/// };
///
/// #[derive(Archive, Serialize, Deserialize)]
/// struct Example {
///     name: String,
///     value: i32,
/// }
///
/// let bytes = with_arena(|arena| {
///     let mut serializer = Serializer::new(
///         AlignedVec::<4>::new(),
///         arena.acquire(),
///         Share::new(),
///     );
///
///     let value = Example {
///         name: "pi".to_string(),
///         value: 31415926,
///     };
///
///     serialize_using::<_, Error>(&value, &mut serializer).unwrap();
///     serializer.into_writer()
/// });
///
/// let archived = access::<ArchivedExample, Error>(&*bytes).unwrap();
/// assert_eq!(archived.value, 31415926);
/// ```
pub fn serialize_using<S, E>(
    value: &impl SerializeUnsized<Strategy<S, E>>,
    serializer: &mut S,
) -> Result<usize, E>
where
    S: Writer<E> + ?Sized,
{
    value.serialize_unsized(Strategy::wrap(serializer))
}

/// Deserialize a value using the given deserializer.
///
/// Most of the time, [`deserialize`](high::deserialize) is a more ergonomic way
/// to deserialize an archived value.
///
/// # Example
///
/// ```
/// use rkyv::{
///     access, api::deserialize_using, de::Pool, rancor::Error, to_bytes,
///     Archive, Deserialize, Serialize,
/// };
///
/// #[derive(Archive, Serialize, Deserialize)]
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
/// let deserialized =
///     deserialize_using::<Example, _, Error>(archived, &mut Pool::new())
///         .unwrap();
/// ```
pub fn deserialize_using<T, D, E>(
    value: &impl Deserialize<T, Strategy<D, E>>,
    deserializer: &mut D,
) -> Result<T, E> {
    value.deserialize(Strategy::wrap(deserializer))
}
