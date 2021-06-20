#[cfg(feature = "wasm")]
wasm_bindgen_test::wasm_bindgen_test_configure!();

use core::fmt::Debug;
use rkyv::{
    archived_root, archived_unsized_root, ser::Serializer, Deserialize, Serialize, SerializeUnsized,
};

const BUFFER_SIZE: usize = 256;

#[cfg(feature = "alloc")]
mod types {
    use super::BUFFER_SIZE;
    use rkyv::{
        de::adapters::SharedDeserializerAdapter,
        ser::{adapters::SharedSerializerAdapter, serializers::BufferSerializer},
        Aligned, Infallible,
    };

    pub type DefaultSerializer =
        SharedSerializerAdapter<BufferSerializer<Aligned<[u8; BUFFER_SIZE]>>>;

    pub fn make_default_serializer() -> DefaultSerializer {
        SharedSerializerAdapter::new(BufferSerializer::new(Aligned([0u8; BUFFER_SIZE])))
    }

    pub fn unwrap_default_serializer(s: DefaultSerializer) -> Aligned<[u8; BUFFER_SIZE]> {
        s.into_inner().into_inner()
    }

    pub type DefaultDeserializer = SharedDeserializerAdapter<Infallible>;

    pub fn make_default_deserializer() -> DefaultDeserializer {
        SharedDeserializerAdapter::new(Infallible)
    }
}

#[cfg(not(feature = "alloc"))]
mod types {
    use super::BUFFER_SIZE;
    use rkyv::{ser::serializers::BufferSerializer, Aligned};

    pub type DefaultSerializer = BufferSerializer<Aligned<[u8; BUFFER_SIZE]>>;

    pub fn make_default_serializer() -> DefaultSerializer {
        BufferSerializer::new(Aligned([0u8; BUFFER_SIZE]))
    }

    pub fn unwrap_default_serializer(s: DefaultSerializer) -> Aligned<[u8; BUFFER_SIZE]> {
        s.into_inner()
    }

    pub struct DefaultDeserializer;

    impl rkyv::Fallible for DefaultDeserializer {
        type Error = ();
    }

    pub fn make_default_deserializer() -> DefaultDeserializer {
        DefaultDeserializer
    }
}

pub use types::*;

pub fn test_archive<T: Debug + Serialize<DefaultSerializer>>(value: &T)
where
    T: PartialEq,
    T::Archived: Debug + PartialEq<T> + Deserialize<T, DefaultDeserializer>,
{
    let mut serializer = make_default_serializer();
    serializer
        .serialize_value(value)
        .expect("failed to archive value");
    let len = serializer.pos();
    let buffer = unwrap_default_serializer(serializer);

    let archived_value = unsafe { archived_root::<T>(&buffer.as_ref()[0..len]) };
    assert_eq!(archived_value, value);
    let mut deserializer = make_default_deserializer();
    assert_eq!(&archived_value.deserialize(&mut deserializer).unwrap(), value);
}

pub fn test_archive_ref<T: Debug + SerializeUnsized<DefaultSerializer> + ?Sized>(value: &T)
where
    T::Archived: Debug + PartialEq<T>,
{
    let mut serializer = make_default_serializer();
    serializer
        .serialize_unsized_value(value)
        .expect("failed to archive ref");
    let len = serializer.pos();
    let buffer = unwrap_default_serializer(serializer);

    let archived_ref = unsafe { archived_unsized_root::<T>(&buffer.as_ref()[0..len]) };
    assert_eq!(archived_ref, value);
}

pub fn test_archive_container<
    T: Serialize<DefaultSerializer, Archived = U> + core::ops::Deref<Target = TV>,
    TV: Debug + ?Sized,
    U: core::ops::Deref<Target = TU>,
    TU: Debug + PartialEq<TV> + ?Sized,
>(
    value: &T,
) {
    let mut serializer = make_default_serializer();
    serializer
        .serialize_value(value)
        .expect("failed to archive ref");
    let len = serializer.pos();
    let buffer = unwrap_default_serializer(serializer);

    let archived_ref = unsafe { archived_root::<T>(&buffer.as_ref()[0..len]) };
    assert_eq!(archived_ref.deref(), value.deref());
}
