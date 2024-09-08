use crate::{
    impls::core::with::atomic::LoadOrdering,
    rancor::Fallible,
    with::{ArchiveWith, AtomicLoad, DeserializeWith},
    Place,
};

macro_rules! impl_multi_byte_atomic {
    ($atomic:ty, $archived:ty) => {
        impl<SO: LoadOrdering> ArchiveWith<$atomic> for AtomicLoad<SO> {
            type Archived = $archived;
            type Resolver = ();

            fn resolve_with(
                field: &$atomic,
                _: Self::Resolver,
                out: Place<Self::Archived>,
            ) {
                out.write(<$archived>::from_native(field.load(SO::ORDERING)));
            }
        }

        impl_serialize_with_atomic_load!($atomic);

        impl<D, SO> DeserializeWith<$archived, $atomic, D> for AtomicLoad<SO>
        where
            D: Fallible + ?Sized,
        {
            fn deserialize_with(
                field: &$archived,
                _: &mut D,
            ) -> Result<$atomic, D::Error> {
                Ok(<$atomic>::new(field.to_native()))
            }
        }
    };
}

macro_rules! impl_multi_byte_atomics {
    ($($atomic:ty, $archived: ty);* $(;)?) => {
        $(
            impl_multi_byte_atomic!($atomic, $archived);
        )*
    }
}

#[cfg(target_has_atomic = "16")]
impl_multi_byte_atomics! {
    core::sync::atomic::AtomicI16, crate::primitive::ArchivedI16;
    core::sync::atomic::AtomicU16, crate::primitive::ArchivedU16;
    rend::AtomicI16_le, rend::i16_le;
    rend::AtomicI16_be, rend::i16_be;
    rend::AtomicU16_le, rend::u16_le;
    rend::AtomicU16_be, rend::u16_be;
}
#[cfg(target_has_atomic = "32")]
impl_multi_byte_atomics! {
    core::sync::atomic::AtomicI32, crate::primitive::ArchivedI32;
    core::sync::atomic::AtomicU32, crate::primitive::ArchivedU32;
    rend::AtomicI32_le, crate::primitive::ArchivedI32;
    rend::AtomicI32_be, crate::primitive::ArchivedI32;
    rend::AtomicU32_le, crate::primitive::ArchivedU32;
    rend::AtomicU32_be, crate::primitive::ArchivedU32;
}
#[cfg(target_has_atomic = "64")]
impl_multi_byte_atomics! {
    core::sync::atomic::AtomicI64, crate::primitive::ArchivedI64;
    core::sync::atomic::AtomicU64, crate::primitive::ArchivedU64;
    rend::AtomicI64_le, crate::primitive::ArchivedI64;
    rend::AtomicI64_be, crate::primitive::ArchivedI64;
    rend::AtomicU64_le, crate::primitive::ArchivedU64;
    rend::AtomicU64_be, crate::primitive::ArchivedU64;
}

// AtomicUsize

macro_rules! impl_atomic_size_type {
    ($atomic:ty, $archived:ty) => {
        impl<SO: LoadOrdering> ArchiveWith<$atomic> for AtomicLoad<SO> {
            type Archived = $archived;
            type Resolver = ();

            fn resolve_with(
                field: &$atomic,
                _: Self::Resolver,
                out: Place<Self::Archived>,
            ) {
                out.write(<$archived>::from_native(
                    field.load(SO::ORDERING) as _
                ));
            }
        }

        impl_serialize_with_atomic_load!($atomic);

        impl<D, SO> DeserializeWith<$archived, $atomic, D> for AtomicLoad<SO>
        where
            D: Fallible + ?Sized,
        {
            fn deserialize_with(
                field: &$archived,
                _: &mut D,
            ) -> Result<$atomic, D::Error> {
                Ok(<$atomic>::new(field.to_native() as _))
            }
        }
    };
}

macro_rules! impl_atomic_size_types {
    ($($atomic:ty, $archived:ty);* $(;)?) => {
        $(
            impl_atomic_size_type!($atomic, $archived);
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
    core::sync::atomic::AtomicIsize, crate::primitive::ArchivedIsize;
    core::sync::atomic::AtomicUsize, crate::primitive::ArchivedUsize;
}

#[cfg(test)]
mod tests {
    #[cfg(target_has_atomic = "32")]
    #[test]
    fn with_atomic_load() {
        use core::sync::atomic::{AtomicU32, Ordering};

        use crate::{
            api::test::roundtrip,
            with::{AtomicLoad, Relaxed},
            Archive, Deserialize, Serialize,
        };

        #[derive(Archive, Debug, Deserialize, Serialize)]
        #[rkyv(crate, derive(Debug))]
        struct Test {
            #[rkyv(with = AtomicLoad<Relaxed>)]
            a: AtomicU32,
        }

        impl PartialEq for Test {
            fn eq(&self, other: &Self) -> bool {
                self.a.load(Ordering::Relaxed)
                    == other.a.load(Ordering::Relaxed)
            }
        }

        impl PartialEq<Test> for ArchivedTest {
            fn eq(&self, other: &Test) -> bool {
                self.a == other.a.load(Ordering::Relaxed)
            }
        }

        let value = Test {
            a: AtomicU32::new(42),
        };
        roundtrip(&value);
    }
}
