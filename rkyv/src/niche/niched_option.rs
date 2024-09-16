//! A niched archived `Option<T>` that uses less space based on a niching
//! [`Decider`].

use core::{cmp, fmt};

use munge::munge;
use rancor::Fallible;

use super::decider::Decider;
use crate::{Archive, Archived, Place, Portable, Serialize};

#[repr(transparent)]
#[cfg_attr(feature = "bytecheck", derive(bytecheck::CheckBytes))]
#[derive(Portable)]
#[rkyv(crate)]
/// A niched archived `Option<T>`.
///
/// Depending on `D`, it may use less space by storing the `None` variant in a
/// custom way.
pub struct NichedOption<T, D>
where
    T: Archive,
    D: Decider<T>,
{
    repr: <D as Decider<T>>::Archived,
}

impl<T, D> NichedOption<T, D>
where
    T: Archive,
    D: Decider<T>,
{
    /// Converts to an `Option<&Archived<T>>`.
    pub fn as_ref(&self) -> Option<&Archived<T>> {
        D::as_option(&self.repr)
    }

    /// Returns `true` if the option is a `None` value.
    pub fn is_none(&self) -> bool {
        self.as_ref().is_none()
    }

    /// Returns `true` if the option is a `Some` value.
    pub fn is_some(&self) -> bool {
        self.as_ref().is_some()
    }

    /// Resolves a [`NichedOption<T, D>`] from an `Option<&T>`.
    pub fn resolve_from_option(
        option: Option<&T>,
        resolver: Option<T::Resolver>,
        out: Place<Self>,
    ) {
        munge!(let Self { repr } = out);
        D::resolve_from_option(option, resolver, repr);
    }

    /// Serializes a `NichedOption<T, D>` from an `Option<&T>`.
    pub fn serialize_from_option<S>(
        option: Option<&T>,
        serializer: &mut S,
    ) -> Result<Option<T::Resolver>, S::Error>
    where
        S: Fallible + ?Sized,
        T: Serialize<S>,
    {
        match option {
            Some(value) => value.serialize(serializer).map(Some),
            None => Ok(None),
        }
    }
}

impl<T, D> fmt::Debug for NichedOption<T, D>
where
    T: Archive<Archived: fmt::Debug>,
    D: Decider<T>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_ref().fmt(f)
    }
}

impl<T, D> Eq for NichedOption<T, D>
where
    T: Archive<Archived: Eq>,
    D: Decider<T>,
{
}

impl<T, D> PartialEq for NichedOption<T, D>
where
    T: Archive<Archived: PartialEq>,
    D: Decider<T>,
{
    fn eq(&self, other: &Self) -> bool {
        self.as_ref().eq(&other.as_ref())
    }
}

impl<T, D, Rhs> PartialEq<Option<Rhs>> for NichedOption<T, D>
where
    T: Archive<Archived: PartialEq<Rhs>>,
    D: Decider<T>,
{
    fn eq(&self, other: &Option<Rhs>) -> bool {
        match (self.as_ref(), other) {
            (Some(self_value), Some(other_value)) => self_value.eq(other_value),
            (None, None) => true,
            _ => false,
        }
    }
}

impl<T, D> Ord for NichedOption<T, D>
where
    T: Archive<Archived: Ord>,
    D: Decider<T>,
{
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.as_ref().cmp(&other.as_ref())
    }
}

impl<T, D> PartialOrd for NichedOption<T, D>
where
    T: Archive<Archived: PartialOrd>,
    D: Decider<T>,
{
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        self.as_ref().partial_cmp(&other.as_ref())
    }
}
