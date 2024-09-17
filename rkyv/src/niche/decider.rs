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
