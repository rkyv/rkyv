//! [`Niching`] implementors for [`Nicher`].
//!
//! [`Nicher`]: crate::with::Nicher

/// A type that can be used to niche a value with [`Nicher`].
///
/// # Safety
///
/// - Assuming `T` implements [`Portable`], it must be safe for a
///   [`MaybeUninit<T>`] that is either initialized or niched by
///   [`resolve_niched`] to implement [`Portable`].
/// - For a [`MaybeUninit<T>`] that is either initialized or niched by
///   [`resolve_niched`], if [`is_niched`] returns true, it must be safe to
///   assume the `MaybeUninit` to be initialized.
/// - [`resolve_niched`] may not write uninitialized bytes to the returned
///   pointer.
/// - The returned pointer of [`niched_ptr`] must lay within `T`.
///
/// # Example
///
/// TODO
///
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
    /// [`resolve_niched`]: Niching::resolve_niched
    fn is_niched(niched: *const T) -> bool;

    /// Writes a niched instance of `T` to the given output.
    fn resolve_niched(out: *mut T);
}

/// Trait to allow `NichedOption<Self, N1>` to be niched further by `N2`.
///
/// # Safety
///
/// Implementors must ensure that the memory regions within `Self` that are used
/// for [`Niching`] impls of `N1` and `N2` are mutually exclusive.
pub unsafe trait SharedNiching<N1, N2> {}

/// [`Niching`] for zero-niched values.
pub struct Zero;

/// [`Niching`] for NaN-niched values.
pub struct NaN;

/// [`Niching`] for null-pointer-niched values.
pub struct Null;
