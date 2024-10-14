//! [`Niching`] implementors for [`Nicher`].
//!
//! [`Nicher`]: crate::with::Nicher

use crate::Place;

/// A type that can be used to niche a value with [`Nicher`].
///
/// # Safety
///
/// - Assuming `T` implements [`Portable`], it must be safe for a
///   [`MaybeUninit<T>`] that is either initialized or niched by
///   [`resolve_niched`] to implement [`Portable`].
/// - For a [`MaybeUninit<T>`] that is either initialized or niched by
///   [`resolve_niched`], if [`is_niched`] returns `false`, it must be safe to
///   assume the `MaybeUninit` to be initialized.
/// - The returned pointer of [`niched_ptr`] must lay within `T`.
///
/// # Example
///
/// ```
/// use rkyv::{
///     niche::niching::Niching, primitive::ArchivedU32, with::Nicher, Archive,
///     Archived, Place, Serialize,
/// };
///
/// // Let's niche `Option<u32>` by using odd values
/// struct NeverOdd;
///
/// unsafe impl Niching<ArchivedU32> for NeverOdd {
///     type Niched = ArchivedU32;
///
///     fn niched_ptr(ptr: *const ArchivedU32) -> *const Self::Niched {
///         // We niche into the same type, no casting required
///         ptr
///     }
///
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
///     #[rkyv(with = Nicher<NeverOdd>)]
///     field: Option<u32>,
/// }
///
/// # fn _main() -> Result<(), rkyv::rancor::Error> {
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
/// [`MaybeUninit<T>`]: core::mem::MaybeUninit
/// [`Nicher`]: crate::with::Nicher
/// [`Portable`]: crate::traits::Portable
/// [`is_niched`]: Niching::is_niched
/// [`resolve_niched`]: Niching::resolve_niched
/// [`niched_ptr`]: Niching::niched_ptr
pub unsafe trait Niching<T> {
    /// The type that is leveraged for niching.
    type Niched;

    /// Returns the pointer within `T` to the value that is being used for
    /// niching.
    fn niched_ptr(ptr: *const T) -> *const Self::Niched;

    /// Whether the given value has been niched or not.
    ///
    /// Dereferencing `*const T` may cause UB depending on how
    /// [`resolve_niched`] niched it.
    ///
    /// # Safety
    ///
    /// `niched` must either point to a valid `T` or a value that was niched by
    /// [`resolve_niched`].
    ///
    /// [`resolve_niched`]: Niching::resolve_niched
    unsafe fn is_niched(niched: *const T) -> bool;

    /// Writes a niched instance of `T` to the given output.
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
/// Also serves as with-wrapper by being shorthand for `Nicher<DefaultNicher>`.
pub struct DefaultNicher;

/// [`Niching`] for zero-niched values.
pub struct Zero;

/// [`Niching`] for NaN-niched values.
pub struct NaN;

/// [`Niching`] for null-pointer-niched values.
pub struct Null;

/// [`Niching`] for booleans.
pub struct Bool;
