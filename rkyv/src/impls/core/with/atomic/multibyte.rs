#[cfg(not(feature = "unaligned"))]
use crate::with::AsAtomic;
use crate::{
    impls::core::with::atomic::LoadOrdering,
    rancor::Fallible,
    with::{ArchiveWith, AtomicLoad, DeserializeWith},
    Place,
};

macro_rules! impl_multi_byte_atomic {
    ($atomic:ty, $archived:ty, $archived_non_atomic:ty) => {
        impl<SO: LoadOrdering> ArchiveWith<$atomic> for AtomicLoad<SO> {
            type Archived = $archived_non_atomic;
            type Resolver = ();

            fn resolve_with(
                field: &$atomic,
                _: Self::Resolver,
                out: Place<Self::Archived>,
            ) {
                out.write(<$archived_non_atomic>::from_native(
                    field.load(SO::ORDERING),
                ));
            }
        }

        impl_serialize_with_atomic_load!($atomic);

        impl<D, SO> DeserializeWith<$archived_non_atomic, $atomic, D>
            for AtomicLoad<SO>
        where
            D: Fallible + ?Sized,
        {
            fn deserialize_with(
                field: &$archived_non_atomic,
                _: &mut D,
            ) -> Result<$atomic, D::Error> {
                Ok(<$atomic>::new(field.to_native()))
            }
        }

        #[cfg(not(feature = "unaligned"))]
        impl<SO, DO> ArchiveWith<$atomic> for AsAtomic<SO, DO>
        where
            SO: LoadOrdering,
        {
            type Archived = $archived;
            type Resolver = ();

            fn resolve_with(
                field: &$atomic,
                _: Self::Resolver,
                out: Place<Self::Archived>,
            ) {
                out.write(<$archived>::new(field.load(SO::ORDERING)))
            }
        }

        #[cfg(not(feature = "unaligned"))]
        impl_serialize_with_as_atomic!($atomic);

        #[cfg(not(feature = "unaligned"))]
        impl<D, SO, DO> DeserializeWith<$archived, $atomic, D>
            for AsAtomic<SO, DO>
        where
            D: Fallible + ?Sized,
            DO: LoadOrdering,
        {
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
    ($($atomic:ty, $archived: ty, $archived_non_atomic:ty);* $(;)?) => {
        $(
            impl_multi_byte_atomic!($atomic, $archived, $archived_non_atomic);
        )*
    }
}

#[cfg(target_has_atomic = "16")]
impl_multi_byte_atomics! {
    core::sync::atomic::AtomicI16,
    crate::primitive::ArchivedAtomicI16,
    crate::primitive::ArchivedI16;

    core::sync::atomic::AtomicU16,
    crate::primitive::ArchivedAtomicU16,
    crate::primitive::ArchivedU16;

    rend::AtomicI16_le, rend::AtomicI16_le, crate::primitive::ArchivedI16;
    rend::AtomicI16_be, rend::AtomicI16_be, crate::primitive::ArchivedI16;
    rend::AtomicU16_le, rend::AtomicU16_le, crate::primitive::ArchivedU16;
    rend::AtomicU16_be, rend::AtomicU16_be, crate::primitive::ArchivedU16;
}
#[cfg(target_has_atomic = "32")]
impl_multi_byte_atomics! {
    core::sync::atomic::AtomicI32,
    crate::primitive::ArchivedAtomicI32,
    crate::primitive::ArchivedI32;

    core::sync::atomic::AtomicU32,
    crate::primitive::ArchivedAtomicU32,
    crate::primitive::ArchivedU32;

    rend::AtomicI32_le, rend::AtomicI32_le, crate::primitive::ArchivedI32;
    rend::AtomicI32_be, rend::AtomicI32_be, crate::primitive::ArchivedI32;
    rend::AtomicU32_le, rend::AtomicU32_le, crate::primitive::ArchivedU32;
    rend::AtomicU32_be, rend::AtomicU32_be, crate::primitive::ArchivedU32;
}
#[cfg(target_has_atomic = "64")]
impl_multi_byte_atomics! {
    core::sync::atomic::AtomicI64,
    crate::primitive::ArchivedAtomicI64,
    crate::primitive::ArchivedI64;

    core::sync::atomic::AtomicU64,
    crate::primitive::ArchivedAtomicU64,
    crate::primitive::ArchivedU64;

    rend::AtomicI64_le, rend::AtomicI64_le, crate::primitive::ArchivedI64;
    rend::AtomicI64_be, rend::AtomicI64_be, crate::primitive::ArchivedI64;
    rend::AtomicU64_le, rend::AtomicU64_le, crate::primitive::ArchivedU64;
    rend::AtomicU64_be, rend::AtomicU64_be, crate::primitive::ArchivedU64;
}

// AtomicUsize

macro_rules! impl_atomic_size_type {
    ($atomic:ty, $archived:ty, $archived_non_atomic:ty) => {
        impl<SO: LoadOrdering> ArchiveWith<$atomic> for AtomicLoad<SO> {
            type Archived = $archived_non_atomic;
            type Resolver = ();

            fn resolve_with(
                field: &$atomic,
                _: Self::Resolver,
                out: Place<Self::Archived>,
            ) {
                out.write(<$archived_non_atomic>::from_native(
                    field.load(SO::ORDERING) as _,
                ));
            }
        }

        impl_serialize_with_atomic_load!($atomic);

        impl<D, SO> DeserializeWith<$archived_non_atomic, $atomic, D>
            for AtomicLoad<SO>
        where
            D: Fallible + ?Sized,
        {
            fn deserialize_with(
                field: &$archived_non_atomic,
                _: &mut D,
            ) -> Result<$atomic, D::Error> {
                Ok(<$atomic>::new(field.to_native() as _))
            }
        }

        #[cfg(not(feature = "unaligned"))]
        impl<SO, DO> ArchiveWith<$atomic> for AsAtomic<SO, DO>
        where
            SO: LoadOrdering,
        {
            type Archived = $archived;
            type Resolver = ();

            fn resolve_with(
                field: &$atomic,
                _: Self::Resolver,
                out: Place<Self::Archived>,
            ) {
                out.write(<$archived>::new(field.load(SO::ORDERING) as _));
            }
        }

        #[cfg(not(feature = "unaligned"))]
        impl_serialize_with_as_atomic!($atomic);

        #[cfg(not(feature = "unaligned"))]
        impl<D, SO, DO> DeserializeWith<$archived, $atomic, D>
            for AsAtomic<SO, DO>
        where
            D: Fallible + ?Sized,
            DO: LoadOrdering,
        {
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
    all(
        target_has_atomic = "32",
        not(any(feature = "pointer_width_16", feature = "pointer_width_64")),
    ),
    all(target_has_atomic = "64", feature = "pointer_width_64"),
))]
impl_atomic_size_types! {
    core::sync::atomic::AtomicIsize,
    crate::primitive::ArchivedAtomicIsize,
    crate::primitive::ArchivedIsize;

    core::sync::atomic::AtomicUsize,
    crate::primitive::ArchivedAtomicUsize,
    crate::primitive::ArchivedUsize;
}
