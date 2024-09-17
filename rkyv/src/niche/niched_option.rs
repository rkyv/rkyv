//! A niched archived `Option<T>` that uses less space based on a niching
//! [`Decider`].

use core::{cmp, fmt, mem::ManuallyDrop};

use munge::munge;
use rancor::Fallible;

use super::decider::Decider;
use crate::{Archive, Archived, Place, Portable, Serialize};

/// A niched archived `Option<T>`.
///
/// Depending on `D`, it may use less space by storing the `None` variant in a
/// custom way.
#[repr(C)]
#[derive(Portable)]
#[rkyv(crate)]
pub union NichedOption<T, D>
where
    T: Archive,
    D: Decider<T> + ?Sized,
{
    /// The archived representation of a `Some` value.
    pub some: ManuallyDrop<<T as Archive>::Archived>,
    /// The archived representation of a `None` value.
    pub niche: ManuallyDrop<<D as Decider<T>>::Niched>,
}

#[cfg(feature = "bytecheck")]
const _: () = {
    use crate::{bytecheck::CheckBytes, rancor::Source};

    unsafe impl<T, D, C> CheckBytes<C> for NichedOption<T, D>
    where
        T: Archive<Archived: CheckBytes<C>>,
        D: Decider<T, Niched: CheckBytes<C>> + ?Sized,
        C: Fallible<Error: Source> + ?Sized,
    {
        unsafe fn check_bytes(
            _value: *const Self,
            _context: &mut C,
        ) -> Result<(), C::Error> {
            // TODO

            Ok(())
        }
    }
};

impl<T, D> NichedOption<T, D>
where
    T: Archive,
    D: Decider<T> + ?Sized,
{
    /// Converts to an `Option<&Archived<T>>`.
    pub fn as_ref(&self) -> Option<&Archived<T>> {
        if D::is_none(self) {
            None
        } else {
            Some(unsafe { &*self.some })
        }
    }

    /// Converts to an `Option<&mut Archived<T>>`.
    pub fn as_mut(&mut self) -> Option<&mut Archived<T>> {
        if D::is_none(self) {
            None
        } else {
            Some(unsafe { &mut *self.some })
        }
    }

    /// Returns `true` if the option is a `None` value.
    pub fn is_none(&self) -> bool {
        self.as_ref().is_none()
    }

    /// Returns `true` if the option is a `Some` value.
    pub fn is_some(&self) -> bool {
        self.as_ref().is_some()
    }

    /// Resolves a `NichedOption<T, D>` from an `Option<&T>`.
    pub fn resolve_from_option(
        option: Option<&T>,
        resolver: Option<T::Resolver>,
        out: Place<Self>,
    ) {
        match option {
            Some(value) => {
                let resolver = resolver.expect("non-niched resolver");
                munge!(let Self { some } = out);
                value.resolve(resolver, unsafe {
                    some.cast_unchecked::<T::Archived>()
                });
            }
            None => {
                munge!(let Self { niche } = out);
                D::resolve_niche(unsafe {
                    niche.cast_unchecked::<D::Niched>()
                });
            }
        }
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
    D: Decider<T> + ?Sized,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_ref().fmt(f)
    }
}

impl<T, D> Eq for NichedOption<T, D>
where
    T: Archive<Archived: Eq>,
    D: Decider<T> + ?Sized,
{
}

impl<T, D> PartialEq for NichedOption<T, D>
where
    T: Archive<Archived: PartialEq>,
    D: Decider<T> + ?Sized,
{
    fn eq(&self, other: &Self) -> bool {
        self.as_ref().eq(&other.as_ref())
    }
}

impl<T, D, Rhs> PartialEq<Option<Rhs>> for NichedOption<T, D>
where
    T: Archive<Archived: PartialEq<Rhs>>,
    D: Decider<T> + ?Sized,
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
    D: Decider<T> + ?Sized,
{
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.as_ref().cmp(&other.as_ref())
    }
}

impl<T, D> PartialOrd for NichedOption<T, D>
where
    T: Archive<Archived: PartialOrd>,
    D: Decider<T> + ?Sized,
{
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        self.as_ref().partial_cmp(&other.as_ref())
    }
}
