#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(feature = "arbitrary_enum_discriminant", feature(arbitrary_enum_discriminant))]

#[cfg(all(test, feature = "validation"))]
mod validation;

#[cfg(test)]
mod util {
    #[cfg(feature = "wasm")]
    wasm_bindgen_test::wasm_bindgen_test_configure!();

    use rkyv::{
        archived_root, archived_unsized_root,
        ser::{serializers::BufferSerializer, Serializer},
        Aligned, Deserialize, Serialize, SerializeUnsized,
    };
    #[cfg(feature = "std")]
    use rkyv::{
        de::{adapters::SharedDeserializerAdapter, deserializers::AllocDeserializer},
        ser::adapters::SharedSerializerAdapter,
    };

    pub const BUFFER_SIZE: usize = 256;

    #[cfg(feature = "std")]
    pub type DefaultSerializer =
        SharedSerializerAdapter<BufferSerializer<Aligned<[u8; BUFFER_SIZE]>>>;

    #[cfg(feature = "std")]
    pub fn make_default_serializer() -> DefaultSerializer {
        SharedSerializerAdapter::new(BufferSerializer::new(Aligned([0u8; BUFFER_SIZE])))
    }

    #[cfg(feature = "std")]
    pub fn unwrap_default_serializer(s: DefaultSerializer) -> Aligned<[u8; BUFFER_SIZE]> {
        s.into_inner().into_inner()
    }

    #[cfg(feature = "std")]
    pub type DefaultDeserializer = SharedDeserializerAdapter<AllocDeserializer>;

    #[cfg(feature = "std")]
    pub fn make_default_deserializer() -> DefaultDeserializer {
        SharedDeserializerAdapter::new(AllocDeserializer)
    }

    #[cfg(not(feature = "std"))]
    pub type DefaultSerializer = BufferSerializer<Aligned<[u8; BUFFER_SIZE]>>;

    #[cfg(not(feature = "std"))]
    pub fn make_default_serializer() -> DefaultSerializer {
        BufferSerializer::new(Aligned([0u8; BUFFER_SIZE]))
    }

    #[cfg(not(feature = "std"))]
    pub fn unwrap_default_serializer(s: DefaultSerializer) -> Aligned<[u8; BUFFER_SIZE]> {
        s.into_inner()
    }

    #[cfg(not(feature = "std"))]
    pub struct DefaultDeserializer;

    #[cfg(not(feature = "std"))]
    impl rkyv::Fallible for DefaultDeserializer {
        type Error = ();
    }

    #[cfg(not(feature = "std"))]
    pub fn make_default_deserializer() -> DefaultDeserializer {
        DefaultDeserializer
    }

    pub fn test_archive<T: Serialize<DefaultSerializer>>(value: &T)
    where
        T: PartialEq,
        T::Archived: PartialEq<T> + Deserialize<T, DefaultDeserializer>,
    {
        let mut serializer = make_default_serializer();
        serializer
            .serialize_value(value)
            .expect("failed to archive value");
        let len = serializer.pos();
        let buffer = unwrap_default_serializer(serializer);

        let archived_value = unsafe { archived_root::<T>(&buffer.as_ref()[0..len]) };
        assert!(archived_value == value);
        let mut deserializer = make_default_deserializer();
        assert!(&archived_value.deserialize(&mut deserializer).unwrap() == value);
    }

    pub fn test_archive_ref<T: SerializeUnsized<DefaultSerializer> + ?Sized>(value: &T)
    where
        T::Archived: PartialEq<T>,
    {
        let mut serializer = make_default_serializer();
        serializer
            .serialize_unsized_value(value)
            .expect("failed to archive ref");
        let len = serializer.pos();
        let buffer = unwrap_default_serializer(serializer);

        let archived_ref = unsafe { archived_unsized_root::<T>(&buffer.as_ref()[0..len]) };
        assert!(archived_ref == value);
    }

    #[cfg(feature = "std")]
    pub fn test_archive_container<
        T: Serialize<DefaultSerializer, Archived = U> + core::ops::Deref<Target = TV>,
        TV: ?Sized,
        U: core::ops::Deref<Target = TU>,
        TU: PartialEq<TV> + ?Sized,
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
        assert!(archived_ref.deref() == value.deref());
    }
}

#[cfg(test)]
mod no_std_tests {
    use crate::util::*;

    #[cfg(feature = "wasm")]
    use wasm_bindgen_test::*;

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn archive_primitives() {
        test_archive(&());
        test_archive(&true);
        test_archive(&false);
        test_archive(&1234567f32);
        test_archive(&12345678901234f64);
        test_archive(&123i8);
        test_archive(&123456i32);
        test_archive(&1234567890i128);
        test_archive(&123u8);
        test_archive(&123456u32);
        test_archive(&1234567890u128);
        #[cfg(not(any(feature = "strict", feature = "archive_le", feature = "archive_be")))]
        test_archive(&(24, true, 16f32));
        test_archive(&[1, 2, 3, 4, 5, 6]);

        test_archive(&Option::<()>::None);
        test_archive(&Some(42));
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn archive_refs() {
        #[cfg(not(feature = "strict"))]
        test_archive_ref::<[i32; 4]>(&[1, 2, 3, 4]);
        test_archive_ref::<str>("hello world");
        test_archive_ref::<[i32]>([1, 2, 3, 4].as_ref());
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn archive_slices() {
        test_archive_ref::<str>("hello world");
        test_archive_ref::<[i32]>([1, 2, 3, 4].as_ref());
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn archive_empty_slice() {
        test_archive_ref::<[i32; 0]>(&[]);
        test_archive_ref::<[i32]>([].as_ref());
        test_archive_ref::<str>("");
    }
}

#[cfg(feature = "std")]
#[cfg(test)]
mod tests {
    use crate::util::*;
    use core::pin::Pin;
    use rkyv::{
        archived_root, archived_root_mut,
        de::{adapters::SharedDeserializerAdapter, deserializers::AllocDeserializer, Deserializer},
        ser::{
            adapters::SharedSerializerAdapter,
            serializers::{AlignedSerializer, BufferSerializer},
            SeekSerializer, Serializer,
        },
        AlignedVec, Archive, Archived, Deserialize, Serialize,
    };

    #[cfg(feature = "wasm")]
    use wasm_bindgen_test::*;

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn archive_containers() {
        test_archive_container(&Box::new(42));
        test_archive_container(&"".to_string().into_boxed_str());
        test_archive_container(&"hello world".to_string().into_boxed_str());
        test_archive_container(&Vec::<i32>::new().into_boxed_slice());
        test_archive_container(&vec![1, 2, 3, 4].into_boxed_slice());
        test_archive_container(&"".to_string());
        test_archive_container(&"hello world".to_string());
        test_archive_container(&Vec::<i32>::new());
        test_archive_container(&vec![1, 2, 3, 4]);
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn archive_composition() {
        test_archive(&Some(Box::new(42)));
        test_archive(&Some("hello world".to_string().into_boxed_str()));
        test_archive(&Some(vec![1, 2, 3, 4].into_boxed_slice()));
        test_archive(&Some("hello world".to_string()));
        test_archive(&Some(vec![1, 2, 3, 4]));
        test_archive(&Some(Box::new(vec![1, 2, 3, 4])));
    }

    mod example {
        #[cfg(feature = "wasm")]
        use wasm_bindgen_test::*;

        #[test]
        #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
        fn archive_example() {
            use rkyv::{
                archived_root,
                de::deserializers::AllocDeserializer,
                ser::{serializers::AlignedSerializer, Serializer},
                AlignedVec, Archive, Deserialize, Serialize,
            };

            #[derive(Archive, Deserialize, Serialize, Debug, PartialEq)]
            struct Test {
                int: u8,
                string: String,
                option: Option<Vec<i32>>,
            }

            let value = Test {
                int: 42,
                string: "hello world".to_string(),
                option: Some(vec![1, 2, 3, 4]),
            };

            let mut serializer = AlignedSerializer::new(AlignedVec::new());
            serializer
                .serialize_value(&value)
                .expect("failed to serialize value");
            let bytes = serializer.into_inner();

            let archived = unsafe { archived_root::<Test>(&bytes[..]) };
            assert_eq!(archived.int, value.int);
            assert_eq!(archived.string, value.string);
            assert_eq!(archived.option, value.option);

            let deserialized = archived
                .deserialize(&mut AllocDeserializer)
                .expect("failed to deserialize value");
            assert_eq!(deserialized, value);
        }
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn archive_hash_map() {
        use std::collections::HashMap;

        #[cfg(not(any(feature = "archive_le", feature = "archive_be")))]
        {
            test_archive(&HashMap::<i32, i32>::new());

            let mut hash_map = HashMap::new();
            hash_map.insert(1, 2);
            hash_map.insert(3, 4);
            hash_map.insert(5, 6);
            hash_map.insert(7, 8);

            test_archive(&hash_map);
        }

        let mut hash_map = HashMap::new();
        hash_map.insert("hello".to_string(), "world".to_string());
        hash_map.insert("foo".to_string(), "bar".to_string());
        hash_map.insert("baz".to_string(), "bat".to_string());

        let mut serializer = AlignedSerializer::new(AlignedVec::new());
        serializer
            .serialize_value(&hash_map)
            .expect("failed to archive value");
        let buf = serializer.into_inner();
        let archived_value = unsafe { archived_root::<HashMap<String, String>>(buf.as_ref()) };

        assert!(archived_value.len() == hash_map.len());

        for (key, value) in hash_map.iter() {
            assert!(archived_value.contains_key(key.as_str()));
            assert!(archived_value[key.as_str()].eq(value));
        }

        for (key, value) in archived_value.iter() {
            assert!(hash_map.contains_key(key.as_str()));
            assert!(hash_map[key.as_str()].eq(value));
        }
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn archive_unit_struct() {
        #[derive(Archive, Serialize, Deserialize, PartialEq)]
        #[archive(compare(PartialEq))]
        struct Test;

        test_archive(&Test);
        test_archive(&vec![Test, Test]);
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn archive_tuple_struct() {
        #[derive(Archive, Serialize, Deserialize, PartialEq)]
        #[archive(compare(PartialEq))]
        struct Test((), i32, String, Option<i32>);

        test_archive(&Test((), 42, "hello world".to_string(), Some(42)));
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn archive_simple_struct() {
        #[derive(Archive, Serialize, Deserialize, PartialEq)]
        #[archive(compare(PartialEq))]
        struct Test {
            a: (),
            b: i32,
            c: String,
            d: Option<i32>,
        }

        test_archive(&Test {
            a: (),
            b: 42,
            c: "hello world".to_string(),
            d: Some(42),
        });
        test_archive(&vec![
            Test {
                a: (),
                b: 42,
                c: "hello world".to_string(),
                d: Some(42),
            },
            Test {
                a: (),
                b: 42,
                c: "hello world".to_string(),
                d: Some(42),
            },
        ]);
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn archive_generic_struct() {
        pub trait TestTrait {
            type Associated: PartialEq;
        }

        impl TestTrait for () {
            type Associated = i32;
        }

        #[derive(Archive, Serialize, Deserialize, PartialEq)]
        #[archive(compare(PartialEq))]
        struct Test<T: TestTrait> {
            a: (),
            b: <T as TestTrait>::Associated,
            c: String,
            d: Option<i32>,
        }

        test_archive(&Test::<()> {
            a: (),
            b: 42,
            c: "hello world".to_string(),
            d: Some(42),
        });
        test_archive(&vec![
            Test::<()> {
                a: (),
                b: 42,
                c: "hello world".to_string(),
                d: Some(42),
            },
            Test::<()> {
                a: (),
                b: 42,
                c: "hello world".to_string(),
                d: Some(42),
            },
        ]);
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn archive_enum() {
        #[derive(Archive, Serialize, Deserialize, PartialEq)]
        #[archive(compare(PartialEq))]
        enum Test {
            A,
            B(String),
            C { a: i32, b: String },
        }

        test_archive(&Test::A);
        test_archive(&Test::B("hello_world".to_string()));
        test_archive(&Test::C {
            a: 42,
            b: "hello world".to_string(),
        });
        test_archive(&vec![
            Test::A,
            Test::B("hello world".to_string()),
            Test::C {
                a: 42,
                b: "hello world".to_string(),
            },
        ]);
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn archive_generic_enum() {
        pub trait TestTrait {
            type Associated: PartialEq;
        }

        impl TestTrait for () {
            type Associated = i32;
        }

        #[derive(Archive, Serialize, Deserialize, PartialEq)]
        #[archive(compare(PartialEq))]
        enum Test<T: TestTrait> {
            A,
            B(String),
            C {
                a: <T as TestTrait>::Associated,
                b: String,
            },
        }

        test_archive(&Test::<()>::A);
        test_archive(&Test::<()>::B("hello_world".to_string()));
        test_archive(&Test::<()>::C {
            a: 42,
            b: "hello world".to_string(),
        });
        test_archive(&vec![
            Test::<()>::A,
            Test::<()>::B("hello world".to_string()),
            Test::<()>::C {
                a: 42,
                b: "hello world".to_string(),
            },
        ]);
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    #[cfg(not(any(feature = "archive_le", feature = "archive_be")))]
    fn archive_copy() {
        #[derive(Archive, Serialize, Deserialize, Clone, Copy, PartialEq)]
        #[archive(copy)]
        struct TestUnit;

        test_archive(&TestUnit);

        #[derive(Archive, Serialize, Deserialize, Clone, Copy, PartialEq)]
        #[cfg_attr(not(any(feature = "archive_be", feature = "archive_be")), archive(copy))]
        struct TestStruct {
            a: (),
            b: i32,
            c: bool,
            d: f32,
            e: TestUnit,
        }

        test_archive(&TestStruct {
            a: (),
            b: 42,
            c: true,
            d: 3.14f32,
            e: TestUnit,
        });

        #[derive(Archive, Serialize, Deserialize, Clone, Copy, PartialEq)]
        #[cfg_attr(not(any(feature = "archive_be", feature = "archive_le")), archive(copy))]
        struct TestTuple((), i32, bool, f32, TestUnit);

        test_archive(&TestTuple((), 42, true, 3.14f32, TestUnit));

        #[derive(Archive, Serialize, Deserialize, Clone, Copy, PartialEq)]
        #[repr(u8)]
        #[cfg_attr(not(any(feature = "archive_be", feature = "archive_le")), archive(copy))]
        enum TestEnum {
            A((), i32, bool, f32, TestUnit),
        }

        test_archive(&TestEnum::A((), 42, true, 3.14f32, TestUnit));

        #[derive(Archive, Serialize, Deserialize, Clone, Copy, PartialEq)]
        #[cfg_attr(not(any(feature = "archive_be", feature = "archive_le")), archive(copy))]
        struct TestGeneric<T>(T);

        test_archive(&TestGeneric(42));
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn archive_derives() {
        #[derive(Archive, Serialize, Clone)]
        #[archive_attr(derive(Clone, Debug, PartialEq))]
        struct Test(i32);

        let value = Test(42);

        let mut buf = AlignedVec::new();
        let mut serializer = AlignedSerializer::new(&mut buf);
        serializer
            .serialize_value(&value)
            .expect("failed to archive value");
        let archived_value = unsafe { archived_root::<Test>(buf.as_ref()) };

        assert_eq!(archived_value, &archived_value.clone());
    }

    #[test]
    #[cfg(not(feature = "wasm"))]
    fn manual_archive_dyn() {
        use rkyv::{
            ArchivePointee, ArchiveUnsized, ArchivedMetadata, DeserializeUnsized, SerializeUnsized,
        };
        use rkyv_dyn::{
            register_impl, ArchivedDynMetadata, DeserializeDyn, DynDeserializer, DynError,
            RegisteredImpl, SerializeDyn,
        };
        use rkyv_typename::TypeName;

        pub trait TestTrait {
            fn get_id(&self) -> i32;
        }

        #[ptr_meta::pointee]
        pub trait SerializeTestTrait: TestTrait + SerializeDyn {}

        impl<T: Archive + SerializeDyn + TestTrait> SerializeTestTrait for T where
            T::Archived: RegisteredImpl<dyn DeserializeTestTrait>
        {
        }

        #[ptr_meta::pointee]
        pub trait DeserializeTestTrait: TestTrait + DeserializeDyn<dyn SerializeTestTrait> {}

        impl<T: TestTrait + DeserializeDyn<dyn SerializeTestTrait>> DeserializeTestTrait for T {}

        impl TypeName for dyn DeserializeTestTrait {
            fn build_type_name<F: FnMut(&str)>(mut f: F) {
                f("dyn DeserializeTestTrait");
            }
        }

        impl ArchiveUnsized for dyn SerializeTestTrait {
            type Archived = dyn DeserializeTestTrait;
            type MetadataResolver = ();

            fn resolve_metadata(
                &self,
                _: usize,
                _: Self::MetadataResolver,
                out: &mut core::mem::MaybeUninit<ArchivedMetadata<Self>>,
            ) {
                ArchivedDynMetadata::emplace(self.archived_type_id(), out);
            }
        }

        impl ArchivePointee for dyn DeserializeTestTrait {
            type ArchivedMetadata = ArchivedDynMetadata<Self>;

            fn pointer_metadata(
                archived: &Self::ArchivedMetadata,
            ) -> <Self as ptr_meta::Pointee>::Metadata {
                archived.pointer_metadata()
            }
        }

        impl<S: Serializer + ?Sized> SerializeUnsized<S> for dyn SerializeTestTrait {
            fn serialize_unsized(&self, mut serializer: &mut S) -> Result<usize, S::Error> {
                self.serialize_dyn(&mut serializer)
                    .map_err(|e| *e.downcast::<S::Error>().unwrap())
            }

            fn serialize_metadata(&self, _: &mut S) -> Result<Self::MetadataResolver, S::Error> {
                Ok(())
            }
        }

        impl<D: Deserializer + ?Sized> DeserializeUnsized<dyn SerializeTestTrait, D>
            for dyn DeserializeTestTrait
        {
            unsafe fn deserialize_unsized(
                &self,
                mut deserializer: &mut D,
            ) -> Result<*mut (), D::Error> {
                self.deserialize_dyn(&mut deserializer)
                    .map_err(|e| *e.downcast().unwrap())
            }

            fn deserialize_metadata(
                &self,
                mut deserializer: &mut D,
            ) -> Result<<dyn SerializeTestTrait as ptr_meta::Pointee>::Metadata, D::Error>
            {
                self.deserialize_dyn_metadata(&mut deserializer)
                    .map_err(|e| *e.downcast().unwrap())
            }
        }

        #[derive(Archive, Serialize, Deserialize)]
        #[archive_attr(derive(TypeName))]
        pub struct Test {
            id: i32,
        }

        impl TestTrait for Test {
            fn get_id(&self) -> i32 {
                self.id
            }
        }

        register_impl!(Archived<Test> as dyn DeserializeTestTrait);

        impl DeserializeDyn<dyn SerializeTestTrait> for Archived<Test>
        where
            Archived<Test>: Deserialize<Test, dyn DynDeserializer>,
        {
            unsafe fn deserialize_dyn(
                &self,
                deserializer: &mut dyn DynDeserializer,
            ) -> Result<*mut (), DynError> {
                let result =
                    deserializer.alloc_dyn(core::alloc::Layout::new::<Test>())? as *mut Test;
                result.write(self.deserialize(deserializer)?);
                Ok(result as *mut ())
            }

            fn deserialize_dyn_metadata(
                &self,
                _: &mut dyn DynDeserializer,
            ) -> Result<<dyn SerializeTestTrait as ptr_meta::Pointee>::Metadata, DynError>
            {
                unsafe {
                    Ok(core::mem::transmute(ptr_meta::metadata(
                        core::ptr::null::<Test>() as *const dyn SerializeTestTrait,
                    )))
                }
            }
        }

        impl TestTrait for Archived<Test> {
            fn get_id(&self) -> i32 {
                self.id.into()
            }
        }

        let value: Box<dyn SerializeTestTrait> = Box::new(Test { id: 42 });

        let mut serializer = AlignedSerializer::new(AlignedVec::new());
        serializer
            .serialize_value(&value)
            .expect("failed to archive value");
        let buf = serializer.into_inner();
        let archived_value = unsafe { archived_root::<Box<dyn SerializeTestTrait>>(buf.as_ref()) };
        assert_eq!(value.get_id(), archived_value.get_id());

        // exercise vtable cache
        assert_eq!(value.get_id(), archived_value.get_id());
        assert_eq!(value.get_id(), archived_value.get_id());

        let deserialized_value: Box<dyn SerializeTestTrait> =
            archived_value.deserialize(&mut AllocDeserializer).unwrap();
        assert_eq!(value.get_id(), deserialized_value.get_id());
    }

    #[test]
    #[cfg(not(feature = "wasm"))]
    fn archive_dyn() {
        use rkyv::AlignedVec;
        use rkyv_dyn::archive_dyn;
        use rkyv_typename::TypeName;

        #[archive_dyn(serialize = "STestTrait", deserialize = "DTestTrait")]
        pub trait TestTrait {
            fn get_id(&self) -> i32;
        }

        #[derive(Archive, Serialize, Deserialize)]
        #[archive_attr(derive(TypeName))]
        pub struct Test {
            id: i32,
        }

        #[archive_dyn(serialize = "STestTrait", deserialize = "DTestTrait")]
        impl TestTrait for Test {
            fn get_id(&self) -> i32 {
                self.id
            }
        }

        impl TestTrait for Archived<Test> {
            fn get_id(&self) -> i32 {
                self.id.into()
            }
        }

        let value: Box<dyn STestTrait> = Box::new(Test { id: 42 });

        let mut serializer = AlignedSerializer::new(AlignedVec::new());
        serializer
            .serialize_value(&value)
            .expect("failed to archive value");
        let buf = serializer.into_inner();
        let archived_value = unsafe { archived_root::<Box<dyn STestTrait>>(buf.as_ref()) };
        assert_eq!(value.get_id(), archived_value.get_id());

        // exercise vtable cache
        assert_eq!(value.get_id(), archived_value.get_id());
        assert_eq!(value.get_id(), archived_value.get_id());

        // deserialize
        let deserialized_value: Box<dyn STestTrait> =
            archived_value.deserialize(&mut AllocDeserializer).unwrap();
        assert_eq!(value.get_id(), deserialized_value.get_id());
        assert_eq!(value.get_id(), deserialized_value.get_id());
    }

    #[test]
    #[cfg(not(feature = "wasm"))]
    fn archive_dyn_generic() {
        use rkyv::archived_value;
        use rkyv_dyn::archive_dyn;
        use rkyv_typename::TypeName;

        use rkyv_dyn::{register_impl, DynDeserializer, DynError, DynSerializer};

        #[archive_dyn(serialize = "STestTrait", deserialize = "DTestTrait")]
        pub trait TestTrait<T: TypeName> {
            fn get_value(&self) -> T;
        }

        #[derive(Archive, Serialize, Deserialize)]
        #[archive_attr(derive(TypeName))]
        pub struct Test<T> {
            value: T,
        }

        #[archive_dyn(serialize = "STestTrait", deserialize = "DTestTrait")]
        impl TestTrait<i32> for Test<i32> {
            fn get_value(&self) -> i32 {
                self.value
            }
        }

        impl TestTrait<i32> for ArchivedTest<i32> {
            fn get_value(&self) -> i32 {
                self.value.into()
            }
        }

        impl<T: core::fmt::Display> TestTrait<String> for Test<T> {
            fn get_value(&self) -> String {
                format!("{}", self.value)
            }
        }

        impl<
                T: Archive
                    + for<'a> Serialize<dyn DynSerializer + 'a>
                    + core::fmt::Display
                    + TypeName
                    + 'static,
            > rkyv_dyn::DeserializeDyn<dyn STestTrait<String>> for ArchivedTest<T>
        where
            ArchivedTest<T>: for<'a> Deserialize<Test<T>, (dyn DynDeserializer + 'a)>
                + rkyv_dyn::RegisteredImpl<dyn DTestTrait<String>>,
        {
            unsafe fn deserialize_dyn(
                &self,
                deserializer: &mut dyn DynDeserializer,
            ) -> Result<*mut (), DynError> {
                let result =
                    deserializer.alloc(core::alloc::Layout::new::<Test<T>>())? as *mut Test<T>;
                result.write(self.deserialize(deserializer)?);
                Ok(result as *mut ())
            }

            fn deserialize_dyn_metadata(
                &self,
                _: &mut dyn DynDeserializer,
            ) -> Result<<dyn STestTrait<String> as ptr_meta::Pointee>::Metadata, DynError>
            {
                unsafe {
                    Ok(core::mem::transmute(ptr_meta::metadata(
                        core::ptr::null::<Test<T>>() as *const dyn STestTrait<String>,
                    )))
                }
            }
        }

        impl<T: Archive> TestTrait<String> for ArchivedTest<T>
        where
            T::Archived: core::fmt::Display,
        {
            fn get_value(&self) -> String {
                format!("{}", self.value)
            }
        }

        register_impl!(Archived<Test<String>> as dyn DTestTrait<String>);

        let i32_value: Box<dyn STestTrait<i32>> = Box::new(Test { value: 42 });
        let string_value: Box<dyn STestTrait<String>> = Box::new(Test {
            value: "hello world".to_string(),
        });

        let mut serializer = AlignedSerializer::new(AlignedVec::new());
        let i32_pos = serializer
            .serialize_value(&i32_value)
            .expect("failed to archive value");
        let string_pos = serializer
            .serialize_value(&string_value)
            .expect("failed to archive value");
        let buf = serializer.into_inner();
        let i32_archived_value =
            unsafe { archived_value::<Box<dyn STestTrait<i32>>>(buf.as_ref(), i32_pos) };
        let string_archived_value =
            unsafe { archived_value::<Box<dyn STestTrait<String>>>(buf.as_ref(), string_pos) };
        assert_eq!(i32_value.get_value(), i32_archived_value.get_value());
        assert_eq!(string_value.get_value(), string_archived_value.get_value());

        // exercise vtable cache
        assert_eq!(i32_value.get_value(), i32_archived_value.get_value());
        assert_eq!(i32_value.get_value(), i32_archived_value.get_value());

        let i32_deserialized_value: Box<dyn STestTrait<i32>> = i32_archived_value
            .deserialize(&mut AllocDeserializer)
            .unwrap();
        assert_eq!(i32_value.get_value(), i32_deserialized_value.get_value());
        assert_eq!(i32_value.get_value(), i32_deserialized_value.get_value());

        assert_eq!(string_value.get_value(), string_archived_value.get_value());
        assert_eq!(string_value.get_value(), string_archived_value.get_value());

        let string_deserialized_value: Box<dyn STestTrait<String>> = string_archived_value
            .deserialize(&mut AllocDeserializer)
            .unwrap();
        assert_eq!(
            string_value.get_value(),
            string_deserialized_value.get_value()
        );
        assert_eq!(
            string_value.get_value(),
            string_deserialized_value.get_value()
        );
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn derive_visibility() {
        mod inner {
            #[derive(super::Archive, super::Serialize)]
            pub struct TestTuple(pub i32);

            #[derive(super::Archive, super::Serialize)]
            pub struct TestStruct {
                pub value: i32,
            }

            #[derive(super::Archive, super::Serialize)]
            pub enum TestEnum {
                B(i32),
                C { value: i32 },
            }
        }

        use inner::{
            ArchivedTestEnum, ArchivedTestStruct, ArchivedTestTuple, TestEnum, TestStruct,
            TestTuple,
        };

        TestTuple(42.into());
        ArchivedTestTuple(42.into());
        TestStruct { value: 42.into() };
        ArchivedTestStruct { value: 42.into() };
        TestEnum::B(42.into());
        TestEnum::C { value: 42.into() };
        ArchivedTestEnum::B(42.into());
        ArchivedTestEnum::C { value: 42.into() };
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn basic_mutable_refs() {
        let mut serializer = AlignedSerializer::new(AlignedVec::new());
        serializer.serialize_value(&42i32).unwrap();
        let mut buf = serializer.into_inner();
        let mut value = unsafe { archived_root_mut::<i32>(Pin::new(buf.as_mut())) };
        assert_eq!(*value, 42);
        *value = 11.into();
        assert_eq!(*value, 11);
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn struct_mutable_refs() {
        use std::collections::HashMap;

        #[derive(Archive, Serialize)]
        struct Test {
            a: Box<i32>,
            b: Vec<String>,
            c: HashMap<i32, [i32; 2]>,
        }

        impl ArchivedTest {
            fn a(self: Pin<&mut Self>) -> Pin<&mut Archived<Box<i32>>> {
                unsafe { self.map_unchecked_mut(|s| &mut s.a) }
            }

            fn b(self: Pin<&mut Self>) -> Pin<&mut Archived<Vec<String>>> {
                unsafe { self.map_unchecked_mut(|s| &mut s.b) }
            }

            fn c(self: Pin<&mut Self>) -> Pin<&mut Archived<HashMap<i32, [i32; 2]>>> {
                unsafe { self.map_unchecked_mut(|s| &mut s.c) }
            }
        }

        let mut value = Test {
            a: Box::new(10),
            b: vec!["hello".to_string(), "world".to_string()],
            c: HashMap::new(),
        };

        value.c.insert(1, [4, 2]);
        value.c.insert(5, [17, 24]);

        let mut serializer = AlignedSerializer::new(AlignedVec::new());
        serializer.serialize_value(&value).unwrap();
        let mut buf = serializer.into_inner();
        let mut value = unsafe { archived_root_mut::<Test>(Pin::new(buf.as_mut())) };

        assert_eq!(*value.a, 10);
        assert_eq!(value.b.len(), 2);
        assert_eq!(value.b[0], "hello");
        assert_eq!(value.b[1], "world");
        assert_eq!(value.c.len(), 2);
        assert_eq!(value.c.get(&1.into()).unwrap(), &[4, 2]);
        assert_eq!(value.c.get(&5.into()).unwrap(), &[17, 24]);

        *value.as_mut().a().get_pin() = 50.into();
        assert_eq!(*value.a, 50);

        value
            .as_mut()
            .b()
            .index_pin(0)
            .str_pin()
            .make_ascii_uppercase();
        value
            .as_mut()
            .b()
            .index_pin(1)
            .str_pin()
            .make_ascii_uppercase();
        assert_eq!(value.b[0], "HELLO");
        assert_eq!(value.b[1], "WORLD");

        let mut c1 = value.as_mut().c().get_pin(&1.into()).unwrap();
        c1[0] = 7.into();
        c1[1] = 18.into();
        assert_eq!(value.c.get(&1.into()).unwrap(), &[7, 18]);
        let mut c5 = value.as_mut().c().get_pin(&5.into()).unwrap();
        c5[0] = 6.into();
        c5[1] = 99.into();
        assert_eq!(value.c.get(&5.into()).unwrap(), &[6, 99]);
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn enum_mutable_ref() {
        #[allow(dead_code)]
        #[derive(Archive, Serialize)]
        enum Test {
            A,
            B(char),
            C(i32),
        }

        let value = Test::A;

        let mut serializer = AlignedSerializer::new(AlignedVec::new());
        serializer.serialize_value(&value).unwrap();
        let mut buf = serializer.into_inner();
        let mut value = unsafe { archived_root_mut::<Test>(Pin::new(buf.as_mut())) };

        if let Archived::<Test>::A = *value {
            ()
        } else {
            panic!("incorrect enum after archiving");
        }

        *value = Archived::<Test>::C(42.into());

        if let Archived::<Test>::C(i) = *value {
            assert_eq!(i, 42);
        } else {
            panic!("incorrect enum after mutation");
        }
    }

    #[test]
    #[cfg(not(feature = "wasm"))]
    fn mutable_dyn_ref() {
        use rkyv_dyn::archive_dyn;
        use rkyv_typename::TypeName;

        #[archive_dyn]
        trait TestTrait {
            fn value(&self) -> i32;
            fn set_value(self: Pin<&mut Self>, value: i32);
        }

        #[derive(Archive, Serialize)]
        #[archive_attr(derive(TypeName))]
        struct Test(i32);

        #[archive_dyn]
        impl TestTrait for Test {
            fn value(&self) -> i32 {
                self.0
            }
            fn set_value(self: Pin<&mut Self>, value: i32) {
                unsafe {
                    let s = self.get_unchecked_mut();
                    s.0 = value;
                }
            }
        }

        impl TestTrait for Archived<Test> {
            fn value(&self) -> i32 {
                self.0.into()
            }
            fn set_value(self: Pin<&mut Self>, value: i32) {
                unsafe {
                    let s = self.get_unchecked_mut();
                    s.0 = value.into();
                }
            }
        }

        let value = Box::new(Test(10)) as Box<dyn SerializeTestTrait>;

        let mut serializer = AlignedSerializer::new(AlignedVec::new());
        serializer.serialize_value(&value).unwrap();
        let mut buf = serializer.into_inner();
        let mut value =
            unsafe { archived_root_mut::<Box<dyn SerializeTestTrait>>(Pin::new(buf.as_mut())) };

        assert_eq!(value.value(), 10);
        value.as_mut().get_pin().set_value(64);
        assert_eq!(value.value(), 64);
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn recursive_structures() {
        #[derive(Archive, Serialize, Deserialize, PartialEq)]
        #[archive(compare(PartialEq))]
        // The derive macros don't apply the right bounds from Box so we have to manually specify
        // what bounds to apply
        #[archive(bound(serialize = "__S: Serializer", deserialize = "__D: Deserializer"))]
        enum Node {
            Nil,
            Cons(#[omit_bounds] Box<Node>),
        }

        test_archive(&Node::Cons(Box::new(Node::Cons(Box::new(Node::Nil)))));
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn archive_root() {
        use rkyv::{archived_value, Aligned};

        #[derive(Archive, Serialize)]
        #[archive(compare(PartialEq))]
        struct Test {
            a: (),
            b: i32,
            c: String,
            d: Option<i32>,
        }

        let value = Test {
            a: (),
            b: 42,
            c: "hello world".to_string(),
            d: Some(42),
        };

        // FIXME: A `BufferSerializer` is used here because `Seek` is required. For most purposes,
        // we should use a `Vec` and wrap it in a `Cursor` to get `Seek`. In this case,
        // `Cursor<AlignedVec>` can't implement `Write` because it's not implemented in this crate
        // so we use a buffer serializer instead.
        let mut serializer = BufferSerializer::new(Aligned([0u8; BUFFER_SIZE]));
        let pos = serializer
            .serialize_front(&value)
            .expect("failed to archive value");
        let buffer = serializer.into_inner();
        assert_eq!(pos, 0);
        let archived_value = unsafe { archived_value::<Test>(buffer.as_ref(), 0) };
        assert!(*archived_value == value);
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn archive_more_std() {
        use core::{
            num::NonZeroU8,
            ops::{RangeFull, Range, RangeInclusive, RangeFrom, RangeTo, RangeToInclusive},
            sync::atomic::{AtomicU32, Ordering},
        };

        #[derive(Archive, Serialize, Deserialize)]
        struct Test {
            a: AtomicU32,
            b: NonZeroU8,
            c: RangeFull,
            d: Range<i32>,
            e: RangeInclusive<i32>,
            f: RangeFrom<i32>,
            g: RangeTo<i32>,
            h: RangeToInclusive<i32>,
        }

        impl PartialEq for Test {
            fn eq(&self, other: &Self) -> bool {
                self.a.load(Ordering::Relaxed) == other.a.load(Ordering::Relaxed)
                    && self.b == other.b
                    && self.c == other.c
                    && self.d == other.d
                    && self.e == other.e
                    && self.f == other.f
                    && self.g == other.g
                    && self.h == other.h
            }
        }

        // Can't derive PartialEq automatically because AtomicU32 doesn't implement PartialEq
        impl PartialEq<Test> for Archived<Test> {
            fn eq(&self, other: &Test) -> bool {
                self.a.load(Ordering::Relaxed) == other.a.load(Ordering::Relaxed)
                    && self.b == other.b
                    && self.c == other.c
                    && self.d == other.d
                    && self.e == other.e
                    && self.f == other.f
                    && self.g == other.g
                    && self.h == other.h
            }
        }

        let value = Test {
            a: AtomicU32::new(42),
            b: NonZeroU8::new(8).unwrap(),
            c: RangeFull,
            d: Range { start: 14, end: 46 },
            e: RangeInclusive::new(12, 22),
            f: RangeFrom { start: 60 },
            g: RangeTo { end: 35 },
            h: RangeToInclusive { end: 87 },
        };

        test_archive(&value);
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn archive_net() {
        use std::net::{Ipv4Addr, Ipv6Addr, IpAddr, SocketAddrV4, SocketAddrV6, SocketAddr};

        #[derive(Archive, Serialize, Deserialize, PartialEq)]
        #[archive(compare(PartialEq))]
        struct TestNet {
            ipv4: Ipv4Addr,
            ipv6: Ipv6Addr,
            ip: IpAddr,
            sockv4: SocketAddrV4,
            sockv6: SocketAddrV6,
            sock: SocketAddr,
        }

        let value = TestNet {
            ipv4: Ipv4Addr::new(31, 41, 59, 26),
            ipv6: Ipv6Addr::new(31, 41, 59, 26, 53, 58, 97, 93),
            ip: IpAddr::V4(Ipv4Addr::new(31, 41, 59, 26)),
            sockv4: SocketAddrV4::new(Ipv4Addr::new(31, 41, 59, 26), 5358),
            sockv6: SocketAddrV6::new(
                Ipv6Addr::new(31, 31, 59, 26, 53, 58, 97, 93),
                2384,
                0,
                0,
            ),
            sock: SocketAddr::V6(SocketAddrV6::new(
                Ipv6Addr::new(31, 31, 59, 26, 53, 58, 97, 93),
                2384,
                0,
                0,
            )),
        };

        test_archive(&value);
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn archive_shared_ptr() {
        use std::rc::Rc;

        #[derive(Archive, Deserialize, Serialize, Eq, PartialEq)]
        #[archive(compare(PartialEq))]
        struct Test {
            a: Rc<u32>,
            b: Rc<u32>,
        }

        impl ArchivedTest {
            fn a(self: Pin<&mut Self>) -> Pin<&mut Archived<Rc<u32>>> {
                unsafe { self.map_unchecked_mut(|s| &mut s.a) }
            }

            fn b(self: Pin<&mut Self>) -> Pin<&mut Archived<Rc<u32>>> {
                unsafe { self.map_unchecked_mut(|s| &mut s.b) }
            }
        }

        let shared = Rc::new(10);
        let value = Test {
            a: shared.clone(),
            b: shared.clone(),
        };

        let mut serializer =
            SharedSerializerAdapter::new(AlignedSerializer::new(AlignedVec::new()));
        serializer
            .serialize_value(&value)
            .expect("failed to archive value");
        let mut buf = serializer.into_inner().into_inner();

        let archived = unsafe { archived_root::<Test>(buf.as_ref()) };
        assert!(archived == &value);

        let mut mutable_archived =
            unsafe { archived_root_mut::<Test>(Pin::new_unchecked(buf.as_mut())) };
        unsafe {
            *mutable_archived.as_mut().a().get_pin_unchecked() = 42u32.into();
        }

        let archived = unsafe { archived_root::<Test>(buf.as_ref()) };
        assert_eq!(*archived.a, 42);
        assert_eq!(*archived.b, 42);

        let mut mutable_archived =
            unsafe { archived_root_mut::<Test>(Pin::new_unchecked(buf.as_mut())) };
        unsafe {
            *mutable_archived.as_mut().b().get_pin_unchecked() = 17u32.into();
        }

        let archived = unsafe { archived_root::<Test>(buf.as_ref()) };
        assert_eq!(*archived.a, 17);
        assert_eq!(*archived.b, 17);

        let mut deserializer = SharedDeserializerAdapter::new(AllocDeserializer);
        let deserialized = archived.deserialize(&mut deserializer).unwrap();

        assert_eq!(*deserialized.a, 17);
        assert_eq!(*deserialized.b, 17);
        assert_eq!(
            &*deserialized.a as *const u32,
            &*deserialized.b as *const u32
        );
        assert_eq!(Rc::strong_count(&deserialized.a), 3);
        assert_eq!(Rc::strong_count(&deserialized.b), 3);
        assert_eq!(Rc::weak_count(&deserialized.a), 0);
        assert_eq!(Rc::weak_count(&deserialized.b), 0);

        core::mem::drop(deserializer);

        assert_eq!(*deserialized.a, 17);
        assert_eq!(*deserialized.b, 17);
        assert_eq!(
            &*deserialized.a as *const u32,
            &*deserialized.b as *const u32
        );
        assert_eq!(Rc::strong_count(&deserialized.a), 2);
        assert_eq!(Rc::strong_count(&deserialized.b), 2);
        assert_eq!(Rc::weak_count(&deserialized.a), 0);
        assert_eq!(Rc::weak_count(&deserialized.b), 0);
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn archive_unsized_shared_ptr() {
        use std::rc::Rc;

        #[derive(Archive, Serialize, Deserialize, PartialEq)]
        #[archive(compare(PartialEq))]
        struct Test {
            a: Rc<[String]>,
            b: Rc<[String]>,
        }

        let rc_slice =
            Rc::<[String]>::from(vec!["hello".to_string(), "world".to_string()].into_boxed_slice());
        let value = Test {
            a: rc_slice.clone(),
            b: rc_slice,
        };

        test_archive(&value);
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn archive_weak_ptr() {
        use std::rc::{Rc, Weak};

        #[derive(Archive, Serialize, Deserialize)]
        struct Test {
            a: Rc<u32>,
            b: Weak<u32>,
        }

        impl ArchivedTest {
            fn a(self: Pin<&mut Self>) -> Pin<&mut Archived<Rc<u32>>> {
                unsafe { self.map_unchecked_mut(|s| &mut s.a) }
            }

            fn b(self: Pin<&mut Self>) -> Pin<&mut Archived<Weak<u32>>> {
                unsafe { self.map_unchecked_mut(|s| &mut s.b) }
            }
        }

        let shared = Rc::new(10);
        let value = Test {
            a: shared.clone(),
            b: Rc::downgrade(&shared),
        };

        let mut serializer =
            SharedSerializerAdapter::new(AlignedSerializer::new(AlignedVec::new()));
        serializer
            .serialize_value(&value)
            .expect("failed to archive value");
        let mut buf = serializer.into_inner().into_inner();

        let archived = unsafe { archived_root::<Test>(buf.as_ref()) };
        assert_eq!(*archived.a, 10);
        assert!(archived.b.upgrade().is_some());
        assert_eq!(**archived.b.upgrade().unwrap(), 10);

        let mut mutable_archived =
            unsafe { archived_root_mut::<Test>(Pin::new_unchecked(buf.as_mut())) };
        unsafe {
            *mutable_archived.as_mut().a().get_pin_unchecked() = 42u32.into();
        }

        let archived = unsafe { archived_root::<Test>(buf.as_ref()) };
        assert_eq!(*archived.a, 42);
        assert!(archived.b.upgrade().is_some());
        assert_eq!(**archived.b.upgrade().unwrap(), 42);

        let mut mutable_archived =
            unsafe { archived_root_mut::<Test>(Pin::new_unchecked(buf.as_mut())) };
        unsafe {
            *mutable_archived
                .as_mut()
                .b()
                .upgrade_pin()
                .unwrap()
                .get_pin_unchecked() = 17u32.into();
        }

        let archived = unsafe { archived_root::<Test>(buf.as_ref()) };
        assert_eq!(*archived.a, 17);
        assert!(archived.b.upgrade().is_some());
        assert_eq!(**archived.b.upgrade().unwrap(), 17);

        let mut deserializer = SharedDeserializerAdapter::new(AllocDeserializer);
        let deserialized = archived.deserialize(&mut deserializer).unwrap();

        assert_eq!(*deserialized.a, 17);
        assert!(deserialized.b.upgrade().is_some());
        assert_eq!(*deserialized.b.upgrade().unwrap(), 17);
        assert_eq!(
            &*deserialized.a as *const u32,
            &*deserialized.b.upgrade().unwrap() as *const u32
        );
        assert_eq!(Rc::strong_count(&deserialized.a), 2);
        assert_eq!(Weak::strong_count(&deserialized.b), 2);
        assert_eq!(Rc::weak_count(&deserialized.a), 1);
        assert_eq!(Weak::weak_count(&deserialized.b), 1);

        core::mem::drop(deserializer);

        assert_eq!(*deserialized.a, 17);
        assert!(deserialized.b.upgrade().is_some());
        assert_eq!(*deserialized.b.upgrade().unwrap(), 17);
        assert_eq!(
            &*deserialized.a as *const u32,
            &*deserialized.b.upgrade().unwrap() as *const u32
        );
        assert_eq!(Rc::strong_count(&deserialized.a), 1);
        assert_eq!(Weak::strong_count(&deserialized.b), 1);
        assert_eq!(Rc::weak_count(&deserialized.a), 1);
        assert_eq!(Weak::weak_count(&deserialized.b), 1);
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn derive_attributes() {
        use rkyv::Fallible;

        #[derive(Archive, PartialEq)]
        #[archive(archived = "ATest", resolver = "RTest", compare(PartialEq))]
        struct Test {
            a: i32,
            b: Option<u32>,
            c: String,
            d: Vec<i32>,
        }

        impl<S: Fallible + ?Sized> Serialize<S> for Test
        where
            i32: Serialize<S>,
            Option<u32>: Serialize<S>,
            String: Serialize<S>,
            Vec<i32>: Serialize<S>,
        {
            fn serialize(&self, serializer: &mut S) -> Result<RTest, S::Error> {
                Ok(RTest {
                    a: self.a.serialize(serializer)?,
                    b: self.b.serialize(serializer)?,
                    c: self.c.serialize(serializer)?,
                    d: self.d.serialize(serializer)?,
                })
            }
        }

        impl<D: Fallible + ?Sized> Deserialize<Test, D> for ATest
        where
            Archived<i32>: Deserialize<i32, D>,
            Archived<Option<u32>>: Deserialize<Option<u32>, D>,
            Archived<String>: Deserialize<String, D>,
            Archived<Vec<i32>>: Deserialize<Vec<i32>, D>,
        {
            fn deserialize(&self, deserializer: &mut D) -> Result<Test, D::Error> {
                Ok(Test {
                    a: self.a.deserialize(deserializer)?,
                    b: self.b.deserialize(deserializer)?,
                    c: self.c.deserialize(deserializer)?,
                    d: self.d.deserialize(deserializer)?,
                })
            }
        }

        let value = Test {
            a: 42,
            b: Some(12),
            c: "hello world".to_string(),
            d: vec![1, 2, 3, 4],
        };

        test_archive(&value);
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn compare() {
        #[derive(Archive, Serialize, Deserialize)]
        #[archive(compare(PartialEq, PartialOrd))]
        pub struct UnitFoo;

        #[derive(Archive, Serialize, Deserialize)]
        #[archive(compare(PartialEq, PartialOrd))]
        pub struct TupleFoo(i32);

        #[derive(Archive, Serialize, Deserialize)]
        #[archive(compare(PartialEq, PartialOrd))]
        pub struct StructFoo {
            t: i32,
        }

        #[derive(Archive, Serialize, Deserialize)]
        #[archive(compare(PartialEq, PartialOrd))]
        pub enum EnumFoo {
            #[allow(dead_code)]
            Foo(i32),
        }
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn default_type_parameters() {
        #[derive(Archive, Serialize, Deserialize)]
        pub struct TupleFoo<T = i32>(T);

        #[derive(Archive, Serialize, Deserialize)]
        pub struct StructFoo<T = i32> {
            t: T,
        }

        #[derive(Archive, Serialize, Deserialize)]
        pub enum EnumFoo<T = i32> {
            #[allow(dead_code)]
            T(T),
        }
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn check_util_bounds() {
        use rkyv::{
            ser::{serializers::AlignedSerializer, Serializer},
            Aligned,
        };
        fn check<T: Serializer>() {}

        check::<BufferSerializer<[u8; 256]>>();
        check::<BufferSerializer<&mut [u8; 256]>>();
        check::<BufferSerializer<&mut [u8]>>();
        check::<BufferSerializer<Aligned<[u8; 256]>>>();
        check::<BufferSerializer<&mut Aligned<[u8; 256]>>>();
        check::<AlignedSerializer<AlignedVec>>();
        check::<AlignedSerializer<&mut AlignedVec>>();
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn const_generics() {
        #[derive(Archive, Deserialize, Serialize, PartialEq)]
        #[archive(compare(PartialEq))]
        pub struct Const<const N: usize>;

        test_archive(&Const::<1>);
        test_archive(&Const::<2>);
        test_archive(&Const::<3>);

        #[derive(Archive, Deserialize, Serialize)]
        pub struct Array<T, const N: usize>([T; N]);
    }

    #[test]
    #[cfg(any(
        not(any(feature = "archive_le", feature = "archive_be")),
        feature = "arbitrary_enum_discriminant"
    ))]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn arbitrary_enum_discriminant() {
        use rkyv::Infallible;

        #[derive(Archive, Deserialize, Serialize, Debug, PartialEq)]
        #[archive_attr(derive(Debug, PartialEq))]
        enum ReallyBigEnum {
            V00, V01, V02, V03, V04, V05, V06, V07, V08, V09, V0A, V0B, V0C, V0D, V0E, V0F,
            V10, V11, V12, V13, V14, V15, V16, V17, V18, V19, V1A, V1B, V1C, V1D, V1E, V1F,
            V20, V21, V22, V23, V24, V25, V26, V27, V28, V29, V2A, V2B, V2C, V2D, V2E, V2F,
            V30, V31, V32, V33, V34, V35, V36, V37, V38, V39, V3A, V3B, V3C, V3D, V3E, V3F,
            V40, V41, V42, V43, V44, V45, V46, V47, V48, V49, V4A, V4B, V4C, V4D, V4E, V4F,
            V50, V51, V52, V53, V54, V55, V56, V57, V58, V59, V5A, V5B, V5C, V5D, V5E, V5F,
            V60, V61, V62, V63, V64, V65, V66, V67, V68, V69, V6A, V6B, V6C, V6D, V6E, V6F,
            V70, V71, V72, V73, V74, V75, V76, V77, V78, V79, V7A, V7B, V7C, V7D, V7E, V7F,
            V80, V81, V82, V83, V84, V85, V86, V87, V88, V89, V8A, V8B, V8C, V8D, V8E, V8F,
            V90, V91, V92, V93, V94, V95, V96, V97, V98, V99, V9A, V9B, V9C, V9D, V9E, V9F,
            VA0, VA1, VA2, VA3, VA4, VA5, VA6, VA7, VA8, VA9, VAA, VAB, VAC, VAD, VAE, VAF,
            VB0, VB1, VB2, VB3, VB4, VB5, VB6, VB7, VB8, VB9, VBA, VBB, VBC, VBD, VBE, VBF,
            VC0, VC1, VC2, VC3, VC4, VC5, VC6, VC7, VC8, VC9, VCA, VCB, VCC, VCD, VCE, VCF,
            VD0, VD1, VD2, VD3, VD4, VD5, VD6, VD7, VD8, VD9, VDA, VDB, VDC, VDD, VDE, VDF,
            VE0, VE1, VE2, VE3, VE4, VE5, VE6, VE7, VE8, VE9, VEA, VEB, VEC, VED, VEE, VEF,
            VF0, VF1, VF2, VF3, VF4, VF5, VF6, VF7, VF8, VF9, VFA, VFB, VFC, VFD, VFE, VFF,
            V100,
        }

        assert_eq!(ReallyBigEnum::V100 as u16, 0x100u16);

        #[cfg(not(any(
            all(target_endian = "little", feature = "archive_be"),
            all(target_endian = "big", feature = "archive_le")
        )))]
        assert_eq!(ArchivedReallyBigEnum::V100 as u16, 0x100u16);
        #[cfg(any(
            all(target_endian = "little", feature = "archive_be"),
            all(target_endian = "big", feature = "archive_le")
        ))]
        assert_eq!(ArchivedReallyBigEnum::V100 as u16, 0x1u16);

        let mut serializer = make_default_serializer();
        serializer.serialize_value(&ReallyBigEnum::V100).unwrap();
        let len = serializer.pos();
        let buf = unwrap_default_serializer(serializer);

        let archived = unsafe { archived_root::<ReallyBigEnum>(&buf.as_ref()[..len]) };
        assert_eq!(archived, &ArchivedReallyBigEnum::V100);

        let deserialized = archived.deserialize(&mut Infallible).unwrap();
        assert_eq!(deserialized, ReallyBigEnum::V100);
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    #[cfg(not(feature = "strict"))]
    fn repr_rust() {
        #[derive(Archive)]
        pub struct Test {
            a: u8,
            b: u16,
            c: u8,
        }

        assert_eq!(core::mem::size_of::<ArchivedTest>(), 4);
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn repr_transparent() {
        #[derive(Archive)]
        #[archive(repr(transparent))]
        pub struct Test {
            a: u32,
        }

        assert_eq!(core::mem::size_of::<ArchivedTest>(), 4);
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn repr_c() {
        #[derive(Archive)]
        #[archive(repr(C))]
        pub struct TestStruct {
            a: u8,
            b: u16,
            c: u8,
        }

        assert_eq!(core::mem::size_of::<ArchivedTestStruct>(), 6);
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn repr_int() {
        #[derive(Archive)]
        #[allow(dead_code)]
        pub enum InferredRepr {
            V0,
            V1,
        }

        assert_eq!(core::mem::size_of::<ArchivedInferredRepr>(), 1);

        #[derive(Archive)]
        #[archive(repr(u16))]
        #[allow(dead_code)]
        pub enum ExplicitRepr {
            V0,
            V1,
        }

        assert_eq!(core::mem::size_of::<ArchivedExplicitRepr>(), 2);
    }
}
