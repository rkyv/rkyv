//! [`Niching`] implementors for [`Nicher`].
//!
//! [`Nicher`]: crate::with::Nicher

/// A type that can be used to niche a value with [`Nicher`].
///
/// # Safety
///
/// Assuming `T` implements [`Portable`], it must be safe for a
/// [`MaybeUninit<T>`] that's either initialized or niched by [`resolve_niched`]
/// to implement [`Portable`].
///
/// # Example
///
/// TODO
///
/// [`Nicher`]: crate::with::Nicher
/// [`Portable`]: crate::traits::Portable
/// [`resolve_niched`]: Niching::resolve_niched
pub unsafe trait Niching<T> {
    /// The type that is leveraged for niching.
    type Niched;

    /// Whether the given value has been niched or not.
    ///
    /// Dereferencing `*const T` may cause UB depending on how
    /// [`resolve_niched`] niched it.
    ///
    /// [`resolve_niched`]: Niching::resolve_niched
    fn is_niched(niched: *const T) -> bool;

    /// Writes a niched instance of `T` to the given output.
    fn resolve_niched(out: *mut T);

    /// First checks whether the given pointer points to a value that has been
    /// niched correctly, and then checks if it has been niched at all.
    ///
    /// # Safety
    ///
    /// The niched value within the given pointer's value must be aligned and
    /// sufficiently initialized to represent the niched type.
    #[cfg(feature = "bytecheck")]
    unsafe fn checked_is_niched<C>(
        niched: *const T,
        _context: &mut C,
    ) -> Result<bool, C::Error>
    where
        C: rancor::Fallible + ?Sized,
        Self::Niched: bytecheck::CheckBytes<C>,
    {
        Ok(Self::is_niched(niched))
    }
}

/// [`Niching`] for zero-niched values.
pub struct Zero;

/// [`Niching`] for NaN-niched values.
pub struct NaN;

/// [`Niching`] for null-pointer-niched values.
pub struct Null;
