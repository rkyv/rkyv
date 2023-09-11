use crate::{
    with::{
        Acquire, ArchiveWith, AsAtomic, AtomicLoad, DeserializeWith, Relaxed,
        SeqCst, SerializeWith,
    },
    Fallible,
};
use core::sync::atomic::Ordering;
#[cfg(target_has_atomic = "8")]
use core::sync::atomic::{AtomicBool, AtomicI8, AtomicU8};
#[cfg(target_has_atomic = "16")]
use {
    crate::primitive::{
        ArchivedAtomicI16, ArchivedAtomicU16, ArchivedI16, ArchivedU16,
    },
    core::sync::atomic::{AtomicI16, AtomicU16},
};
#[cfg(target_has_atomic = "32")]
use {
    crate::primitive::{
        ArchivedAtomicI32, ArchivedAtomicU32, ArchivedI32, ArchivedU32,
    },
    core::sync::atomic::{AtomicI32, AtomicU32},
};
#[cfg(target_has_atomic = "64")]
use {
    crate::primitive::{
        ArchivedAtomicI64, ArchivedAtomicU64, ArchivedI64, ArchivedU64,
    },
    core::sync::atomic::{AtomicI64, AtomicU64},
};
#[cfg(any(
    all(target_has_atomic = "16", feature = "pointer_width_16"),
    all(target_has_atomic = "32", feature = "pointer_width_32"),
    all(target_has_atomic = "64", feature = "pointer_width_64"),
))]
use {
    crate::primitive::{
        ArchivedAtomicIsize, ArchivedAtomicUsize, ArchivedIsize, ArchivedUsize,
    },
    core::sync::atomic::{AtomicIsize, AtomicUsize},
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

macro_rules! impl_serialize_with_atomic {
    ($atomic:ty) => {
        impl<S, SO> SerializeWith<$atomic, S> for AtomicLoad<SO>
        where
            S: Fallible + ?Sized,
            SO: LoadOrdering,
        {
            #[inline]
            fn serialize_with(
                _: &$atomic,
                _: &mut S,
            ) -> Result<Self::Resolver, S::Error> {
                Ok(())
            }
        }

        impl<S, SO, DO> SerializeWith<$atomic, S> for AsAtomic<SO, DO>
        where
            S: Fallible + ?Sized,
            SO: LoadOrdering,
        {
            #[inline]
            fn serialize_with(
                _: &$atomic,
                _: &mut S,
            ) -> Result<Self::Resolver, S::Error> {
                Ok(())
            }
        }
    };
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

macro_rules! impl_multi_byte_atomic {
    ($atomic:ty, $archived:ty, $archived_non_atomic:ty) => {
        impl<SO: LoadOrdering> ArchiveWith<$atomic> for AtomicLoad<SO> {
            type Archived = $archived_non_atomic;
            type Resolver = ();

            #[inline]
            unsafe fn resolve_with(
                field: &$atomic,
                _: usize,
                _: Self::Resolver,
                out: *mut Self::Archived,
            ) {
                out.write(<$archived_non_atomic>::from_native(
                    field.load(SO::ORDERING),
                ));
            }
        }

        impl<SO, DO> ArchiveWith<$atomic> for AsAtomic<SO, DO>
        where
            SO: LoadOrdering,
        {
            type Archived = $archived;
            type Resolver = ();

            #[inline]
            unsafe fn resolve_with(
                field: &$atomic,
                _: usize,
                _: Self::Resolver,
                out: *mut Self::Archived,
            ) {
                out.write(<$archived>::new(field.load(SO::ORDERING)))
            }
        }

        impl_serialize_with_atomic!($atomic);

        impl<D, SO> DeserializeWith<$archived_non_atomic, $atomic, D>
            for AtomicLoad<SO>
        where
            D: Fallible + ?Sized,
        {
            #[inline]
            fn deserialize_with(
                field: &$archived_non_atomic,
                _: &mut D,
            ) -> Result<$atomic, D::Error> {
                Ok(<$atomic>::new(field.to_native()))
            }
        }

        impl<D, SO, DO> DeserializeWith<$archived, $atomic, D>
            for AsAtomic<SO, DO>
        where
            D: Fallible + ?Sized,
            DO: LoadOrdering,
        {
            #[inline]
            fn deserialize_with(
                field: &$archived,
                _: &mut D,
            ) -> Result<$atomic, D::Error> {
                Ok(<$atomic>::new(field.load(DO::ORDERING)))
            }
        }
    };
}

macro_rules! impl_multi_byte_atomics {
    ($($atomic:ty, $archived:ty, $archived_non_atomic:ty);* $(;)?) => {
        $(
            impl_multi_byte_atomic!($atomic, $archived, $archived_non_atomic);
        )*
    }
}

#[cfg(target_has_atomic = "16")]
impl_multi_byte_atomics! {
    AtomicI16, ArchivedAtomicI16, ArchivedI16;
    AtomicU16, ArchivedAtomicU16, ArchivedU16;
}
#[cfg(target_has_atomic = "32")]
impl_multi_byte_atomics! {
    AtomicI32, ArchivedAtomicI32, ArchivedI32;
    AtomicU32, ArchivedAtomicU32, ArchivedU32;
}
#[cfg(target_has_atomic = "64")]
impl_multi_byte_atomics! {
    AtomicI64, ArchivedAtomicI64, ArchivedI64;
    AtomicU64, ArchivedAtomicU64, ArchivedU64;
}

// AtomicUsize

macro_rules! impl_atomic_size_type {
    ($atomic:ty, $archived:ty, $archived_non_atomic:ty) => {
        impl<SO: LoadOrdering> ArchiveWith<$atomic> for AtomicLoad<SO> {
            type Archived = $archived_non_atomic;
            type Resolver = ();

            #[inline]
            unsafe fn resolve_with(
                field: &$atomic,
                _: usize,
                _: Self::Resolver,
                out: *mut Self::Archived,
            ) {
                out.write(<$archived_non_atomic>::from_native(
                    field.load(SO::ORDERING) as _,
                ));
            }
        }

        impl<SO, DO> ArchiveWith<$atomic> for AsAtomic<SO, DO>
        where
            SO: LoadOrdering,
        {
            type Archived = $archived;
            type Resolver = ();

            #[inline]
            unsafe fn resolve_with(
                field: &$atomic,
                _: usize,
                _: Self::Resolver,
                out: *mut Self::Archived,
            ) {
                out.write(<$archived>::new(field.load(SO::ORDERING) as _));
            }
        }

        impl_serialize_with_atomic!($atomic);

        impl<D, SO> DeserializeWith<$archived_non_atomic, $atomic, D>
            for AtomicLoad<SO>
        where
            D: Fallible + ?Sized,
        {
            #[inline]
            fn deserialize_with(
                field: &$archived_non_atomic,
                _: &mut D,
            ) -> Result<$atomic, D::Error> {
                Ok(<$atomic>::new(field.to_native() as _))
            }
        }

        impl<D, SO, DO> DeserializeWith<$archived, $atomic, D>
            for AsAtomic<SO, DO>
        where
            D: Fallible + ?Sized,
            DO: LoadOrdering,
        {
            #[inline]
            fn deserialize_with(
                field: &$archived,
                _: &mut D,
            ) -> Result<$atomic, D::Error> {
                Ok(<$atomic>::new(field.load(DO::ORDERING) as _))
            }
        }
    };
}

macro_rules! impl_atomic_size_types {
    ($($atomic:ty, $archived:ty, $archived_non_atomic:ty;)*) => {
        $(
            impl_atomic_size_type!($atomic, $archived, $archived_non_atomic);
        )*
    }
}

#[cfg(any(
    all(target_has_atomic = "16", feature = "pointer_width_16"),
    all(target_has_atomic = "32", feature = "pointer_width_32"),
    all(target_has_atomic = "64", feature = "pointer_width_64"),
))]
impl_atomic_size_types! {
    AtomicIsize, ArchivedAtomicIsize, ArchivedIsize;
    AtomicUsize, ArchivedAtomicUsize, ArchivedUsize;
}

// TODO: provide impls for rend atomics
