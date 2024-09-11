use bytecheck::CheckBytes;
use ptr_meta::Pointee;
use rancor::{Source, Strategy};

use crate::{
    api::{access_pos_unchecked, root_position},
    validation::{ArchiveContext, ArchiveContextExt},
    Portable,
};

/// Check a byte slice with a given root position and context.
///
/// Most of the time, `access` is a more ergonomic way to check and access a
/// byte slice.
///
/// # Example
///
/// ```
/// use rkyv::{
///     api::{check_pos_with_context, root_position},
///     rancor::Error,
///     to_bytes,
///     validation::{
///         archive::ArchiveValidator, shared::SharedValidator, Validator,
///     },
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
///
/// check_pos_with_context::<ArchivedExample, _, Error>(
///     &*bytes,
///     root_position::<ArchivedExample>(bytes.len()),
///     &mut Validator::new(
///         ArchiveValidator::new(&*bytes),
///         SharedValidator::new(),
///     ),
/// )
/// .unwrap();
/// ```
pub fn check_pos_with_context<T, C, E>(
    bytes: &[u8],
    pos: usize,
    context: &mut C,
) -> Result<(), E>
where
    T: CheckBytes<Strategy<C, E>> + Pointee<Metadata = ()>,
    C: ArchiveContext<E> + ?Sized,
    E: Source,
{
    let context = Strategy::<C, E>::wrap(context);
    let ptr = bytes.as_ptr().wrapping_add(pos).cast::<T>();
    context.in_subtree(ptr, |context| {
        // SAFETY: `in_subtree` has guaranteed that `ptr` is properly aligned
        // and points to enough bytes for a `T`.
        unsafe { T::check_bytes(ptr, context) }
    })
}

/// Access a byte slice with a given root position and context.
///
/// This is a safe alternative to [`access_pos_unchecked`].
///
/// Most of the time, the context should be newly-created and not reused. Prefer
/// `access_pos` whenever possible.
///
/// # Example
///
/// ```
/// use rkyv::{
///     api::{access_pos_with_context, root_position},
///     rancor::Error,
///     to_bytes,
///     validation::{
///         archive::ArchiveValidator, shared::SharedValidator, Validator,
///     },
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
///
/// let archived = access_pos_with_context::<ArchivedExample, _, Error>(
///     &*bytes,
///     root_position::<ArchivedExample>(bytes.len()),
///     &mut Validator::new(
///         ArchiveValidator::new(&*bytes),
///         SharedValidator::new(),
///     ),
/// )
/// .unwrap();
///
/// assert_eq!(archived.name.as_str(), "pi");
/// assert_eq!(archived.value.to_native(), 31415926);
/// ```
pub fn access_pos_with_context<'a, T, C, E>(
    bytes: &'a [u8],
    pos: usize,
    context: &mut C,
) -> Result<&'a T, E>
where
    T: Portable + CheckBytes<Strategy<C, E>> + Pointee<Metadata = ()>,
    C: ArchiveContext<E> + ?Sized,
    E: Source,
{
    check_pos_with_context::<T, C, E>(bytes, pos, context)?;
    unsafe { Ok(access_pos_unchecked::<T>(bytes, pos)) }
}

/// Access a byte slice with a given context.
///
/// This is a safe alternative to [`access_unchecked`].
///
/// Most of the time, the context should be newly-created and not reused. Prefer
/// `access` whenever possible.
///
/// [`access_unchecked`]: crate::api::access_unchecked
///
/// # Example
///
/// ```
/// use rkyv::{
///     api::{access_with_context, root_position},
///     rancor::Error,
///     to_bytes,
///     validation::{
///         archive::ArchiveValidator, shared::SharedValidator, Validator,
///     },
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
///
/// let archived = access_with_context::<ArchivedExample, _, Error>(
///     &*bytes,
///     &mut Validator::new(
///         ArchiveValidator::new(&*bytes),
///         SharedValidator::new(),
///     ),
/// )
/// .unwrap();
///
/// assert_eq!(archived.name.as_str(), "pi");
/// assert_eq!(archived.value.to_native(), 31415926);
/// ```
pub fn access_with_context<'a, T, C, E>(
    bytes: &'a [u8],
    context: &mut C,
) -> Result<&'a T, E>
where
    T: Portable + CheckBytes<Strategy<C, E>> + Pointee<Metadata = ()>,
    C: ArchiveContext<E> + ?Sized,
    E: Source,
{
    access_pos_with_context::<T, C, E>(
        bytes,
        root_position::<T>(bytes.len()),
        context,
    )
}
