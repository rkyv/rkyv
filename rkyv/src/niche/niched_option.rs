//! A niched archived `Option<T>` that may use less space based on a niching
//! [`Decider`].

use core::{cmp, fmt, mem::ManuallyDrop, ops::Deref};

use munge::munge;
use rancor::Fallible;

use super::decider::Decider;
use crate::{seal::Seal, Archive, Archived, Place, Portable, Serialize};

/// A niched archived `Option<T>`.
///
/// Depending on `D`, it may use less space by storing the `None` variant in a
/// custom way.
#[derive(Portable)]
#[rkyv(crate)]
#[cfg_attr(feature = "bytecheck", derive(bytecheck::CheckBytes))]
#[repr(transparent)]
pub struct NichedOption<T, D>
where
    T: Archive,
    D: Decider<T> + ?Sized,
{
    repr: Repr<T, D>,
}

#[repr(C)]
#[derive(Portable)]
#[rkyv(crate)]
union Repr<T, D>
where
    T: Archive,
    D: Decider<T> + ?Sized,
{
    some: ManuallyDrop<<T as Archive>::Archived>,
    niche: ManuallyDrop<<D as Decider<T>>::Niched>,
}

#[cfg(feature = "bytecheck")]
const _: () = {
    use crate::bytecheck::CheckBytes;

    unsafe impl<T, D, C> CheckBytes<C> for Repr<T, D>
    where
        T: Archive<Archived: CheckBytes<C>>,
        D: Decider<T, Niched: CheckBytes<C>> + ?Sized,
        C: Fallible + ?Sized,
    {
        unsafe fn check_bytes(
            value: *const Self,
            context: &mut C,
        ) -> Result<(), C::Error> {
            unsafe { <D::Niched>::check_bytes(&*(*value).niche, context)? };

            if D::is_none(unsafe { &*(*value).niche }) {
                return Ok(());
            }

            unsafe { <T::Archived>::check_bytes(&*(*value).some, context) }
        }
    }
};

impl<T, D> NichedOption<T, D>
where
    T: Archive,
    D: Decider<T> + ?Sized,
{
    /// Returns `true` if the option is a `None` value.
    pub fn is_none(&self) -> bool {
        D::is_none(unsafe { &*self.repr.niche })
    }

    /// Returns `true` if the option is a `Some` value.
    pub fn is_some(&self) -> bool {
        !self.is_none()
    }

    /// Converts to an `Option<&Archived<T>>`.
    pub fn as_ref(&self) -> Option<&Archived<T>> {
        if self.is_none() {
            None
        } else {
            Some(unsafe { &*self.repr.some })
        }
    }

    /// Converts to an `Option<&mut Archived<T>>`.
    pub fn as_mut(&mut self) -> Option<&mut Archived<T>> {
        if self.is_none() {
            None
        } else {
            Some(unsafe { &mut *self.repr.some })
        }
    }

    /// Converts from `Seal<'_, NichedOption<T, D>>` to `Option<Seal<'_,
    /// Archived<T>>>`.
    pub fn as_seal(this: Seal<'_, Self>) -> Option<Seal<'_, Archived<T>>> {
        let this = unsafe { Seal::unseal_unchecked(this) };
        this.as_mut().map(Seal::new)
    }

    /// Returns an iterator over the possibly-contained value.
    pub fn iter(&self) -> Iter<&'_ Archived<T>> {
        Iter::new(self.as_ref())
    }

    /// Returns an iterator over the mutable possibly-contained value.
    pub fn iter_mut(&mut self) -> Iter<&'_ mut Archived<T>> {
        Iter::new(self.as_mut())
    }

    /// Returns an iterator over the sealed possibly-contained value.
    pub fn iter_seal(this: Seal<'_, Self>) -> Iter<Seal<'_, Archived<T>>> {
        Iter::new(Self::as_seal(this))
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
                munge!(let Self { repr: Repr { some } } = out);
                value.resolve(resolver, unsafe {
                    some.cast_unchecked::<T::Archived>()
                });
            }
            None => {
                munge!(let Self { repr: Repr { niche } } = out);
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

impl<T, D> NichedOption<T, D>
where
    T: Archive<Archived: Deref>,
    D: Decider<T> + ?Sized,
{
    /// Converts from `&NichedOption<T, D>` to `Option<&Archived<T>::Target>`.
    pub fn as_deref(&self) -> Option<&<Archived<T> as Deref>::Target> {
        self.as_ref().map(Deref::deref)
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

/// An iterator over a reference to the `Some` variant of a `NichedOption`.
///
/// This iterator yields one value if the `NichedOption` is a `Some`, otherwise
/// none.
pub type Iter<P> = crate::option::Iter<P>;
