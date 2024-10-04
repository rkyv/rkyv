//! A niched archived `Option<T>` that uses less space based on a [`Niching`].

use core::{cmp, fmt, mem::ManuallyDrop, ops::Deref};

use munge::munge;
use rancor::Fallible;

use super::niching::Niching;
use crate::{
    seal::Seal, Archive, Archived, Place, Portable, Resolver, Serialize,
};

/// A niched archived `Option<T>`.
///
/// It uses less space by storing the `None` variant in a custom way based on
/// `N`.
#[derive(Portable)]
#[rkyv(crate)]
#[cfg_attr(feature = "bytecheck", derive(bytecheck::CheckBytes))]
#[repr(transparent)]
pub struct NichedOption<T, N>
where
    T: Archive,
    N: Niching<T> + ?Sized,
{
    repr: Repr<T, N>,
}

#[repr(C)]
#[derive(Portable)]
#[rkyv(crate)]
union Repr<T: Archive, N: Niching<T> + ?Sized> {
    some: ManuallyDrop<Archived<T>>,
    niche: ManuallyDrop<Archived<<N as Niching<T>>::Niched>>,
}

impl<T: Archive, N: Niching<T> + ?Sized> Repr<T, N> {
    /// Compile-time check to make sure that the niched type is not greater than
    /// the archived type.
    ///
    /// ```compile_fail
    /// use rkyv::{
    ///     niche::{niching::Niching, niched_option::NichedOption},
    ///     Archived, Place,
    /// };
    ///
    /// struct UselessNiching;
    ///
    /// unsafe impl Niching<u16> for UselessNiching {
    ///     type Niched = u32;
    ///
    ///     fn niched() -> Self::Niched {
    ///         unimplemented!()
    ///     }
    ///
    ///     fn is_niched(_: &Archived<Self::Niched>) -> bool {
    ///         unimplemented!()
    ///     }
    /// }
    ///
    /// let archived: Archived<u32> = 456.into();
    /// let niched: &NichedOption<u16, UselessNiching> =
    ///     unsafe { std::mem::transmute(&archived) };
    /// let _ = niched.is_none(); // <- size check = compile error
    /// ```
    const NICHE_SIZE_CHECK: () = {
        if size_of::<Archived<<N as Niching<T>>::Niched>>()
            > size_of::<Archived<T>>()
        {
            panic!(
                "`Archived<N::Niched>` is greater than `Archived<T>` and thus \
                 useless for niching"
            );
        }
    };
}

#[cfg(feature = "bytecheck")]
const _: () = {
    use crate::bytecheck::CheckBytes;

    unsafe impl<T, N, C> CheckBytes<C> for Repr<T, N>
    where
        T: Archive<Archived: CheckBytes<C>>,
        N: Niching<T> + ?Sized,
        <N::Niched as Archive>::Archived: CheckBytes<C>,
        C: Fallible + ?Sized,
    {
        unsafe fn check_bytes(
            value: *const Self,
            context: &mut C,
        ) -> Result<(), C::Error> {
            unsafe {
                <Archived<N::Niched>>::check_bytes(&*(*value).niche, context)?
            };

            if N::is_niched(unsafe { &*(*value).niche }) {
                return Ok(());
            }

            unsafe { <Archived<T>>::check_bytes(&*(*value).some, context) }
        }
    }
};

impl<T, N> NichedOption<T, N>
where
    T: Archive,
    N: Niching<T> + ?Sized,
{
    /// Returns `true` if the option is a `None` value.
    pub fn is_none(&self) -> bool {
        #[allow(clippy::let_unit_value)]
        let _ = Repr::<T, N>::NICHE_SIZE_CHECK;
        N::is_niched(unsafe { &*self.repr.niche })
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

    /// Converts from `Seal<'_, NichedOption<T, N>>` to `Option<Seal<'_,
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

    /// Resolves a `NichedOption<T, N>` from an `Option<&T>`.
    pub fn resolve_from_option(
        option: Option<&T>,
        resolver: NichedOptionResolver<T, N>,
        out: Place<Self>,
    ) {
        #[allow(clippy::let_unit_value)]
        let _ = Repr::<T, N>::NICHE_SIZE_CHECK;
        match option {
            Some(value) => {
                let NichedOptionResolver::Some(resolver) = resolver else {
                    panic!("expected non-niche resolver");
                };
                munge!(let Self { repr: Repr { some } } = out);
                let out = unsafe { some.cast_unchecked::<Archived<T>>() };
                value.resolve(resolver, out);
            }
            None => {
                let NichedOptionResolver::Niched(resolver) = resolver else {
                    panic!("expected niche resolver");
                };
                munge!(let Self { repr: Repr { niche } } = out);
                let out =
                    unsafe { niche.cast_unchecked::<Archived<N::Niched>>() };
                N::niched().resolve(resolver, out);
            }
        }
    }

    /// Serializes a `NichedOption<T, N>` from an `Option<&T>`.
    pub fn serialize_from_option<S>(
        option: Option<&T>,
        serializer: &mut S,
    ) -> Result<NichedOptionResolver<T, N>, S::Error>
    where
        S: Fallible + ?Sized,
        T: Serialize<S>,
        N::Niched: Serialize<S>,
    {
        match option {
            Some(value) => {
                value.serialize(serializer).map(NichedOptionResolver::Some)
            }
            None => N::niched()
                .serialize(serializer)
                .map(NichedOptionResolver::Niched),
        }
    }
}

impl<T, N> NichedOption<T, N>
where
    T: Archive<Archived: Deref>,
    N: Niching<T> + ?Sized,
{
    /// Converts from `&NichedOption<T, N>` to `Option<&Archived<T>::Target>`.
    pub fn as_deref(&self) -> Option<&<Archived<T> as Deref>::Target> {
        self.as_ref().map(Deref::deref)
    }
}

impl<T, N> fmt::Debug for NichedOption<T, N>
where
    T: Archive<Archived: fmt::Debug>,
    N: Niching<T> + ?Sized,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_ref().fmt(f)
    }
}

impl<T, N> Eq for NichedOption<T, N>
where
    T: Archive<Archived: Eq>,
    N: Niching<T> + ?Sized,
{
}

impl<T, N> PartialEq for NichedOption<T, N>
where
    T: Archive<Archived: PartialEq>,
    N: Niching<T> + ?Sized,
{
    fn eq(&self, other: &Self) -> bool {
        self.as_ref().eq(&other.as_ref())
    }
}

impl<T, N, Rhs> PartialEq<Option<Rhs>> for NichedOption<T, N>
where
    T: Archive<Archived: PartialEq<Rhs>>,
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
    T: Archive<Archived: Ord>,
    N: Niching<T> + ?Sized,
{
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.as_ref().cmp(&other.as_ref())
    }
}

impl<T, N> PartialOrd for NichedOption<T, N>
where
    T: Archive<Archived: PartialOrd>,
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

/// A resolver for `Option<T>` when niched by `N`.
pub enum NichedOptionResolver<T: Archive, N: Niching<T> + ?Sized> {
    /// A resolver for `T`.
    Some(Resolver<T>),
    /// A resolver for `N::Niched`.
    Niched(Resolver<N::Niched>),
}
