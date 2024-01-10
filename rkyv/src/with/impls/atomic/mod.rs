#[macro_use]
mod _macros;
#[cfg(not(feature = "unaligned"))]
mod multibyte;

use crate::with::{
    Acquire, ArchiveWith, AsAtomic, AtomicLoad, DeserializeWith, Relaxed,
    SeqCst,
};
use core::sync::atomic::Ordering;
#[cfg(target_has_atomic = "8")]
use core::sync::atomic::{AtomicBool, AtomicI8, AtomicU8};
use rancor::Fallible;

trait LoadOrdering {
    const ORDERING: Ordering;
}

impl LoadOrdering for Relaxed {
    const ORDERING: Ordering = Ordering::Relaxed;
}

impl LoadOrdering for Acquire {
    const ORDERING: Ordering = Ordering::Acquire;
}

impl LoadOrdering for SeqCst {
    const ORDERING: Ordering = Ordering::SeqCst;
}

macro_rules! impl_single_byte_atomic {
    ($atomic:ty, $non_atomic:ty) => {
        impl<SO: LoadOrdering> ArchiveWith<$atomic> for AtomicLoad<SO> {
            type Archived = $non_atomic;
            type Resolver = ();

            #[inline]
            unsafe fn resolve_with(
                field: &$atomic,
                _: usize,
                _: Self::Resolver,
                out: *mut Self::Archived,
            ) {
                out.write(field.load(SO::ORDERING));
            }
        }

        impl<SO, DO> ArchiveWith<$atomic> for AsAtomic<SO, DO>
        where
            SO: LoadOrdering,
        {
            type Archived = $atomic;
            type Resolver = ();

            #[inline]
            unsafe fn resolve_with(
                field: &$atomic,
                _: usize,
                _: Self::Resolver,
                out: *mut Self::Archived,
            ) {
                out.write(<$atomic>::new(field.load(SO::ORDERING)))
            }
        }

        impl_serialize_with_atomic!($atomic);

        impl<D, SO> DeserializeWith<$non_atomic, $atomic, D> for AtomicLoad<SO>
        where
            D: Fallible + ?Sized,
        {
            #[inline]
            fn deserialize_with(
                field: &$non_atomic,
                _: &mut D,
            ) -> Result<$atomic, D::Error> {
                Ok(<$atomic>::new(*field))
            }
        }

        impl<D, SO, DO> DeserializeWith<$atomic, $atomic, D>
            for AsAtomic<SO, DO>
        where
            D: Fallible + ?Sized,
            DO: LoadOrdering,
        {
            #[inline]
            fn deserialize_with(
                field: &$atomic,
                _: &mut D,
            ) -> Result<$atomic, D::Error> {
                Ok(<$atomic>::new(field.load(DO::ORDERING)))
            }
        }
    };
}

macro_rules! impl_single_byte_atomics {
    ($($atomic:ty, $non_atomic:ty);* $(;)?) => {
        $(
            impl_single_byte_atomic!($atomic, $non_atomic);
        )*
    }
}

#[cfg(target_has_atomic = "8")]
impl_single_byte_atomics!(
    AtomicBool, bool;
    AtomicI8, i8;
    AtomicU8, u8;
);
