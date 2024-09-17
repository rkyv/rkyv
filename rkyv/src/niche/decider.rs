//! Deciders for niching values with [`Nicher`].
//!
//! [`Nicher`]: crate::with::Nicher

use super::niched_option::NichedOption;
use crate::{Archive, Place, Portable};

/// A type that can be used to niche a value with [`Nicher`].
///
/// [`Nicher`]: crate::with::Nicher
pub trait Decider<T: Archive> {
    /// The archived representation of a niched value.
    type Niched: Portable;

    /// Whether the given `NichedOption` represents a niched value or not.
    fn is_none(option: &NichedOption<T, Self>) -> bool;

    /// Creates a `Self::Niched` and writes it to the given output.
    fn resolve_niche(out: Place<Self::Niched>);
}

/// [`Decider`] for zero-niched values.
pub struct Zero;

/// [`Decider`] for NaN-niched values.
pub struct NaN;

/// [`Decider`] for null-pointer-niched values.
pub struct Null;
