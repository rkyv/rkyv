//! Deciders for niching values with [`Nicher`].
//!
//! [`Nicher`]: crate::with::Nicher

use crate::{Archive, Archived, Place, Portable};

/// A type that can be used to niche a value with [`Nicher`].
///
/// [`Nicher`]: crate::with::Nicher
pub trait Decider<T: Archive> {
    /// The archived representation of both niched and non-niched values.
    type Archived: Portable;

    /// Converts a niched archive to `None`; otherwise `Some(_)`.
    fn as_option(archived: &Self::Archived) -> Option<&Archived<T>>;

    /// Creates a `Self::Archived` from an `Option<&T>` and writes it to the
    /// given output.
    fn resolve_from_option(
        option: Option<&T>,
        resolver: Option<T::Resolver>,
        out: Place<Self::Archived>,
    );
}

/// [`Decider`] for zero-niched values.
pub struct Zero;

/// [`Decider`] for NaN-niched values.
pub struct NaN;

/// [`Decider`] for null-pointer-niched values.
pub struct Null;
