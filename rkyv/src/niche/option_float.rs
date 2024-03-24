#![allow(rustdoc::invalid_html_tags)] // sees `Option<f32>` as part HTML tag.

//! A niched archived `Option<Float>` that uses less space.

use core::{cmp, fmt, marker::PhantomData, pin::Pin};

use crate::{Archived, Portable};

/// Trait for checking whether a floating point number is *valid*.
///
/// Mostly copied from `noisy_float` to keep it an optional dependency,
/// and automatically implemented for all types that implement `noisy_float::FloatChecker`.
///
/// The implementation defines its own criteria for what constitutes a *valid* value.
pub trait FloatChecker<F> {
    /// Returns `true` if (and only if) the given floating point number is *valid*
    /// according to this checker's criteria.
    ///
    /// The only hard requirement is that NaN *must* be considered *invalid*
    /// for all implementations of `FloatChecker`.
    fn check(value: F) -> bool;
}

#[cfg(feature = "noisy_float")]
impl<F, C: noisy_float::FloatChecker<F>> FloatChecker<F> for C {
    #[inline(always)]
    fn check(value: F) -> bool {
        <C as noisy_float::FloatChecker<F>>::check(value)
    }
}

macro_rules! impl_archived_option_float {
    ($ar:ident, $f:ty) => {
        #[doc = concat!("A niched archived `Option<", stringify!($f), ">`")]
        /// where `None` is represented as `NaN`. `NaN`s will be converted to `None` when
        /// deserialized, so use this type with caution.
        #[derive(Portable)]
        #[archive(crate)]
        #[repr(transparent)]
        pub struct $ar<C: FloatChecker<$f>> {
            inner: Archived<$f>,
            _checker: PhantomData<C>,
        }

        impl<C: FloatChecker<$f>> $ar<C> {
            /// Returns `true` if the option is a `None` value.
            #[inline]
            pub fn is_none(&self) -> bool {
                !self.is_some()
            }

            /// Returns `true` if the option is a `Some` value.
            #[inline]
            pub fn is_some(&self) -> bool {
                C::check(self.inner.to_native())
            }

            #[doc = concat!("Converts to an `Option<&Archived<", stringify!($f), ">>`")]
            #[inline]
            pub fn as_ref(&self) -> Option<&Archived<$f>> {
                if self.is_none() { None } else { Some(&self.inner) }
            }

            #[doc = concat!("Converts to an `Option<&mut Archived<", stringify!($f), ">>`")]
            #[inline]
            pub fn as_mut(&mut self) -> Option<&mut Archived<$f>> {
                if self.is_none() { None } else { Some(&mut self.inner) }
            }

            #[doc = concat!("Converts from `Pin<&ArchivedOption", stringify!($f), ">` to `Option<Pin<&Archived<", stringify!($f), ">>>.")]
            #[inline]
            pub fn as_pin_ref(self: Pin<&Self>) -> Option<Pin<&Archived<$f>>> {
                unsafe { Pin::get_ref(self).as_ref().map(|x| Pin::new_unchecked(x)) }
            }

            #[doc = concat!("Converts from `Pin<&mut ArchivedOption", stringify!($f), ">` to `Option<Pin<&mut Archived<", stringify!($f), ">>>`.")]
            #[inline]
            pub fn as_pin_mut(self: Pin<&mut Self>) -> Option<Pin<&mut Archived<$f>>> {
                unsafe { Pin::get_unchecked_mut(self).as_mut().map(|x| Pin::new_unchecked(x)) }
            }

            /// Returns an iterator over the possibly contained value.
            #[inline]
            pub fn iter(&self) -> Iter<'_, Archived<$f>> {
                Iter { inner: self.as_ref() }
            }

            /// Returns a mutable iterator over the possibly contained value.
            #[inline]
            pub fn iter_mut(&mut self) -> IterMut<'_, Archived<$f>> {
                IterMut { inner: self.as_mut() }
            }

            /// Inserts `v` into the option if it is `None`, then returns a mutable
            /// reference to the contained value.
            #[inline]
            pub fn get_or_insert(&mut self, v: $f) -> &mut Archived<$f> {
                self.get_or_insert_with(move || v)
            }

            /// Inserts a value computed from `f` into the option if it is `None`, then
            /// returns a mutable reference to the contained value.
            #[inline]
            pub fn get_or_insert_with<F: FnOnce() -> $f>(&mut self, f: F) -> &mut Archived<$f> {
                if self.is_none() {
                    self.inner = f().into();
                }

                &mut self.inner
            }

            #[doc = concat!("Resolves an `ArchivedOption", stringify!($f), "` from an `Option<", stringify!($f), ">`.")]
            ///
            /// # Safety
            ///
            /// - `pos` must be the position of `out` within the archive
            #[inline]
            pub unsafe fn resolve_from_option(field: Option<$f>, out: *mut Self) {
                let (_, fo) = out_field!(out.inner);

                fo.write(match field {
                    Some(f) => f.into(),
                    None => <$f>::NAN.into(),
                });
            }
        }

        impl<C: FloatChecker<$f>> fmt::Debug for $ar<C> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                match self.as_ref() {
                    Some(inner) => inner.fmt(f),
                    None => f.debug_tuple("None").finish(),
                }
            }
        }

        impl<C: FloatChecker<$f>> PartialEq for $ar<C> {
            #[inline]
            fn eq(&self, other: &Self) -> bool {
                self.as_ref().eq(&other.as_ref())
            }
        }

        impl<C: FloatChecker<$f>> PartialOrd for $ar<C> {
            #[inline]
            fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
                self.as_ref().partial_cmp(&other.as_ref())
            }
        }

        #[cfg(feature = "bytecheck")]
        const _: () = {
            use bytecheck::CheckBytes;

            unsafe impl<FC: FloatChecker<$f>, C> CheckBytes<C> for $ar<FC>
                where C: rancor::Fallible + ?Sized
            {
                #[inline]
                unsafe fn check_bytes(value: *const Self, context: &mut C) -> Result<(), C::Error> {
                    Archived::<$f>::check_bytes(value.cast(), context)
                }
            }
        };
    };
}

impl_archived_option_float!(ArchivedOptionF32, f32);
impl_archived_option_float!(ArchivedOptionF64, f64);

/// An iterator over a reference to the `Some` variant of an
/// `ArchivedOptionFloat` float.
///
/// This iterator yields one value if the `ArchivedOptionFloat` float is a
/// `Some`, otherwise none.
pub type Iter<'a, T> = crate::option::Iter<'a, T>;

/// An iterator over a mutable reference to the `Some` variant of an
/// `ArchivedOptionFloat` float.
///
/// This iterator yields one value if the `ArchivedOptionFloat` float is a
/// `Some`, otherwise none.
pub type IterMut<'a, T> = crate::option::IterMut<'a, T>;
