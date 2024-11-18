//! A niched `ArchivedOption<T>` that uses less space based on a [`Niching`].

use core::{cmp, fmt, marker::PhantomData, mem::MaybeUninit, ops::Deref};

use munge::munge;
use rancor::Fallible;

use super::niching::Niching;
use crate::{seal::Seal, Archive, Place, Portable, Serialize};

/// A niched `ArchivedOption<T>`.
///
/// It has the same layout as `T`, and thus uses less space by storing the
/// `None` variant in a custom way based on `N`.
#[derive(Portable)]
#[rkyv(crate)]
#[repr(transparent)]
pub struct NichedOption<T, N: ?Sized> {
    repr: MaybeUninit<T>,
    _niching: PhantomData<N>,
}

#[cfg(feature = "bytecheck")]
const _: () = {
    use core::ptr::addr_of;

    use crate::bytecheck::CheckBytes;

    unsafe impl<T, N, C> CheckBytes<C> for NichedOption<T, N>
    where
        T: CheckBytes<C>,
        N: Niching<T> + ?Sized,
        C: Fallible + ?Sized,
    {
        unsafe fn check_bytes(
            value: *const Self,
            context: &mut C,
        ) -> Result<(), C::Error> {
            let ptr = unsafe { addr_of!((*value).repr).cast::<T>() };
            let is_niched = unsafe { N::is_niched(ptr) };

            if !is_niched {
                unsafe {
                    T::check_bytes(ptr, context)?;
                }
            }
            Ok(())
        }
    }
};

impl<T, N: Niching<T> + ?Sized> NichedOption<T, N> {
    /// Returns `true` if the option is a `None` value.
    pub fn is_none(&self) -> bool {
        unsafe { N::is_niched(self.repr.as_ptr()) }
    }

    /// Returns `true` if the option is a `Some` value.
    pub fn is_some(&self) -> bool {
        !self.is_none()
    }

    /// Converts to an `Option<&T>`.
    pub fn as_ref(&self) -> Option<&T> {
        if self.is_none() {
            None
        } else {
            Some(unsafe { self.repr.assume_init_ref() })
        }
    }

    /// Converts to an `Option<&mut T>`.
    pub fn as_mut(&mut self) -> Option<&mut T> {
        if self.is_none() {
            None
        } else {
            Some(unsafe { self.repr.assume_init_mut() })
        }
    }

    /// Converts from `Seal<'_, NichedOption<T, N>>` to `Option<Seal<'_, T>>`.
    pub fn as_seal(this: Seal<'_, Self>) -> Option<Seal<'_, T>> {
        let this = unsafe { Seal::unseal_unchecked(this) };
        this.as_mut().map(Seal::new)
    }

    /// Returns an iterator over the possibly-contained value.
    pub fn iter(&self) -> Iter<&'_ T> {
        Iter::new(self.as_ref())
    }

    /// Returns an iterator over the mutable possibly-contained value.
    pub fn iter_mut(&mut self) -> Iter<&'_ mut T> {
        Iter::new(self.as_mut())
    }

    /// Returns an iterator over the sealed possibly-contained value.
    pub fn iter_seal(this: Seal<'_, Self>) -> Iter<Seal<'_, T>> {
        Iter::new(Self::as_seal(this))
    }

    /// Resolves a `NichedOption<U::Archived, N>` from an `Option<&U>`.
    pub fn resolve_from_option<U>(
        option: Option<&U>,
        resolver: Option<U::Resolver>,
        out: Place<Self>,
    ) where
        U: Archive<Archived = T>,
    {
        let out = Self::munge_place(out);
        match option {
            Some(value) => {
                let resolver = resolver.expect("non-niched resolver");
                value.resolve(resolver, out);
            }
            None => N::resolve_niched(out),
        }
    }

    /// Serializes a `NichedOption<U::Archived, N>` from an `Option<&U>`.
    pub fn serialize_from_option<U, S>(
        option: Option<&U>,
        serializer: &mut S,
    ) -> Result<Option<U::Resolver>, S::Error>
    where
        U: Serialize<S, Archived = T>,
        S: Fallible + ?Sized,
    {
        match option {
            Some(value) => value.serialize(serializer).map(Some),
            None => Ok(None),
        }
    }

    pub(crate) fn munge_place(out: Place<Self>) -> Place<T> {
        munge!(let Self { repr, .. } = out);

        unsafe { repr.cast_unchecked::<T>() }
    }
}

impl<T, N> NichedOption<T, N>
where
    T: Deref,
    N: Niching<T> + ?Sized,
{
    /// Converts from `&NichedOption<T, N>` to `Option<&T::Target>`.
    pub fn as_deref(&self) -> Option<&<T as Deref>::Target> {
        self.as_ref().map(Deref::deref)
    }
}

impl<T, N> fmt::Debug for NichedOption<T, N>
where
    T: fmt::Debug,
    N: Niching<T> + ?Sized,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_ref().fmt(f)
    }
}

impl<T, N> Eq for NichedOption<T, N>
where
    T: Eq,
    N: Niching<T> + ?Sized,
{
}

impl<T, N> PartialEq for NichedOption<T, N>
where
    T: PartialEq,
    N: Niching<T> + ?Sized,
{
    fn eq(&self, other: &Self) -> bool {
        self.as_ref().eq(&other.as_ref())
    }
}

impl<T, N, Rhs> PartialEq<Option<Rhs>> for NichedOption<T, N>
where
    T: PartialEq<Rhs>,
    N: Niching<T> + ?Sized,
{
    fn eq(&self, other: &Option<Rhs>) -> bool {
        match (self.as_ref(), other) {
            (Some(self_value), Some(other_value)) => self_value.eq(other_value),
            (None, None) => true,
            _ => false,
        }
    }
}

impl<T, N> Ord for NichedOption<T, N>
where
    T: Ord,
    N: Niching<T> + ?Sized,
{
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.as_ref().cmp(&other.as_ref())
    }
}

impl<T, N> PartialOrd for NichedOption<T, N>
where
    T: PartialOrd,
    N: Niching<T> + ?Sized,
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
