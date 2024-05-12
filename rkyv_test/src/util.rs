#[cfg(feature = "wasm")]
wasm_bindgen_test::wasm_bindgen_test_configure!();

pub mod core {
    use core::{fmt::Debug, mem::MaybeUninit};

    use rkyv::{
        access_unchecked,
        de::pooling::Unpool,
        deserialize,
        rancor::{Error, Strategy},
        ser::{
            allocator::SubAllocator, sharing::Unshare, writer::Buffer,
            CoreSerializer, Serializer,
        },
        util::{serialize_into, Align},
        Deserialize, Serialize,
    };

    pub type DefaultSerializer<'a, E> = CoreSerializer<'a, E>;
    pub type DefaultDeserializer<E> = Strategy<Unpool, E>;

    pub fn test_archive_with<T, C>(value: &T, cmp: C)
    where
        T: Debug + PartialEq + for<'a> Serialize<DefaultSerializer<'a, Error>>,
        T::Archived: Debug + Deserialize<T, DefaultDeserializer<Error>>,
        C: Fn(&T, &T::Archived) -> bool,
    {
        let mut output = Align([MaybeUninit::<u8>::uninit(); 256]);
        let mut scratch = [MaybeUninit::<u8>::uninit(); 256];

        let buffer = serialize_into(
            value,
            Serializer::new(
                Buffer::from(&mut *output),
                SubAllocator::new(&mut scratch),
                Unshare,
            ),
        )
        .expect("failed to serialize value")
        .into_writer();

        let archived_value =
            unsafe { access_unchecked::<T::Archived>(&buffer) };
        assert!(cmp(value, archived_value));

        let de_value =
            deserialize::<T, _, Error>(archived_value, &mut Unpool).unwrap();
        assert_eq!(&de_value, value);
    }

    pub fn test_archive<T>(value: &T)
    where
        T: Debug + PartialEq + for<'a> Serialize<DefaultSerializer<'a, Error>>,
        T::Archived:
            Debug + PartialEq<T> + Deserialize<T, DefaultDeserializer<Error>>,
    {
        test_archive_with(value, |a, b| b == a);
    }
}

#[cfg(feature = "alloc")]
pub mod alloc {
    use core::fmt::Debug;

    use rkyv::{
        access_unchecked,
        de::pooling::Pool,
        deserialize,
        rancor::{Error, Strategy},
        to_bytes, Deserialize, Serialize,
    };

    pub type DefaultSerializer<'a, E> = rkyv::ser::DefaultSerializer<'a, E>;
    pub type DefaultDeserializer<E> = Strategy<Pool, E>;

    pub fn test_archive_with<T, C>(value: &T, cmp: C)
    where
        T: Debug + PartialEq + for<'a> Serialize<DefaultSerializer<'a, Error>>,
        T::Archived: Debug + Deserialize<T, DefaultDeserializer<Error>>,
        C: Fn(&T, &T::Archived) -> bool,
    {
        let bytes =
            to_bytes::<Error>(value).expect("failed to serialize value");

        let archived_value = unsafe { access_unchecked::<T::Archived>(&bytes) };
        assert!(cmp(value, archived_value));

        let de_value =
            deserialize::<T, _, Error>(archived_value, &mut Pool::new())
                .unwrap();
        assert_eq!(&de_value, value);
    }

    pub fn test_archive<T>(value: &T)
    where
        T: Debug + PartialEq + for<'a> Serialize<DefaultSerializer<'a, Error>>,
        T::Archived:
            Debug + PartialEq<T> + Deserialize<T, DefaultDeserializer<Error>>,
    {
        test_archive_with(value, |a, b| b == a);
    }
}
