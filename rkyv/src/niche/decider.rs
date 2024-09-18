//! Deciders for niching values with [`Nicher`].
//!
//! [`Nicher`]: crate::with::Nicher

use crate::{Archive, Place, Portable};

/// A type that can be used to niche a value with [`Nicher`].
///
/// # Safety
///
/// For a union with two fields of type `Self::Niched` and `T::Archived`, it
/// must always be safe to access the `Self::Niched` field.
///
/// Additionally, if [`is_none`] returns `false` when being passed such a
/// union's `Self::Niched` field, it must be safe to access the `T::Archived`
/// field.
///
/// # Example
///
/// ```
/// use rkyv::{
///     niche::decider::Decider, with::Nicher, Archive, Archived, Place,
///     Serialize,
/// };
///
/// // Let's define a decider to niche `Some(1)` into `None`.
/// struct One;
///
/// // SAFETY: `Self::Niched` is the same as `T::Archived` so it's always valid
/// // to access it within a union of the two. Furthermore, we can be sure that
/// // the `T::Archived` field is safe to access if `is_none` returns `false`.
/// unsafe impl Decider<u32> for One {
///     type Niched = Archived<u32>;
///
///     fn is_none(niched: &Self::Niched) -> bool {
///         *niched == 1
///     }
///
///     fn resolve_niche(out: Place<Self::Niched>) {
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
/// # #[cfg(feature = "alloc")] {
/// let values = [Niched(Some(1)), Niched(Some(42)), Niched(None)];
/// let bytes = rkyv::to_bytes(&values)?;
/// # #[cfg(feature = "bytecheck")]
/// let mut iter = rkyv::access::<[ArchivedNiched; 3], _>(&bytes)?.iter();
/// # #[cfg(not(feature = "bytecheck"))]
/// # let mut iter = unsafe {
/// #     rkyv::access_unchecked::<[ArchivedNiched; 3]>(&bytes)
/// # }.iter();
/// assert_eq!(iter.next().unwrap().0.as_ref(), None);
/// assert_eq!(iter.next().unwrap().0.as_ref(), Some(&42.into()));
/// assert_eq!(iter.next().unwrap().0.as_ref(), None);
/// assert!(iter.next().is_none());
/// # }
/// # Ok(()) }
/// ```
///
/// [`Nicher`]: crate::with::Nicher
/// [`is_none`]: Decider::is_none
pub unsafe trait Decider<T: Archive> {
    /// The archived representation of a niched value.
    type Niched: Portable;

    /// Whether the given archived value has been niched or not.
    fn is_none(niched: &Self::Niched) -> bool;

    /// Creates a `Self::Niched` and writes it to the given output.
    fn resolve_niche(out: Place<Self::Niched>);
}

/// [`Decider`] for zero-niched values.
pub struct Zero;

/// [`Decider`] for NaN-niched values.
pub struct NaN;

/// [`Decider`] for null-pointer-niched values.
pub struct Null;
