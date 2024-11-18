//! [`Niching`] implementors for [`NicheInto`].
//!
//! [`NicheInto`]: crate::with::NicheInto

use crate::Place;

/// A type that can be used to niche a value with [`NicheInto`].
///
/// # Example
///
/// ```
/// use rkyv::{
///     niche::niching::Niching, primitive::ArchivedU32, with::NicheInto,
///     Archive, Archived, Place, Serialize,
/// };
///
/// // Let's niche `Option<u32>` by using odd values
/// struct NeverOdd;
///
/// impl Niching<ArchivedU32> for NeverOdd {
///     unsafe fn is_niched(niched: *const ArchivedU32) -> bool {
///         // Interprete odd values as "niched"
///         unsafe { *niched % 2 == 1 }
///     }
///
///     fn resolve_niched(out: Place<ArchivedU32>) {
///         // To niche, we use the value `1`
///         out.write(ArchivedU32::from_native(1))
///     }
/// }
///
/// #[derive(Archive)]
/// struct Basic {
///     field: Option<u32>,
/// }
///
/// #[derive(Archive, Serialize)]
/// struct Niched {
///     #[rkyv(with = NicheInto<NeverOdd>)]
///     field: Option<u32>,
/// }
///
/// # fn main() -> Result<(), rkyv::rancor::Error> {
/// // Indeed, we have a smaller archived representation
/// assert!(size_of::<ArchivedNiched>() < size_of::<ArchivedBasic>());
///
/// let values: Vec<Niched> =
///     (0..4).map(|n| Niched { field: Some(n) }).collect();
///
/// let bytes = rkyv::to_bytes(&values)?;
/// let archived = rkyv::access::<Archived<Vec<Niched>>, _>(&bytes)?;
/// assert_eq!(archived[0].field.as_ref(), Some(&0.into()));
/// assert_eq!(archived[1].field.as_ref(), None);
/// assert_eq!(archived[2].field.as_ref(), Some(&2.into()));
/// assert_eq!(archived[3].field.as_ref(), None);
/// # Ok(()) }
/// ```
///
/// [`NicheInto`]: crate::with::NicheInto
pub trait Niching<T> {
    /// Returns whether the given value has been niched.
    ///
    /// While `niched` is guaranteed to point to bytes which are all valid to
    /// read, the value it points to is not guaranteed to be a valid instance of
    /// `T`.
    ///
    /// # Safety
    ///
    /// `niched` must be non-null, properly-aligned, and safe for reads. It does
    /// not have to point to a valid `T`.
    unsafe fn is_niched(niched: *const T) -> bool;

    /// Writes data to `out` indicating that a `T` is niched.
    fn resolve_niched(out: Place<T>);
}

/// Trait to allow `NichedOption<Self, N1>` to be niched further by `N2`.
///
/// # Safety
///
/// Implementors must ensure that the memory regions within `Self` that are used
/// for [`Niching`] impls of `N1` and `N2` are mutually exclusive.
pub unsafe trait SharedNiching<N1, N2> {}

/// Default [`Niching`] for various types.
///
/// Also serves as with-wrapper by being shorthand for
/// `NicheInto<DefaultNiche>`.
pub struct DefaultNiche;

/// [`Niching`] for zero-niched values.
pub struct Zero;

/// [`Niching`] for NaN-niched values.
pub struct NaN;

/// [`Niching`] for null-pointer-niched values.
pub struct Null;

/// [`Niching`] for booleans.
pub struct Bool;
