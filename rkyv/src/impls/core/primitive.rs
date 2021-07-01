//! [`Archive`] implementations for primitives.

use crate::{Archive, Archived, Deserialize, Fallible, FixedIsize, FixedUsize, Serialize};
#[cfg(has_atomics)]
use core::sync::atomic::{
    AtomicBool, AtomicI16, AtomicI32, AtomicI8, AtomicIsize, AtomicU16, AtomicU32, AtomicU8, AtomicUsize, Ordering,
};
#[cfg(has_atomics_64)]
use core::sync::atomic::{AtomicI64, AtomicU64};
use core::{
    marker::{PhantomData, PhantomPinned},
    mem::MaybeUninit,
    num::{
        NonZeroI128, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI8, NonZeroIsize, NonZeroU128,
        NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU8, NonZeroUsize,
    },
};
pub use rend::NativeEndian;

macro_rules! impl_primitive {
    (@serialize $type:ty) => {
        impl<S: Fallible + ?Sized> Serialize<S> for $type {
            #[inline]
            fn serialize(&self, _: &mut S) -> Result<Self::Resolver, S::Error> {
                Ok(())
            }
        }
    };
    ($type:ty) => {
        impl Archive for $type {
            type Archived = Self;
            type Resolver = ();

            #[inline]
            unsafe fn resolve(&self, _: usize, _: Self::Resolver, out: &mut MaybeUninit<Self::Archived>) {
                out.as_mut_ptr().write(*self);
            }
        }

        impl_primitive!(@serialize $type);

        impl<D: Fallible + ?Sized> Deserialize<$type, D> for Archived<$type> {
            #[inline]
            fn deserialize(&self, _: &mut D) -> Result<$type, D::Error> {
                Ok(*self)
            }
        }
    };
    (@multibyte $type:ty) => {
        const _: () = {
            #[cfg(not(any(feature = "archive_le", feature = "archive_be")))]
            type Archived = $type;
            #[cfg(feature = "archive_le")]
            type Archived = rend::LittleEndian<$type>;
            #[cfg(feature = "archive_be")]
            type Archived = rend::BigEndian<$type>;

            impl Archive for $type {
                type Archived = Archived;
                type Resolver = ();

                #[inline]
                unsafe fn resolve(&self, _: usize, _: Self::Resolver, out: &mut MaybeUninit<Self::Archived>) {
                    out.as_mut_ptr().write(to_archived!(*self as Self));
                }
            }

            impl_primitive!(@serialize $type);

            impl<D: Fallible + ?Sized> Deserialize<$type, D> for Archived {
                #[inline]
                fn deserialize(&self, _: &mut D) -> Result<$type, D::Error> {
                    Ok(from_archived!(*self))
                }
            }
        };
    };
}

macro_rules! impl_atomic {
    (@serialize_deserialize $type:ty) => {
        impl<S: Fallible + ?Sized> Serialize<S> for $type {
            #[inline]
            fn serialize(&self, _: &mut S) -> Result<Self::Resolver, S::Error> {
                Ok(())
            }
        }
    };
    ($type:ty) => {
        impl Archive for $type {
            type Archived = Self;
            type Resolver = ();

            #[inline]
            unsafe fn resolve(&self, _: usize, _: Self::Resolver, out: &mut MaybeUninit<Self::Archived>) {
                (&*out.as_ptr()).store(self.load(Ordering::Relaxed), Ordering::Relaxed);
            }
        }

        impl_atomic!(@serialize_deserialize $type);

        impl<D: Fallible + ?Sized> Deserialize<$type, D> for Archived<$type> {
            #[inline]
            fn deserialize(&self, _: &mut D) -> Result<$type, D::Error> {
                Ok(self.load(Ordering::Relaxed).into())
            }
        }
    };
    (@multibyte $type:ty) => {
        impl Archive for $type {
            #[cfg(not(any(feature = "archive_le", feature = "archive_be")))]
            type Archived = Self;
            #[cfg(feature = "archive_le")]
            type Archived = rend::LittleEndian<$type>;
            #[cfg(feature = "archive_be")]
            type Archived = rend::BigEndian<$type>;

            type Resolver = ();

            #[inline]
            unsafe fn resolve(&self, _: usize, _: Self::Resolver, out: &mut MaybeUninit<Self::Archived>) {
                (&*out.as_ptr()).store(self.load(Ordering::Relaxed), Ordering::Relaxed);
            }
        }

        impl_atomic!(@serialize_deserialize $type);

        impl<D: Fallible + ?Sized> Deserialize<$type, D> for Archived<$type> {
            #[inline]
            fn deserialize(&self, _: &mut D) -> Result<$type, D::Error> {
                Ok(self.load(Ordering::Relaxed).into())
            }
        }
    };
}

impl_primitive!(());
impl_primitive!(bool);
impl_primitive!(i8);
impl_primitive!(u8);
impl_primitive!(NonZeroI8);
impl_primitive!(NonZeroU8);
#[cfg(has_atomics)]
impl_atomic!(AtomicBool);
#[cfg(has_atomics)]
impl_atomic!(AtomicI8);
#[cfg(has_atomics)]
impl_atomic!(AtomicU8);

impl_primitive!(@multibyte i16);
impl_primitive!(@multibyte i32);
impl_primitive!(@multibyte i64);
impl_primitive!(@multibyte i128);
impl_primitive!(@multibyte u16);
impl_primitive!(@multibyte u32);
impl_primitive!(@multibyte u64);
impl_primitive!(@multibyte u128);

impl_primitive!(@multibyte f32);
impl_primitive!(@multibyte f64);

impl_primitive!(@multibyte char);

impl_primitive!(@multibyte NonZeroI16);
impl_primitive!(@multibyte NonZeroI32);
impl_primitive!(@multibyte NonZeroI64);
impl_primitive!(@multibyte NonZeroI128);
impl_primitive!(@multibyte NonZeroU16);
impl_primitive!(@multibyte NonZeroU32);
impl_primitive!(@multibyte NonZeroU64);
impl_primitive!(@multibyte NonZeroU128);

#[cfg(has_atomics)]
impl_atomic!(@multibyte AtomicI16);
#[cfg(has_atomics)]
impl_atomic!(@multibyte AtomicI32);
#[cfg(has_atomics_64)]
impl_atomic!(@multibyte AtomicI64);
#[cfg(has_atomics)]
impl_atomic!(@multibyte AtomicU16);
#[cfg(has_atomics)]
impl_atomic!(@multibyte AtomicU32);
#[cfg(has_atomics_64)]
impl_atomic!(@multibyte AtomicU64);

// PhantomData

impl<T: ?Sized> Archive for PhantomData<T> {
    type Archived = PhantomData<T>;
    type Resolver = ();

    #[inline]
    unsafe fn resolve(&self, _: usize, _: Self::Resolver, _: &mut MaybeUninit<Self::Archived>) {}
}

impl<T: ?Sized, S: Fallible + ?Sized> Serialize<S> for PhantomData<T> {
    #[inline]
    fn serialize(&self, _: &mut S) -> Result<Self::Resolver, S::Error> {
        Ok(())
    }
}

impl<T: ?Sized, D: Fallible + ?Sized> Deserialize<PhantomData<T>, D> for PhantomData<T> {
    #[inline]
    fn deserialize(&self, _: &mut D) -> Result<PhantomData<T>, D::Error> {
        Ok(PhantomData)
    }
}

// PhantomPinned
impl Archive for PhantomPinned {
    type Archived = PhantomPinned;
    type Resolver = ();

    #[inline]
    unsafe fn resolve(&self, _: usize, _: Self::Resolver, _: &mut MaybeUninit<Self::Archived>) {}
}

impl<S: Fallible + ?Sized> Serialize<S> for PhantomPinned {
    #[inline]
    fn serialize(&self, _: &mut S) -> Result<Self::Resolver, S::Error> {
        Ok(())
    }
}

impl<D: Fallible + ?Sized> Deserialize<PhantomPinned, D> for PhantomPinned {
    #[inline]
    fn deserialize(&self, _: &mut D) -> Result<PhantomPinned, D::Error> {
        Ok(PhantomPinned)
    }
}

// usize

impl Archive for usize {
    type Archived = Archived<FixedUsize>;
    type Resolver = ();

    #[inline]
    unsafe fn resolve(&self, _: usize, _: Self::Resolver, out: &mut MaybeUninit<Self::Archived>) {
        out.as_mut_ptr().write(to_archived!(*self as FixedUsize));
    }
}

impl<S: Fallible + ?Sized> Serialize<S> for usize {
    #[inline]
    fn serialize(&self, _: &mut S) -> Result<Self::Resolver, S::Error> {
        Ok(())
    }
}

impl<D: Fallible + ?Sized> Deserialize<usize, D> for Archived<usize> {
    #[inline]
    fn deserialize(&self, _: &mut D) -> Result<usize, D::Error> {
        Ok(from_archived!(*self) as usize)
    }
}

// isize

impl Archive for isize {
    type Archived = Archived<FixedIsize>;
    type Resolver = ();

    #[inline]
    unsafe fn resolve(&self, _: usize, _: Self::Resolver, out: &mut MaybeUninit<Self::Archived>) {
        out.as_mut_ptr().write(to_archived!(*self as FixedIsize));
    }
}

impl<S: Fallible + ?Sized> Serialize<S> for isize {
    #[inline]
    fn serialize(&self, _: &mut S) -> Result<Self::Resolver, S::Error> {
        Ok(())
    }
}

impl<D: Fallible + ?Sized> Deserialize<isize, D> for Archived<isize> {
    #[inline]
    fn deserialize(&self, _: &mut D) -> Result<isize, D::Error> {
        Ok(from_archived!(*self) as isize)
    }
}

// NonZeroUsize

type FixedNonZeroUsize = pick_size_type!(NonZeroU16, NonZeroU32, NonZeroU64);

impl Archive for NonZeroUsize {
    type Archived = Archived<FixedNonZeroUsize>;
    type Resolver = ();

    #[inline]
    unsafe fn resolve(&self, _: usize, _: Self::Resolver, out: &mut MaybeUninit<Self::Archived>) {
        out.as_mut_ptr().write(
            to_archived!(FixedNonZeroUsize::new_unchecked(self.get() as FixedUsize))
        );
    }
}

impl<S: Fallible + ?Sized> Serialize<S> for NonZeroUsize {
    #[inline]
    fn serialize(&self, _: &mut S) -> Result<Self::Resolver, S::Error> {
        Ok(())
    }
}

impl<D: Fallible + ?Sized> Deserialize<NonZeroUsize, D> for Archived<NonZeroUsize> {
    #[inline]
    fn deserialize(&self, _: &mut D) -> Result<NonZeroUsize, D::Error> {
        Ok(unsafe {
            NonZeroUsize::new_unchecked(from_archived!(*self).get() as usize)
        })
    }
}

// NonZeroIsize

type FixedNonZeroIsize = pick_size_type!(NonZeroI16, NonZeroI32, NonZeroI64);

impl Archive for NonZeroIsize {
    type Archived = Archived<FixedNonZeroIsize>;
    type Resolver = ();

    #[inline]
    unsafe fn resolve(&self, _: usize, _: Self::Resolver, out: &mut MaybeUninit<Self::Archived>) {
        out.as_mut_ptr().write(
            to_archived!(FixedNonZeroIsize::new_unchecked(self.get() as FixedIsize))
        );
    }
}

impl<S: Fallible + ?Sized> Serialize<S> for NonZeroIsize {
    #[inline]
    fn serialize(&self, _: &mut S) -> Result<Self::Resolver, S::Error> {
        Ok(())
    }
}

impl<D: Fallible + ?Sized> Deserialize<NonZeroIsize, D> for Archived<NonZeroIsize> {
    #[inline]
    fn deserialize(&self, _: &mut D) -> Result<NonZeroIsize, D::Error> {
        Ok(unsafe {
            NonZeroIsize::new_unchecked(from_archived!(*self).get() as isize)
        })
    }
}

// AtomicUsize

type FixedAtomicUsize = pick_size_type!(AtomicU16, AtomicU32, AtomicU64);

impl Archive for AtomicUsize {
    type Archived = Archived<FixedAtomicUsize>;
    type Resolver = ();

    #[inline]
    unsafe fn resolve(&self, _: usize, _: Self::Resolver, out: &mut MaybeUninit<Self::Archived>) {
        (&*out.as_ptr()).store(self.load(Ordering::Relaxed) as FixedUsize, Ordering::Relaxed);
    }
}

impl<S: Fallible + ?Sized> Serialize<S> for AtomicUsize {
    #[inline]
    fn serialize(&self, _: &mut S) -> Result<Self::Resolver, S::Error> {
        Ok(())
    }
}

impl<D: Fallible + ?Sized> Deserialize<AtomicUsize, D> for Archived<AtomicUsize> {
    #[inline]
    fn deserialize(&self, _: &mut D) -> Result<AtomicUsize, D::Error> {
        Ok((self.load(Ordering::Relaxed) as usize).into())
    }
}

// AtomicIsize

type FixedAtomicIsize = pick_size_type!(AtomicI16, AtomicI32, AtomicI64);

impl Archive for AtomicIsize {
    type Archived = Archived<FixedAtomicIsize>;
    type Resolver = ();

    #[inline]
    unsafe fn resolve(&self, _: usize, _: Self::Resolver, out: &mut MaybeUninit<Self::Archived>) {
        (&*out.as_ptr()).store(self.load(Ordering::Relaxed) as FixedIsize, Ordering::Relaxed);
    }
}

impl<S: Fallible + ?Sized> Serialize<S> for AtomicIsize {
    #[inline]
    fn serialize(&self, _: &mut S) -> Result<Self::Resolver, S::Error> {
        Ok(())
    }
}

impl<D: Fallible + ?Sized> Deserialize<AtomicIsize, D> for Archived<AtomicIsize> {
    #[inline]
    fn deserialize(&self, _: &mut D) -> Result<AtomicIsize, D::Error> {
        Ok((self.load(Ordering::Relaxed) as isize).into())
    }
}
