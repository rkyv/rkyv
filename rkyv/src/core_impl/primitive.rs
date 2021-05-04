use crate::{
    Archive, ArchiveCopy, Archived, ArchivedIsize, ArchivedUsize, Fallible, FixedIsize, FixedUsize,
    Serialize, Deserialize,
};
use core::{
    marker::PhantomData,
    mem::MaybeUninit,
    num::{
        NonZeroI8, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI128, NonZeroU8, NonZeroU16,
        NonZeroU32, NonZeroU64, NonZeroU128,
    },
};
#[cfg(has_atomics)]
use core::sync::atomic::{
    AtomicBool, AtomicI16, AtomicI32, AtomicI8, AtomicU16, AtomicU32, AtomicU8, Ordering,
};
#[cfg(has_atomics_64)]
use core::sync::atomic::{AtomicI64, AtomicU64};

macro_rules! impl_primitive {
    (@serialize_deserialize $type:ty) => {
        impl<S: Fallible + ?Sized> Serialize<S> for $type {
            #[inline]
            fn serialize(&self, _: &mut S) -> Result<Self::Resolver, S::Error> {
                Ok(())
            }
        }

        impl<D: Fallible + ?Sized> Deserialize<$type, D> for Archived<$type> {
            #[inline]
            fn deserialize(&self, _: &mut D) -> Result<$type, D::Error> {
                Ok((*self).into())
            }
        }
    };
    ($type:ty) => {
        impl Archive for $type {
            type Archived = Self;
            type Resolver = ();

            #[inline]
            fn resolve(&self, _: usize, _: Self::Resolver, out: &mut MaybeUninit<Self::Archived>) {
                unsafe {
                    out.as_mut_ptr().write((*self).into());
                }
            }
        }

        impl_primitive!(@serialize_deserialize $type);

        unsafe impl ArchiveCopy for $type {}
    };
    ($type:ty, $type_le:ident, $type_be:ident) => {
        impl Archive for $type {
            #[cfg(not(any(feature = "archive_le", feature = "archive_be")))]
            type Archived = Self;
            #[cfg(feature = "archive_le")]
            type Archived = rend::$type_le;
            #[cfg(feature = "archive_be")]
            type Archived = rend::$type_be;

            type Resolver = ();

            #[inline]
            fn resolve(&self, _: usize, _: Self::Resolver, out: &mut MaybeUninit<Self::Archived>) {
                unsafe {
                    out.as_mut_ptr().write((*self).into());
                }
            }
        }

        impl_primitive!(@serialize_deserialize $type);

        #[cfg(not(any(feature = "archive_le", feature = "archive_be")))]
        unsafe impl ArchiveCopy for $type {}
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

        impl<D: Fallible + ?Sized> Deserialize<$type, D> for Archived<$type> {
            #[inline]
            fn deserialize(&self, _: &mut D) -> Result<$type, D::Error> {
                Ok(self.load(Ordering::Relaxed).into())
            }
        }
    };
    ($type:ty) => {
        impl Archive for $type {
            type Archived = Self;
            type Resolver = ();

            #[inline]
            fn resolve(&self, _: usize, _: Self::Resolver, out: &mut MaybeUninit<Self::Archived>) {
                unsafe {
                    (&mut *out.as_mut_ptr()).store(self.load(Ordering::Relaxed), Ordering::Relaxed);
                }
            }
        }

        impl_atomic!(@serialize_deserialize $type);
    };
    ($type:ty, $type_le:ident, $type_be:ident) => {
        impl Archive for $type {
            #[cfg(not(any(feature = "archive_le", feature = "archive_be")))]
            type Archived = Self;
            #[cfg(feature = "archive_le")]
            type Archived = rend::$type_le;
            #[cfg(feature = "archive_be")]
            type Archived = rend::$type_be;

            type Resolver = ();

            #[inline]
            fn resolve(&self, _: usize, _: Self::Resolver, out: &mut MaybeUninit<Self::Archived>) {
                unsafe {
                    (&mut *out.as_mut_ptr()).store(self.load(Ordering::Relaxed), Ordering::Relaxed);
                }
            }
        }

        impl_atomic!(@serialize_deserialize $type);
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

impl_primitive!(i16, i16_le, i16_be);
impl_primitive!(i32, i32_le, i32_be);
impl_primitive!(i64, i64_le, i64_be);
impl_primitive!(i128, i128_le, i128_be);
impl_primitive!(u16, u16_le, u16_be);
impl_primitive!(u32, u32_le, u32_be);
impl_primitive!(u64, u64_le, u64_be);
impl_primitive!(u128, u128_le, u128_be);

impl_primitive!(f32, f32_le, f32_be);
impl_primitive!(f64, f64_le, f64_be);

impl_primitive!(char, char_le, char_be);

impl_primitive!(NonZeroI16, NonZeroI16_le, NonZeroI16_be);
impl_primitive!(NonZeroI32, NonZeroI32_le, NonZeroI32_be);
impl_primitive!(NonZeroI64, NonZeroI64_le, NonZeroI64_be);
impl_primitive!(NonZeroI128, NonZeroI128_le, NonZeroI128_be);
impl_primitive!(NonZeroU16, NonZeroU16_le, NonZeroU16_be);
impl_primitive!(NonZeroU32, NonZeroU32_le, NonZeroU32_be);
impl_primitive!(NonZeroU64, NonZeroU64_le, NonZeroU64_be);
impl_primitive!(NonZeroU128, NonZeroU128_le, NonZeroU128_be);

#[cfg(has_atomics)]
impl_atomic!(AtomicI16, AtomicI16_le, AtomicI16_be);
#[cfg(has_atomics)]
impl_atomic!(AtomicI32, AtomicI32_le, AtomicI32_be);
#[cfg(has_atomics_64)]
impl_atomic!(AtomicI64, AtomicI64_le, AtomicI64_be);
#[cfg(has_atomics)]
impl_atomic!(AtomicU16, AtomicU16_le, AtomicU16_be);
#[cfg(has_atomics)]
impl_atomic!(AtomicU32, AtomicU32_le, AtomicU32_be);
#[cfg(has_atomics_64)]
impl_atomic!(AtomicU64, AtomicU64_le, AtomicU64_be);

// PhantomData

impl<T: ?Sized> Archive for PhantomData<T> {
    type Archived = PhantomData<T>;
    type Resolver = ();

    #[inline]
    fn resolve(&self, _: usize, _: Self::Resolver, _: &mut MaybeUninit<Self::Archived>) {
    }
}

impl<T: ?Sized, S: Fallible + ?Sized> Serialize<S> for PhantomData<T> {
    #[inline]
    fn serialize(&self, _: &mut S) -> Result<Self::Resolver, S::Error> {
        Ok(())
    }
}

unsafe impl<T: ?Sized> ArchiveCopy for PhantomData<T> {}

impl<T: ?Sized, D: Fallible + ?Sized> Deserialize<PhantomData<T>, D> for PhantomData<T> {
    #[inline]
    fn deserialize(&self, _: &mut D) -> Result<PhantomData<T>, D::Error> {
        Ok(PhantomData)
    }
}

// usize

impl Archive for usize {
    type Archived = ArchivedUsize;
    type Resolver = ();

    #[inline]
    fn resolve(&self, _: usize, _: Self::Resolver, out: &mut MaybeUninit<Self::Archived>) {
        unsafe {
            out.as_mut_ptr().write((*self as FixedUsize).into());
        }
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
        Ok(FixedUsize::from(*self) as usize)
    }
}

// isize

impl Archive for isize {
    type Archived = ArchivedIsize;
    type Resolver = ();

    #[inline]
    fn resolve(&self, _: usize, _: Self::Resolver, out: &mut MaybeUninit<Self::Archived>) {
        unsafe {
            out.as_mut_ptr().write((*self as FixedIsize).into());
        }
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
        Ok(FixedIsize::from(*self) as isize)
    }
}
