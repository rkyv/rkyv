//! [`Niching`] implementors for [`Nicher`].
//!
//! [`Nicher`]: crate::with::Nicher

use crate::{Archive, Archived};

/// A type that can be used to niche a value with [`Nicher`].
///
/// # Safety
///
/// For a union with two fields of type `Archived<Self::Niched>` and
/// `Archived<T>`, it must always be safe to access the `Archived<Self::Niched>`
/// field.
///
/// Additionally, if [`is_niched`] returns `false` when being passed such a
/// union's `Archived<Self::Niched>` field, it must be safe to access the
/// `Archived<T>` field.
///
/// # Example
///
/// ```
/// use rkyv::{
///     niche::niching::Niching, with::Nicher, Archive, Archived, Serialize,
/// };
///
/// #[derive(Archive, Serialize)]
/// struct Nichable {
///     num: NonZeroU16, // <- we can use this for niching
///     other: u64,
/// }
///
/// #[derive(Archive)]
/// struct WithoutNiching {
///     field: Option<Nichable>, // <- wrapped in an `Option`
/// }
///
/// #[derive(Archive, Serialize)]
/// struct WithNiching {
///     #[rkyv(with = Nicher<MyNiching>)] // <- this time with a `Nicher`
///     field: Option<Nichable>,
/// }
///
/// #[derive(Archive, Serialize)]
/// struct NichedNichable {
///     num: u16, // <- same as above but with `u16` instead of `NonZeroU16`
///     other: u64,
/// }
///
/// // Now let's niche `Option<Nichable>` into `NichedNichable`
/// struct MyNiching;
///
/// // SAFETY: The only difference between `Archived<Nichable>` and
/// // `Archived<NichedNichable>` is the `num` field. Every `NonZeroU16` is a
/// // valid `u16` so the invariant is maintained. Additionally, the following
/// // implementation of `is_niched` guarantees that `Archived<Nichable>` is
/// // safely accessible if `is_niched` returns false.
/// unsafe impl Niching<Nichable> for MyNiching {
///     type Niched = NichedNichable;
///
///     fn niched() -> Self::Niched {
///         NichedNichable {
///             num: 0,
///             other: 123, // irrelevant
///         }
///     }
///
///     fn is_niched(niched: &Archived<Self::Niched>) -> bool {
///         niched.num == 0
///     }
/// }
///
/// # fn main() -> Result<(), rkyv::rancor::Error> {
/// // Indeed, we have a smaller archived representation
/// assert!(
///     size_of::<ArchivedWithNiching>() < size_of::<ArchivedWithoutNiching>()
/// );
///
/// let values = [
///     WithNiching {
///         field: Some(Nichable {
///             num: unsafe { NonZeroU16::new_unchecked(123) },
///             other: 789,
///         }),
///     },
///     WithNiching { field: None },
/// ];
/// let bytes = rkyv::to_bytes(&values)?;
/// let archived = rkyv::access::<[ArchivedWithNiching; 2], _>(&bytes)?;
/// assert_eq!(archived[0].field.as_ref(), Some(&Archived<789>));
/// assert!(archived[1].field.is_none());
/// # Ok(()) }
/// ```
///
/// [`Nicher`]: crate::with::Nicher
/// [`is_niched`]: Niching::is_niched
pub unsafe trait Niching<T> {
    /// The niched representation.
    type Niched: Archive;

    /// The value that serializes and resolves into a niched instance of
    /// `T::Archived`.
    fn niched() -> Self::Niched;

    /// Whether the given archived value has been niched or not.
    fn is_niched(niched: &Archived<Self::Niched>) -> bool;
}

/// [`Niching`] for zero-niched values.
pub struct Zero;

/// [`Niching`] for NaN-niched values.
pub struct NaN;

/// [`Niching`] for null-pointer-niched values.
pub struct Null;
