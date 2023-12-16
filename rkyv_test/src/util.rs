#[cfg(feature = "wasm")]
wasm_bindgen_test::wasm_bindgen_test_configure!();

macro_rules! impl_test_archive {
    ($ser:ty, $de:ty) => {
        use core::fmt::Debug;
        use rkyv::{
            access_unchecked, deserialize,
            rancor::{Failure, Strategy},
            ser::Serializer,
            util::{
                access_unsized_unchecked, serialize_into,
                serialize_rel_ptr_into,
            },
            Deserialize, Serialize, SerializeUnsized,
        };

        pub fn test_archive<T>(value: &T)
        where
            T: Debug + PartialEq + Serialize<Strategy<$ser, Failure>>,
            T::Archived:
                Debug + PartialEq<T> + Deserialize<T, Strategy<$de, Failure>>,
        {
            let serializer = serialize_into(value, <$ser>::default())
                .expect("failed to serialize value");
            let len = Serializer::<Failure>::pos(&serializer);
            let buffer = serializer.into_serializer().into_inner();

            let archived_value =
                unsafe { access_unchecked::<T>(&buffer[0..len]) };
            assert_eq!(archived_value, value);
            let mut deserializer = <$de>::default();
            let de_value =
                deserialize::<T, _, Failure>(archived_value, &mut deserializer)
                    .unwrap();
            assert_eq!(&de_value, value);
        }

        pub fn test_archive_ref<
            T: Debug + SerializeUnsized<Strategy<$ser, Failure>> + ?Sized,
        >(
            value: &T,
        ) where
            T::Archived: Debug + PartialEq<T>,
        {
            let serializer = serialize_rel_ptr_into(value, <$ser>::default())
                .expect("failed to serialize relative pointer");
            let len = Serializer::<Failure>::pos(&serializer);
            let buffer = serializer.into_serializer().into_inner();

            let archived_ref =
                unsafe { access_unsized_unchecked::<T>(&buffer[0..len]) };
            assert_eq!(archived_ref, value);
        }

        pub fn test_archive_container<
            T: Serialize<Strategy<$ser, Failure>, Archived = U>
                + core::ops::Deref<Target = TV>,
            TV: Debug + ?Sized,
            U: core::ops::Deref<Target = TU>,
            TU: Debug + PartialEq<TV> + ?Sized,
        >(
            value: &T,
        ) {
            let serializer = serialize_into(value, <$ser>::default())
                .expect("failed to serialize value");
            let len = Serializer::<Failure>::pos(&serializer);
            let buffer = serializer.into_serializer().into_inner();

            let archived_ref =
                unsafe { access_unchecked::<T>(&buffer[0..len]) };
            assert_eq!(archived_ref.deref(), value.deref());
        }
    };
}

pub mod core {
    const BUFFER_SIZE: usize = 256;
    const SCRATCH_SIZE: usize = 256;

    pub type DefaultSerializer =
        rkyv::ser::serializers::CoreSerializer<BUFFER_SIZE, SCRATCH_SIZE>;
    pub type DefaultDeserializer = ();

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
