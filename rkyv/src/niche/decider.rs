//! Deciders for niching values with [`Nicher`].
//!
//! [`Nicher`]: crate::with::Nicher

use super::niched_option::NichedOption;
use crate::{Archive, Place, Portable};

/// A type that can be used to niche a value with [`Nicher`].
///
/// # Safety
///
/// If the method `is_none` returns `true`, it must be safe to access the field
/// [`NichedOption::niche`]. Similarly, if `is_none` returns `false`, it must be
/// safe to access the field [`NichedOption::some`].
///
/// [`Nicher`]: crate::with::Nicher
pub unsafe trait Decider<T: Archive> {
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
