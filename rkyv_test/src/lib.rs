#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(all(test, feature = "validation"))]
mod validation;

#[cfg(test)]
mod util {
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
    pub type DefaultSerializer<'a> =
        SharedSerializerAdapter<BufferSerializer<'a, Aligned<[u8; BUFFER_SIZE]>>>;

    #[cfg(feature = "std")]
    pub fn make_default_serializer<'a>(buffer: &'a mut Aligned<[u8; BUFFER_SIZE]>) -> DefaultSerializer<'a> {
        SharedSerializerAdapter::new(BufferSerializer::new(buffer))
    }

    #[cfg(feature = "std")]
    pub type DefaultDeserializer = SharedDeserializerAdapter<AllocDeserializer>;

    #[cfg(feature = "std")]
    pub fn make_default_deserializer() -> DefaultDeserializer {
        SharedDeserializerAdapter::new(AllocDeserializer)
    }

    #[cfg(not(feature = "std"))]
    pub type DefaultSerializer<'a> = BufferSerializer<'a, Aligned<[u8; BUFFER_SIZE]>>;

    #[cfg(not(feature = "std"))]
    pub fn make_default_serializer<'a>(buffer: &'a mut Aligned<[u8; BUFFER_SIZE]>) -> DefaultSerializer<'a> {
        BufferSerializer::new(buffer)
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

    pub fn test_archive<T: for<'a> Serialize<DefaultSerializer<'a>>>(value: &T)
    where
        T: PartialEq,
        T::Archived: PartialEq<T> + Deserialize<T, DefaultDeserializer>,
    {
        let mut buffer = Aligned([0u8; BUFFER_SIZE]);
        let mut serializer = make_default_serializer(&mut buffer);
        serializer
            .serialize_value(value)
            .expect("failed to archive value");
        let len = serializer.pos();
        let archived_value = unsafe { archived_root::<T>(&buffer.as_ref()[0..len]) };
        assert!(archived_value == value);
        let mut deserializer = make_default_deserializer();
        assert!(&archived_value.deserialize(&mut deserializer).unwrap() == value);
    }

    pub fn test_archive_ref<T: for<'a> SerializeUnsized<DefaultSerializer<'a>> + ?Sized>(value: &T)
    where
        T::Archived: PartialEq<T>,
    {
        let mut buffer = Aligned([0u8; BUFFER_SIZE]);
        let mut serializer = make_default_serializer(&mut buffer);
        serializer
            .serialize_unsized_value(value)
            .expect("failed to archive ref");
        let len = serializer.pos();
        let archived_ref = unsafe { archived_unsized_root::<T>(&buffer.as_ref()[0..len]) };
        assert!(archived_ref == value);
    }

    #[cfg(feature = "std")]
    pub fn test_archive_container<
        T: for<'a> Serialize<DefaultSerializer<'a>, Archived = U> + core::ops::Deref<Target = TV>,
        TV: ?Sized,
        U: core::ops::Deref<Target = TU>,
        TU: PartialEq<TV> + ?Sized,
    >(
        value: &T,
    ) {
        let mut buffer = Aligned([0u8; BUFFER_SIZE]);
        let mut serializer = make_default_serializer(&mut buffer);
        serializer
            .serialize_value(value)
            .expect("failed to archive ref");
        let len = serializer.pos();
        let archived_ref = unsafe { archived_root::<T>(&buffer.as_ref()[0..len]) };
        assert!(archived_ref.deref() == value.deref());
    }
}

#[cfg(test)]
mod no_std_tests {
    use crate::util::*;

    #[test]
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
        #[cfg(not(feature = "strict"))]
        test_archive(&(24, true, 16f32));
        test_archive(&[1, 2, 3, 4, 5, 6]);

        test_archive(&Option::<()>::None);
        test_archive(&Some(42));
    }

    #[test]
    fn archive_refs() {
        #[cfg(not(feature = "strict"))]
        test_archive_ref::<[i32; 4]>(&[1, 2, 3, 4]);
        test_archive_ref::<str>("hello world");
        test_archive_ref::<[i32]>([1, 2, 3, 4].as_ref());
    }

    #[test]
    fn archive_slices() {
        test_archive_ref::<str>("hello world");
        test_archive_ref::<[i32]>([1, 2, 3, 4].as_ref());
    }

    #[test]
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
            serializers::{BufferSerializer, WriteSerializer},
            SeekSerializer, Serializer,
        },
        AlignedVec, Archive, ArchiveUnsized, Archived, Deserialize, DeserializeUnsized, Serialize,
        SerializeUnsized,
    };

    #[test]
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
    fn archive_composition() {
        test_archive(&Some(Box::new(42)));
        test_archive(&Some("hello world".to_string().into_boxed_str()));
        test_archive(&Some(vec![1, 2, 3, 4].into_boxed_slice()));
        test_archive(&Some("hello world".to_string()));
        test_archive(&Some(vec![1, 2, 3, 4]));
        test_archive(&Some(Box::new(vec![1, 2, 3, 4])));
    }

    mod example {
        #[test]
        fn archive_example() {
            use rkyv::{
                archived_root,
                de::deserializers::AllocDeserializer,
                ser::{serializers::WriteSerializer, Serializer},
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

            let mut serializer = WriteSerializer::new(AlignedVec::new());
            serializer
                .serialize_value(&value)
                .expect("failed to serialize value");
            let buf = serializer.into_inner();

            let archived = unsafe { archived_root::<Test>(buf.as_ref()) };
            assert_eq!(archived.int, value.int);
            assert_eq!(archived.string, value.string);
            assert_eq!(archived.option, value.option);

            let mut deserializer = AllocDeserializer;
            let deserialized = archived
                .deserialize(&mut deserializer)
                .expect("failed to deserialize value");
            assert_eq!(deserialized, value);
        }
    }

    #[test]
    fn archive_hash_map() {
        use std::collections::HashMap;

        test_archive(&HashMap::<i32, i32>::new());

        let mut hash_map = HashMap::new();
        hash_map.insert(1, 2);
        hash_map.insert(3, 4);
        hash_map.insert(5, 6);
        hash_map.insert(7, 8);

        test_archive(&hash_map);

        let mut hash_map = HashMap::new();
        hash_map.insert("hello".to_string(), "world".to_string());
        hash_map.insert("foo".to_string(), "bar".to_string());
        hash_map.insert("baz".to_string(), "bat".to_string());

        let mut serializer = WriteSerializer::new(AlignedVec::new());
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
    fn archive_unit_struct() {
        #[derive(Archive, Serialize, Deserialize, PartialEq)]
        #[archive(compare(PartialEq))]
        struct Test;

        test_archive(&Test);
        test_archive(&vec![Test, Test]);
    }

    #[test]
    fn archive_tuple_struct() {
        #[derive(Archive, Serialize, Deserialize, PartialEq)]
        #[archive(compare(PartialEq))]
        struct Test((), i32, String, Option<i32>);

        test_archive(&Test((), 42, "hello world".to_string(), Some(42)));
    }

    #[test]
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
    fn archive_copy() {
        #[derive(Archive, Serialize, Deserialize, Clone, Copy, PartialEq)]
        #[archive(copy)]
        struct TestUnit;

        test_archive(&TestUnit);

        #[derive(Archive, Serialize, Deserialize, Clone, Copy, PartialEq)]
        #[archive(copy)]
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
        #[archive(copy)]
        struct TestTuple((), i32, bool, f32, TestUnit);

        test_archive(&TestTuple((), 42, true, 3.14f32, TestUnit));

        #[derive(Archive, Serialize, Deserialize, Clone, Copy, PartialEq)]
        #[repr(u8)]
        #[archive(copy)]
        enum TestEnum {
            A((), i32, bool, f32, TestUnit),
        }

        test_archive(&TestEnum::A((), 42, true, 3.14f32, TestUnit));

        #[derive(Archive, Serialize, Deserialize, Clone, Copy, PartialEq)]
        #[archive(copy)]
        struct TestGeneric<T>(T);

        test_archive(&TestGeneric(42));
    }

    #[test]
    fn archive_derives() {
        #[derive(Archive, Serialize, Clone)]
        #[archive(derive(Clone, Debug, PartialEq))]
        struct Test(i32);

        let value = Test(42);

        let mut serializer = WriteSerializer::new(AlignedVec::new());
        serializer
            .serialize_value(&value)
            .expect("failed to archive value");
        let buf = serializer.into_inner();
        let archived_value = unsafe { archived_root::<Test>(buf.as_ref()) };

        assert_eq!(archived_value, &archived_value.clone());
    }

    #[test]
    fn manual_archive_dyn() {
        use rkyv::{ArchivePointee, ArchivedMetadata};
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
            ) -> ArchivedMetadata<Self> {
                ArchivedDynMetadata::new(self.archived_type_id())
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
        #[archive(derive(TypeName))]
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
                self.id
            }
        }

        let value: Box<dyn SerializeTestTrait> = Box::new(Test { id: 42 });

        let mut serializer = WriteSerializer::new(AlignedVec::new());
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
    fn archive_dyn() {
        use rkyv::AlignedVec;
        use rkyv_dyn::archive_dyn;
        use rkyv_typename::TypeName;

        #[archive_dyn(serialize = "STestTrait", deserialize = "DTestTrait")]
        pub trait TestTrait {
            fn get_id(&self) -> i32;
        }

        #[derive(Archive, Serialize, Deserialize)]
        #[archive(derive(TypeName))]
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
                self.id
            }
        }

        let value: Box<dyn STestTrait> = Box::new(Test { id: 42 });

        let mut serializer = WriteSerializer::new(AlignedVec::new());
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
        #[archive(derive(TypeName))]
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
                self.value
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

        let mut serializer = WriteSerializer::new(AlignedVec::new());
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

        TestTuple(42);
        ArchivedTestTuple(42);
        TestStruct { value: 42 };
        ArchivedTestStruct { value: 42 };
        TestEnum::B(42);
        TestEnum::C { value: 42 };
        ArchivedTestEnum::B(42);
        ArchivedTestEnum::C { value: 42 };
    }

    #[test]
    fn basic_mutable_refs() {
        let mut serializer = WriteSerializer::new(AlignedVec::new());
        serializer.serialize_value(&42i32).unwrap();
        let mut buf = serializer.into_inner();
        let mut value = unsafe { archived_root_mut::<i32>(Pin::new(buf.as_mut())) };
        assert_eq!(*value, 42);
        *value = 11;
        assert_eq!(*value, 11);
    }

    #[test]
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

        let mut serializer = WriteSerializer::new(AlignedVec::new());
        serializer.serialize_value(&value).unwrap();
        let mut buf = serializer.into_inner();
        let mut value = unsafe { archived_root_mut::<Test>(Pin::new(buf.as_mut())) };

        assert_eq!(*value.a, 10);
        assert_eq!(value.b.len(), 2);
        assert_eq!(value.b[0], "hello");
        assert_eq!(value.b[1], "world");
        assert_eq!(value.c.len(), 2);
        assert_eq!(value.c.get(&1).unwrap(), &[4, 2]);
        assert_eq!(value.c.get(&5).unwrap(), &[17, 24]);

        *value.as_mut().a().get_pin() = 50;
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

        let mut c1 = value.as_mut().c().get_pin(&1).unwrap();
        c1[0] = 7;
        c1[1] = 18;
        assert_eq!(value.c.get(&1).unwrap(), &[7, 18]);
        let mut c5 = value.as_mut().c().get_pin(&5).unwrap();
        c5[0] = 6;
        c5[1] = 99;
        assert_eq!(value.c.get(&5).unwrap(), &[6, 99]);
    }

    #[test]
    fn enum_mutable_ref() {
        #[allow(dead_code)]
        #[derive(Archive, Serialize)]
        enum Test {
            A,
            B(char),
            C(i32),
        }

        let value = Test::A;

        let mut serializer = WriteSerializer::new(AlignedVec::new());
        serializer.serialize_value(&value).unwrap();
        let mut buf = serializer.into_inner();
        let mut value = unsafe { archived_root_mut::<Test>(Pin::new(buf.as_mut())) };

        if let Archived::<Test>::A = *value {
            ()
        } else {
            panic!("incorrect enum after archiving");
        }

        *value = Archived::<Test>::C(42);

        if let Archived::<Test>::C(i) = *value {
            assert_eq!(i, 42);
        } else {
            panic!("incorrect enum after mutation");
        }
    }

    #[test]
    fn mutable_dyn_ref() {
        use rkyv_dyn::archive_dyn;
        use rkyv_typename::TypeName;

        #[archive_dyn]
        trait TestTrait {
            fn value(&self) -> i32;
            fn set_value(self: Pin<&mut Self>, value: i32);
        }

        #[derive(Archive, Serialize)]
        #[archive(derive(TypeName))]
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
                self.0
            }
            fn set_value(self: Pin<&mut Self>, value: i32) {
                unsafe {
                    let s = self.get_unchecked_mut();
                    s.0 = value;
                }
            }
        }

        let value = Box::new(Test(10)) as Box<dyn SerializeTestTrait>;

        let mut serializer = WriteSerializer::new(AlignedVec::new());
        serializer.serialize_value(&value).unwrap();
        let mut buf = serializer.into_inner();
        let mut value =
            unsafe { archived_root_mut::<Box<dyn SerializeTestTrait>>(Pin::new(buf.as_mut())) };

        assert_eq!(value.value(), 10);
        value.as_mut().get_pin().set_value(64);
        assert_eq!(value.value(), 64);
    }

    #[test]
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
        let mut buffer = Aligned([0u8; BUFFER_SIZE]);
        let mut serializer = BufferSerializer::new(&mut buffer);
        let pos = serializer
            .serialize_front(&value)
            .expect("failed to archive value");
        assert_eq!(pos, 0);
        let archived_value = unsafe { archived_value::<Test>(buffer.as_ref(), 0) };
        assert!(*archived_value == value);
    }

    #[test]
    fn archive_more_std() {
        use core::{
            num::NonZeroU8,
            ops::Range,
            sync::atomic::{AtomicU32, Ordering},
        };

        #[derive(Archive, Serialize, Deserialize)]
        struct Test {
            a: AtomicU32,
            b: Range<i32>,
            c: NonZeroU8,
        }

        impl PartialEq for Test {
            fn eq(&self, other: &Self) -> bool {
                self.a.load(Ordering::Relaxed) == other.a.load(Ordering::Relaxed)
                    && self.b == other.b
                    && self.c == other.c
            }
        }

        // Can't derive PartialEq automatically because AtomicU32 doesn't implement PartialEq
        impl PartialEq<Test> for Archived<Test> {
            fn eq(&self, other: &Test) -> bool {
                self.a.load(Ordering::Relaxed) == other.a.load(Ordering::Relaxed)
                    && self.b == other.b
                    && self.c == other.c
            }
        }

        let value = Test {
            a: AtomicU32::new(42),
            b: Range { start: 14, end: 46 },
            c: NonZeroU8::new(8).unwrap(),
        };

        test_archive(&value);
    }

    #[test]
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

        let mut serializer = SharedSerializerAdapter::new(WriteSerializer::new(AlignedVec::new()));
        serializer
            .serialize_value(&value)
            .expect("failed to archive value");
        let mut buf = serializer.into_inner().into_inner();

        let archived = unsafe { archived_root::<Test>(buf.as_ref()) };
        assert!(archived == &value);

        let mut mutable_archived =
            unsafe { archived_root_mut::<Test>(Pin::new_unchecked(buf.as_mut())) };
        unsafe {
            *mutable_archived.as_mut().a().get_pin_unchecked() = 42;
        }

        let archived = unsafe { archived_root::<Test>(buf.as_ref()) };
        assert_eq!(*archived.a, 42);
        assert_eq!(*archived.b, 42);

        let mut mutable_archived =
            unsafe { archived_root_mut::<Test>(Pin::new_unchecked(buf.as_mut())) };
        unsafe {
            *mutable_archived.as_mut().b().get_pin_unchecked() = 17;
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

        let mut serializer = SharedSerializerAdapter::new(WriteSerializer::new(AlignedVec::new()));
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
            *mutable_archived.as_mut().a().get_pin_unchecked() = 42;
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
                .get_pin_unchecked() = 17;
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
}
