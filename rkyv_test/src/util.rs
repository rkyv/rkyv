#[cfg(feature = "wasm")]
wasm_bindgen_test::wasm_bindgen_test_configure!();

macro_rules! impl_test_archive {
    ($ser:ty, $de:ty) => {
        use core::fmt::Debug;
        use rkyv::{
            archived_root, archived_unsized_root, ser::Serializer, Deserialize,
            Serialize, SerializeUnsized,
        };

        pub fn test_archive<T>(value: &T)
        where
            T: Debug + PartialEq + Serialize<$ser>,
            T::Archived: Debug + PartialEq<T> + Deserialize<T, $de>,
        {
            let mut serializer = <$ser>::default();
            serializer
                .serialize_value(value)
                .expect("failed to archive value");
            let len = serializer.pos();
            let buffer = serializer.into_serializer().into_inner();

            let archived_value = unsafe { archived_root::<T>(&buffer[0..len]) };
            assert_eq!(archived_value, value);
            let mut deserializer = <$de>::default();
            assert_eq!(
                &archived_value.deserialize(&mut deserializer).unwrap(),
                value
            );
        }

        pub fn test_archive_ref<T: Debug + SerializeUnsized<$ser> + ?Sized>(
            value: &T,
        ) where
            T::Archived: Debug + PartialEq<T>,
        {
            let mut serializer = <$ser>::default();
            serializer
                .serialize_unsized_value(value)
                .expect("failed to archive ref");
            let len = serializer.pos();
            let buffer = serializer.into_serializer().into_inner();

            let archived_ref =
                unsafe { archived_unsized_root::<T>(&buffer[0..len]) };
            assert_eq!(archived_ref, value);
        }

        pub fn test_archive_container<
            T: Serialize<$ser, Archived = U> + core::ops::Deref<Target = TV>,
            TV: Debug + ?Sized,
            U: core::ops::Deref<Target = TU>,
            TU: Debug + PartialEq<TV> + ?Sized,
        >(
            value: &T,
        ) {
            let mut serializer = <$ser>::default();
            serializer
                .serialize_value(value)
                .expect("failed to archive ref");
            let len = serializer.pos();
            let buffer = serializer.into_serializer().into_inner();

            let archived_ref = unsafe { archived_root::<T>(&buffer[0..len]) };
            assert_eq!(archived_ref.deref(), value.deref());
        }
    };
}

pub mod core {
    const BUFFER_SIZE: usize = 256;
    const SCRATCH_SIZE: usize = 256;

    pub type DefaultSerializer =
        rkyv::ser::serializers::CoreSerializer<BUFFER_SIZE, SCRATCH_SIZE>;
    pub type DefaultDeserializer = rkyv::Infallible;

    impl_test_archive!(DefaultSerializer, DefaultDeserializer);
}

#[cfg(feature = "alloc")]
pub mod alloc {
    const SCRATCH_SIZE: usize = 256;

    pub type DefaultSerializer =
        rkyv::ser::serializers::AllocSerializer<SCRATCH_SIZE>;
    pub type DefaultDeserializer =
        rkyv::de::deserializers::SharedDeserializeMap;

    impl_test_archive!(DefaultSerializer, DefaultDeserializer);
}
