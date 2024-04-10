#[cfg(feature = "wasm")]
wasm_bindgen_test::wasm_bindgen_test_configure!();

pub mod core {
    use core::fmt::Debug;

    use rkyv::{
        access_unchecked, deserialize,
        rancor::{Error, Strategy},
        ser::Positional as _,
        util::serialize_into,
        Deserialize, Serialize,
    };

    const BUFFER_SIZE: usize = 256;
    const SCRATCH_SIZE: usize = 256;

    pub type DefaultSerializer =
        rkyv::ser::CoreSerializer<BUFFER_SIZE, SCRATCH_SIZE>;
    pub type DefaultDeserializer = rkyv::de::pooling::Duplicate;

    pub fn test_archive_with<T, C>(value: &T, cmp: C)
    where
        T: Debug + PartialEq + Serialize<Strategy<DefaultSerializer, Error>>,
        T::Archived:
            Debug + Deserialize<T, Strategy<DefaultDeserializer, Error>>,
        C: Fn(&T, &T::Archived) -> bool,
    {
        let serializer = serialize_into(value, DefaultSerializer::default())
            .expect("failed to serialize value");
        let len = serializer.pos();
        let buffer = serializer.writer.inner();

        let archived_value =
            unsafe { access_unchecked::<T::Archived>(&buffer[0..len]) };
        assert!(cmp(value, archived_value));

        let mut deserializer = DefaultDeserializer::default();
        let de_value =
            deserialize::<T, _, Error>(archived_value, &mut deserializer)
                .unwrap();
        assert_eq!(&de_value, value);
    }

    pub fn test_archive<T>(value: &T)
    where
        T: Debug + PartialEq + Serialize<Strategy<DefaultSerializer, Error>>,
        T::Archived: Debug
            + PartialEq<T>
            + Deserialize<T, Strategy<DefaultDeserializer, Error>>,
    {
        test_archive_with(value, |a, b| b == a);
    }
}

#[cfg(feature = "alloc")]
pub mod alloc {
    use core::fmt::Debug;

    use rkyv::{
        access_unchecked, deserialize,
        rancor::{Error, Strategy},
        ser::Positional as _,
        util::serialize_into,
        Deserialize, Serialize,
    };

    pub type DefaultSerializer = rkyv::ser::AllocSerializer;
    pub type DefaultDeserializer = rkyv::de::pooling::Unify;

    pub fn test_archive_with<T, C>(value: &T, cmp: C)
    where
        T: Debug + PartialEq + Serialize<Strategy<DefaultSerializer, Error>>,
        T::Archived:
            Debug + Deserialize<T, Strategy<DefaultDeserializer, Error>>,
        C: Fn(&T, &T::Archived) -> bool,
    {
        let serializer = serialize_into(value, DefaultSerializer::default())
            .expect("failed to serialize value");
        let len = serializer.pos();
        let buffer = &serializer.writer;

        let archived_value =
            unsafe { access_unchecked::<T::Archived>(&buffer[0..len]) };
        assert!(cmp(value, archived_value));

        let mut deserializer = DefaultDeserializer::default();
        let de_value =
            deserialize::<T, _, Error>(archived_value, &mut deserializer)
                .unwrap();
        assert_eq!(&de_value, value);
    }

    pub fn test_archive<T>(value: &T)
    where
        T: Debug + PartialEq + Serialize<Strategy<DefaultSerializer, Error>>,
        T::Archived: Debug
            + PartialEq<T>
            + Deserialize<T, Strategy<DefaultDeserializer, Error>>,
    {
        test_archive_with(value, |a, b| b == a);
    }
}
