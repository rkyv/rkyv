//! [`Niching`] implementors for [`Nicher`].
//!
//! [`Nicher`]: crate::with::Nicher

use crate::{Place, Portable};

/// A type that can be used to niche a value with [`Nicher`].
///
/// # Safety
///
/// For a union with two fields of type `Self::Niched` and `T`, it must always
/// be safe to access the `Self::Niched` field.
///
/// Additionally, if [`is_niched`] returns `false` when being passed such a
/// union's `Self::Niched` field, it must be safe to access the `T` field.
///
/// # Example
///
/// ```
/// use rkyv::{
///     niche::niching::Niching, with::Nicher, Archive, Archived, Place,
///     Serialize,
/// };
///
/// // Let's make it so that `Some(1)` is niched into `None`.
/// struct One;
///
/// // SAFETY: `Self::Niched` is the same as `T` so it's always valid to access
/// // it within a union of the two. Furthermore, we can be sure that the `T`
/// // field is safe to access if `is_niched` returns `false`.
/// unsafe impl Niching<Archived<u32>> for One {
///     type Niched = Archived<u32>;
///
///     fn is_niched(niched: &Self::Niched) -> bool {
///         *niched == 1
///     }
///
///     fn resolve_niched(out: Place<Self::Niched>) {
///         u32::resolve(&1, (), out);
///     }
/// }
///
/// #[derive(Archive)]
/// struct Basic(Option<u32>);
///
/// #[derive(Archive, Serialize)]
/// struct Niched(#[rkyv(with = Nicher<One>)] Option<u32>);
///
/// # fn main() -> Result<(), rkyv::rancor::Error> {
/// assert!(size_of::<ArchivedNiched>() < size_of::<ArchivedBasic>());
///
/// let values = [Niched(Some(1)), Niched(Some(42)), Niched(None)];
/// let bytes = rkyv::to_bytes(&values)?;
/// let archived = rkyv::access::<[ArchivedNiched; 3], _>(&bytes)?;
/// assert_eq!(archived[0].0.as_ref(), None);
/// assert_eq!(archived[1].0.as_ref(), Some(&42.into()));
/// assert_eq!(archived[2].0.as_ref(), None);
/// # Ok(()) }
/// ```
///
/// [`Nicher`]: crate::with::Nicher
/// [`is_niched`]: Niching::is_niched
pub unsafe trait Niching<T> {
    /// The archived representation of a niched value.
    type Niched: Portable;

    /// Whether the given archived value has been niched or not.
    fn is_niched(niched: &Self::Niched) -> bool;

    /// Creates a `Self::Niched` and writes it to the given output.
    fn resolve_niched(out: Place<Self::Niched>);
}

/// [`Niching`] for zero-niched values.
pub struct Zero;

/// [`Niching`] for NaN-niched values.
pub struct NaN;

/// [`Niching`] for null-pointer-niched values.
pub struct Null;
