use crate::{
    Archive, ArchiveCopy, Archived, ArchivedIsize, ArchivedUsize, Fallible, FixedIsize, FixedUsize,
    Serialize, Deserialize,
};
use core::{
    marker::PhantomData,
    num::{
        NonZeroI8,
        NonZeroI16,
        NonZeroI32,
        NonZeroI64,
        NonZeroI128,
        NonZeroU8,
        NonZeroU16,
        NonZeroU32,
        NonZeroU64,
        NonZeroU128,
    },
};
#[cfg(rkyv_atomic)]
use core::sync::atomic::{
    self, AtomicBool, AtomicI16, AtomicI32, AtomicI8, AtomicU16, AtomicU32, AtomicU8,
};
#[cfg(rkyv_atomic_64)]
use core::sync::atomic::{AtomicI64, AtomicU64};

macro_rules! impl_primitive {
    ($type:ty) => {
        impl Archive for $type
        where
            $type: Copy,
        {
            type Archived = Self;
            type Resolver = ();

            #[inline]
            fn resolve(&self, _: usize, _: Self::Resolver) -> Self::Archived {
                *self
            }
        }

        impl<S: Fallible + ?Sized> Serialize<S> for $type {
            #[inline]
            fn serialize(&self, _: &mut S) -> Result<Self::Resolver, S::Error> {
                Ok(())
            }
        }

        unsafe impl ArchiveCopy for $type {}

        impl<D: Fallible + ?Sized> Deserialize<$type, D> for $type
        where
            $type: Copy,
        {
            #[inline]
            fn deserialize(&self, _: &mut D) -> Result<$type, D::Error> {
                Ok(*self)
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

macro_rules! impl_integer {
    ($ne:ty, $le:ident, $be:ident) => {
        #[repr(transparent)]
        pub struct $le($ne);

        impl From<$le> for $ne {
            #[cfg(target_endian = "little")]
            #[inline]
            fn from(value: $le) -> Self {
                value.0
            }

            #[cfg(target_endian = "big")]
            #[inline]
            fn from(value: $le) -> Self {
                value.0.swap_bytes()
            }
        }

        impl From<$ne> for $le {
            #[cfg(target_endian = "little")]
            #[inline]
            fn from(value: $ne) -> Self {
                $le(value)
            }

            #[cfg(target_endian = "big")]
            #[inline]
            fn from(value: $ne) -> Self {
                $le(value.swap_bytes())
            }
        }

        #[repr(transparent)]
        pub struct $be($ne);

        impl From<$be> for $ne {
            #[cfg(target_endian = "little")]
            #[inline]
            fn from(value: $be) -> Self {
                value.0.swap_bytes()
            }
            
            #[cfg(target_endian = "big")]
            #[inline]
            fn from(value: $be) -> Self {
                value.0
            }
        }

        impl From<$ne> for $be {
            #[cfg(target_endian = "little")]
            #[inline]
            fn from(value: $ne) -> Self {
                $be(value.swap_bytes())
            }

            #[cfg(target_endian = "big")]
            #[inline]
            fn from(value: $ne) -> Self {
                $be(value)
            }
        }

        impl Archive for $ne {
            #[cfg(not(any(feature = "archive_le", feature = "archive_be")))]
            type Archived = $ne;
            #[cfg(feature = "archive_le")]
            type Archived = $le;
            #[cfg(feature = "archive_be")]
            type Archived = $be;

            type Resolver = ();

            #[inline]
            fn resolve(&self, _: usize, _: Self::Resolver) -> Self::Archived {
                (*self).into()
            }
        }

        impl<S: Fallible + ?Sized> Serialize<S> for $ne {
            #[inline]
            fn serialize(&self, _: &mut S) -> Result<Self::Resolver, S::Error> {
                Ok(())
            }
        }

        #[cfg(any(
            not(any(feature = "archive_le", feature = "archive_be")),
            all(feature = "archive_le", target_endian = "little"),
            all(feature = "archive_be", target_endian = "big")
        ))]
        unsafe impl ArchiveCopy for $ne {}

        impl<D: Fallible + ?Sized> Deserialize<$ne, D> for Archived<$ne> {
            #[inline]
            fn deserialize(&self, _: &mut D) -> Result<$ne, D::Error> {
                Ok((*self).into())
            }
        }
    }
}

impl_integer!(i16, I16LE, I16BE);
impl_integer!(i32, I32LE, I32BE);
impl_integer!(i64, I64LE, I64BE);
impl_integer!(i128, I128LE, I128BE);
impl_integer!(u16, U16LE, U16BE);
impl_integer!(u32, U32LE, U32BE);
impl_integer!(u64, U64LE, U64BE);
impl_integer!(u128, U128LE, U128BE);

macro_rules! impl_float {
    ($ne:ty, $le:ident, $be:ident, $width:expr) => {
        #[repr(transparent)]
        pub struct $le([u8; $width]);

        impl From<$le> for $ne {
            #[inline]
            fn from(value: $le) -> Self {
                <$ne>::from_le_bytes(value.0)
            }
        }

        impl From<$ne> for $le {
            #[inline]
            fn from(value: $ne) -> Self {
                $le(value.to_le_bytes())
            }
        }

        #[repr(transparent)]
        pub struct $be([u8; $width]);

        impl From<$be> for $ne {
            #[inline]
            fn from(value: $be) -> Self {
                <$ne>::from_be_bytes(value.0)
            }
        }

        impl From<$ne> for $be {
            #[inline]
            fn from(value: $ne) -> Self {
                $be(value.to_be_bytes())
            }
        }

        impl Archive for $ne {
            #[cfg(not(any(feature = "archive_le", feature = "archive_be")))]
            type Archived = $ne;
            #[cfg(feature = "archive_le")]
            type Archived = $le;
            #[cfg(feature = "archive_be")]
            type Archived = $be;

            type Resolver = ();

            #[inline]
            fn resolve(&self, _: usize, _: Self::Resolver) -> Self::Archived {
                (*self).into()
            }
        }

        impl<S: Fallible + ?Sized> Serialize<S> for $ne {
            #[inline]
            fn serialize(&self, _: &mut S) -> Result<Self::Resolver, S::Error> {
                Ok(())
            }
        }

        #[cfg(any(
            not(any(feature = "archive_le", feature = "archive_be")),
            all(feature = "archive_le", target_endian = "little"),
            all(feature = "archive_be", target_endian = "big")
        ))]
        unsafe impl ArchiveCopy for $ne {}

        impl<D: Fallible + ?Sized> Deserialize<$ne, D> for Archived<$ne> {
            #[inline]
            fn deserialize(&self, _: &mut D) -> Result<$ne, D::Error> {
                Ok((*self).into())
            }
        }
    }
}

impl_float!(f32, F32LE, F32BE, 4);
impl_float!(f64, F64LE, F64BE, 8);

#[repr(transparent)]
pub struct CharLE(u32);

impl From<CharLE> for char {
    #[cfg(target_endian = "little")]
    #[inline]
    fn from(value: CharLE) -> Self {
        unsafe { core::char::from_u32_unchecked(value.0) }
    }

    #[cfg(target_endian = "big")]
    #[inline]
    fn from(value: CharLE) -> Self {
        unsafe { core::char::from_u32_unchecked(value.0.swap_bytes()) }
    }
}

impl From<char> for CharLE {
    #[cfg(target_endian = "little")]
    #[inline]
    fn from(value: char) -> Self {
        CharLE(value as u32)
    }

    #[cfg(target_endian = "big")]
    #[inline]
    fn from(value: char) -> Self {
        CharLE((value as u32).swap_bytes())
    }
}

#[repr(transparent)]
pub struct CharBE(u32);

impl From<CharBE> for char {
    #[cfg(target_endian = "little")]
    #[inline]
    fn from(value: CharBE) -> Self {
        unsafe { core::char::from_u32_unchecked(value.0.swap_bytes()) }
    }

    #[cfg(target_endian = "big")]
    #[inline]
    fn from(value: CharBE) -> Self {
        unsafe { core::char::from_u32_unchecked(value.0) }
    }
}

impl From<char> for CharBE {
    #[cfg(target_endian = "little")]
    #[inline]
    fn from(value: char) -> Self {
        CharBE((value as u32).swap_bytes())
    }

    #[cfg(target_endian = "big")]
    #[inline]
    fn from(value: char) -> Self {
        CharBE(value as u32)
    }
}

impl Archive for char {
    #[cfg(not(any(feature = "archive_le", feature = "archive_be")))]
    type Archived = char;
    #[cfg(feature = "archive_le")]
    type Archived = CharLE;
    #[cfg(feature = "archive_be")]
    type Archived = CharBE;

    type Resolver = ();

    #[inline]
    fn resolve(&self, _: usize, _: Self::Resolver) -> Self::Archived {
        (*self).into()
    }
}

impl<S: Fallible + ?Sized> Serialize<S> for char {
    #[inline]
    fn serialize(&self, _: &mut S) -> Result<Self::Resolver, S::Error> {
        Ok(())
    }
}

#[cfg(any(
    not(any(feature = "archive_le", feature = "archive_be")),
    all(feature = "archive_le", target_endian = "little"),
    all(feature = "archive_be", target_endian = "big")
))]
unsafe impl ArchiveCopy for char {}

impl<D: Fallible + ?Sized> Deserialize<char, D> for Archived<char> {
    #[inline]
    fn deserialize(&self, _: &mut D) -> Result<char, D::Error> {
        Ok((*self).into())
    }
}

macro_rules! impl_nonzero {
    ($ne:ty, $le:ident, $be:ident) => {
        #[repr(transparent)]
        pub struct $le($ne);

        impl From<$le> for $ne {
            #[cfg(target_endian = "little")]
            #[inline]
            fn from(value: $le) -> Self {
                value.0
            }

            #[cfg(target_endian = "big")]
            #[inline]
            fn from(value: $le) -> Self {
                unsafe { <$ne>::new_unchecked(value.0.get().swap_bytes()) }
            }
        }

        impl From<$ne> for $le {
            #[cfg(target_endian = "little")]
            #[inline]
            fn from(value: $ne) -> Self {
                $le(value)
            }

            #[cfg(target_endian = "big")]
            #[inline]
            fn from(value: $ne) -> Self {
                unsafe { $le(<$ne>::new_unchecked(value.get().swap_bytes())) }
            }
        }

        #[repr(transparent)]
        pub struct $be($ne);

        impl From<$be> for $ne {
            #[cfg(target_endian = "little")]
            #[inline]
            fn from(value: $be) -> Self {
                unsafe { <$ne>::new_unchecked(value.0.get().swap_bytes()) }
            }

            #[cfg(target_endian = "big")]
            #[inline]
            fn from(value: $be) -> Self {
                value.0
            }
        }

        impl From<$ne> for $be {
            #[cfg(target_endian = "little")]
            #[inline]
            fn from(value: $ne) -> Self {
                unsafe { $be(<$ne>::new_unchecked(value.get().swap_bytes())) }
            }

            #[cfg(target_endian = "big")]
            #[inline]
            fn from(value: $ne) -> Self {
                $be(value)
            }
        }

        impl Archive for $ne {
            #[cfg(not(any(feature = "archive_le", feature = "archive_be")))]
            type Archived = $ne;
            #[cfg(feature = "archive_le")]
            type Archived = $le;
            #[cfg(feature = "archive_be")]
            type Archived = $be;

            type Resolver = ();

            #[inline]
            fn resolve(&self, _: usize, _: Self::Resolver) -> Self::Archived {
                (*self).into()
            }
        }

        #[cfg(any(
            not(any(feature = "archive_le", feature = "archive_be")),
            all(feature = "archive_le", target_endian = "little"),
            all(feature = "archive_be", target_endian = "big")
        ))]
        unsafe impl ArchiveCopy for $ne {}

        impl<D: Fallible + ?Sized> Deserialize<$ne, D> for Archived<$ne> {
            #[inline]
            fn deserialize(&self, _: &mut D) -> Result<$ne, D::Error> {
                Ok((*self).into())
            }
        }
    }
}

impl_nonzero!(NonZeroI16, NonZeroI16LE, NonZeroI16BE);
impl_nonzero!(NonZeroI32, NonZeroI32LE, NonZeroI32BE);
impl_nonzero!(NonZeroI64, NonZeroI64LE, NonZeroI64BE);
impl_nonzero!(NonZeroI128, NonZeroI128LE, NonZeroI128BE);
impl_nonzero!(NonZeroU16, NonZeroU16LE, NonZeroU16BE);
impl_nonzero!(NonZeroU32, NonZeroU32LE, NonZeroU32BE);
impl_nonzero!(NonZeroU64, NonZeroU64LE, NonZeroU64BE);
impl_nonzero!(NonZeroU128, NonZeroU128LE, NonZeroU128BE);

impl<T: ?Sized> Archive for PhantomData<T> {
    type Archived = PhantomData<T>;
    type Resolver = ();

    #[inline]
    fn resolve(&self, _: usize, _: Self::Resolver) -> Self::Archived {
        PhantomData
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

impl Archive for usize {
    type Archived = ArchivedUsize;
    type Resolver = ();

    #[inline]
    fn resolve(&self, _: usize, _: Self::Resolver) -> Self::Archived {
        (*self as FixedUsize).into()
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

impl Archive for isize {
    type Archived = ArchivedIsize;
    type Resolver = ();

    #[inline]
    fn resolve(&self, _: usize, _: Self::Resolver) -> Self::Archived {
        (*self as FixedIsize).into()
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

// TODO: fix these

/// The resolver for atomic types.
pub struct AtomicResolver;

#[cfg(rkyv_atomic)]
macro_rules! impl_atomic {
    ($type:ty) => {
        impl Archive for $type {
            type Archived = Self;
            type Resolver = AtomicResolver;

            #[inline]
            fn resolve(&self, _pos: usize, _resolver: AtomicResolver) -> $type {
                <$type>::new(self.load(atomic::Ordering::Relaxed))
            }
        }

        impl<S: Fallible + ?Sized> Serialize<S> for $type {
            #[inline]
            fn serialize(&self, _: &mut S) -> Result<Self::Resolver, S::Error> {
                Ok(AtomicResolver)
            }
        }

        impl<D: Fallible + ?Sized> Deserialize<$type, D> for $type {
            #[inline]
            fn deserialize(&self, _: &mut D) -> Result<$type, D::Error> {
                Ok(<$type>::new(self.load(atomic::Ordering::Relaxed)))
            }
        }
    };
}

#[cfg(rkyv_atomic)]
impl_atomic!(AtomicBool);
#[cfg(rkyv_atomic)]
impl_atomic!(AtomicI8);
#[cfg(rkyv_atomic)]
impl_atomic!(AtomicI16);
#[cfg(rkyv_atomic)]
impl_atomic!(AtomicI32);
#[cfg(rkyv_atomic_64)]
impl_atomic!(AtomicI64);
#[cfg(rkyv_atomic)]
impl_atomic!(AtomicU8);
#[cfg(rkyv_atomic)]
impl_atomic!(AtomicU16);
#[cfg(rkyv_atomic)]
impl_atomic!(AtomicU32);
#[cfg(rkyv_atomic_64)]
impl_atomic!(AtomicU64);