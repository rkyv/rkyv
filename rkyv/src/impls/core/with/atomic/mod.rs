#[macro_use]
mod _macros;
#[cfg(any(
    target_has_atomic = "16",
    target_has_atomic = "32",
    target_has_atomic = "64",
))]
mod multibyte;

use core::sync::atomic::Ordering;
#[cfg(target_has_atomic = "8")]
use core::sync::atomic::{AtomicBool, AtomicI8, AtomicU8};

use rancor::Fallible;

use crate::{
    with::{
        Acquire, ArchiveWith, AtomicLoad, DeserializeWith, Relaxed, SeqCst,
    },
    Place,
};

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

            fn resolve_with(
                field: &$atomic,
                _: Self::Resolver,
                out: Place<Self::Archived>,
            ) {
                out.write(field.load(SO::ORDERING));
            }
        }

        impl_serialize_with_atomic_load!($atomic);

        impl<D, SO> DeserializeWith<$non_atomic, $atomic, D> for AtomicLoad<SO>
        where
            D: Fallible + ?Sized,
        {
            fn deserialize_with(
                field: &$non_atomic,
                _: &mut D,
            ) -> Result<$atomic, D::Error> {
                Ok(<$atomic>::new(*field))
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
