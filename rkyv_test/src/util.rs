#[cfg(feature = "wasm")]
wasm_bindgen_test::wasm_bindgen_test_configure!();

pub mod core {
    use core::fmt::Debug;

    use rkyv::{
        access_unchecked,
        de::pooling::Unify,
        deserialize,
        rancor::{Failure, Strategy},
        ser::Positional as _,
        util::{
            access_unsized_unchecked, serialize_into, serialize_rel_ptr_into,
        },
        Deserialize, Serialize, SerializeUnsized,
    };

    const BUFFER_SIZE: usize = 256;
    const SCRATCH_SIZE: usize = 256;

    pub type DefaultSerializer =
        rkyv::ser::CoreSerializer<BUFFER_SIZE, SCRATCH_SIZE>;
    pub type DefaultDeserializer = Unify;

    pub fn test_archive_with<T, C>(value: &T, cmp: C)
    where
        T: Debug + PartialEq + Serialize<Strategy<DefaultSerializer, Failure>>,
        T::Archived:
            Debug + Deserialize<T, Strategy<DefaultDeserializer, Failure>>,
        C: Fn(&T, &T::Archived) -> bool,
    {
        let serializer = serialize_into(value, DefaultSerializer::default())
            .expect("failed to serialize value");
        let len = serializer.pos();
        let buffer = serializer.writer.inner();

        let archived_value = unsafe { access_unchecked::<T>(&buffer[0..len]) };
        assert!(cmp(value, archived_value));

        let mut deserializer = DefaultDeserializer::default();
        let de_value =
            deserialize::<T, _, Failure>(archived_value, &mut deserializer)
                .unwrap();
        assert_eq!(&de_value, value);
    }

    pub fn test_archive<T>(value: &T)
    where
        T: Debug + PartialEq + Serialize<Strategy<DefaultSerializer, Failure>>,
        T::Archived: Debug
            + PartialEq<T>
            + Deserialize<T, Strategy<DefaultDeserializer, Failure>>,
    {
        test_archive_with(value, |a, b| b == a);
    }

    pub fn test_archive_ref<T>(value: &T)
    where
        T: Debug
            + SerializeUnsized<Strategy<DefaultSerializer, Failure>>
            + ?Sized,
        T::Archived: Debug + PartialEq<T>,
    {
        let serializer =
            serialize_rel_ptr_into(value, DefaultSerializer::default())
                .expect("failed to serialize relative pointer");
        let len = serializer.pos();
        let buffer = serializer.writer.inner();

        let archived_ref =
            unsafe { access_unsized_unchecked::<T>(&buffer[0..len]) };
        assert_eq!(archived_ref, value);
    }
}

#[cfg(feature = "alloc")]
pub mod alloc {
    use core::fmt::Debug;

    use rkyv::{
        access_unchecked, deserialize,
        rancor::{Failure, Strategy},
        ser::Positional as _,
        util::serialize_into,
        Deserialize, Serialize,
    };

    const SCRATCH_SIZE: usize = 256;

    pub type DefaultSerializer = rkyv::ser::AllocSerializer<SCRATCH_SIZE>;
    pub type DefaultDeserializer = rkyv::de::pooling::Unify;

    pub fn test_archive_with<T, C>(value: &T, cmp: C)
    where
        T: Debug + PartialEq + Serialize<Strategy<DefaultSerializer, Failure>>,
        T::Archived:
            Debug + Deserialize<T, Strategy<DefaultDeserializer, Failure>>,
        C: Fn(&T, &T::Archived) -> bool,
    {
        let serializer = serialize_into(value, DefaultSerializer::default())
            .expect("failed to serialize value");
        let len = serializer.pos();
        let buffer = &serializer.writer;

        let archived_value = unsafe { access_unchecked::<T>(&buffer[0..len]) };
        assert!(cmp(value, archived_value));

        let mut deserializer = DefaultDeserializer::default();
        let de_value =
            deserialize::<T, _, Failure>(archived_value, &mut deserializer)
                .unwrap();
        assert_eq!(&de_value, value);
    }

    pub fn test_archive<T>(value: &T)
    where
        T: Debug + PartialEq + Serialize<Strategy<DefaultSerializer, Failure>>,
        T::Archived: Debug
            + PartialEq<T>
            + Deserialize<T, Strategy<DefaultDeserializer, Failure>>,
    {
        test_archive_with(value, |a, b| b == a);
    }
}
