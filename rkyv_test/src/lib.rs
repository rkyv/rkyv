#[cfg(all(test, feature = "validation"))]
mod validation;

#[cfg(test)]
mod tests {
    use core::pin::Pin;
    use rkyv::{
        archived_value, archived_value_mut, archived_value_ref, Aligned, AllocDeserializer, Archive,
        ArchiveRef, Archived, BufferSerializer, GlobalAllocDeserializer, SeekSerializer, Serialize, SerializeRef, Deserialize, Serializer,
    };
    use rkyv_dyn::archive_dyn;
    use rkyv_typename::TypeName;
    use std::collections::HashMap;

    const BUFFER_SIZE: usize = 256;

    fn test_archive<T: Serialize<BufferSerializer<Aligned<[u8; BUFFER_SIZE]>>>>(value: &T)
    where
        T: PartialEq,
        T::Archived: PartialEq<T> + Deserialize<T, GlobalAllocDeserializer>,
    {
        let mut serializer = BufferSerializer::new(Aligned([0u8; BUFFER_SIZE]));
        let pos = serializer.serialize(value).expect("failed to archive value");
        let buf = serializer.into_inner();
        let archived_value = unsafe { archived_value::<T>(buf.as_ref(), pos) };
        assert!(archived_value == value);
        assert!(&archived_value.deserialize(&mut GlobalAllocDeserializer) == value);
    }

    fn test_archive_ref<T: SerializeRef<BufferSerializer<Aligned<[u8; BUFFER_SIZE]>>> + ?Sized>(value: &T)
    where
        T::Archived: PartialEq<T>,
    {
        let mut serializer = BufferSerializer::new(Aligned([0u8; BUFFER_SIZE]));
        let pos = serializer.serialize_ref(value).expect("failed to archive ref");
        let buf = serializer.into_inner();
        let archived_ref = unsafe { archived_value_ref::<T>(buf.as_ref(), pos) };
        assert!(&**archived_ref == value);
    }

    fn test_archive_container<
        T: Serialize<BufferSerializer<Aligned<[u8; BUFFER_SIZE]>>, Archived = U> + core::ops::Deref<Target = TV>,
        TV: ?Sized,
        U: core::ops::Deref<Target = TU>,
        TU: PartialEq<TV> + ?Sized,
    >(
        value: &T,
    ) {
        let mut serializer = BufferSerializer::new(Aligned([0u8; BUFFER_SIZE]));
        let pos = serializer.serialize(value).expect("failed to archive ref");
        let buf = serializer.into_inner();
        let archived_ref = unsafe { archived_value::<T>(buf.as_ref(), pos) };
        assert!(&**archived_ref == &**value);
    }

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
    fn archive_empty_slice() {
        test_archive_ref::<[i32; 0]>(&[]);
        test_archive_ref::<[i32]>([].as_ref());
        test_archive_ref::<str>("");
    }

    #[test]
    fn archive_containers() {
        test_archive_container(&Box::new(42));
        test_archive_container(&"hello world".to_string().into_boxed_str());
        test_archive_container(&vec![1, 2, 3, 4].into_boxed_slice());
        test_archive_container(&"hello world".to_string());
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

    #[test]
    fn archive_hash_map() {
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

        let mut serializer = BufferSerializer::new(Aligned([0u8; BUFFER_SIZE]));
        let pos = serializer.serialize(&hash_map).expect("failed to archive value");
        let buf = serializer.into_inner();
        let archived_value =
            unsafe { archived_value::<HashMap<String, String>>(buf.as_ref(), pos) };

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
        struct Test;

        impl PartialEq<Test> for Archived<Test> {
            fn eq(&self, _other: &Test) -> bool {
                true
            }
        }

        test_archive(&Test);
        test_archive(&vec![Test, Test]);
    }

    #[test]
    fn archive_tuple_struct() {
        #[derive(Archive, Serialize, Deserialize, PartialEq)]
        struct Test((), i32, String, Option<i32>);

        impl PartialEq<Test> for Archived<Test> {
            fn eq(&self, other: &Test) -> bool {
                self.0 == other.0 && self.1 == other.1 && self.2 == other.2 && self.3 == other.3
            }
        }

        test_archive(&Test((), 42, "hello world".to_string(), Some(42)));
    }

    #[test]
    fn archive_simple_struct() {
        #[derive(Archive, Serialize, Deserialize, PartialEq)]
        struct Test {
            a: (),
            b: i32,
            c: String,
            d: Option<i32>,
        }

        impl PartialEq<Test> for Archived<Test> {
            fn eq(&self, other: &Test) -> bool {
                self.a == other.a && self.b == other.b && self.c == other.c && self.d == other.d
            }
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
        struct Test<T: TestTrait> {
            a: (),
            b: <T as TestTrait>::Associated,
            c: String,
            d: Option<i32>,
        }

        impl<T: TestTrait> PartialEq<Test<T>> for Archived<Test<T>>
        where
            <T as TestTrait>::Associated: Archive,
            Archived<<T as TestTrait>::Associated>: PartialEq<<T as TestTrait>::Associated>,
        {
            fn eq(&self, other: &Test<T>) -> bool {
                self.a == other.a && self.b == other.b && self.c == other.c && self.d == other.d
            }
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
        enum Test {
            A,
            B(String),
            C { a: i32, b: String },
        }

        impl PartialEq<Test> for Archived<Test> {
            fn eq(&self, other: &Test) -> bool {
                match self {
                    Self::A => {
                        if let Test::A = other {
                            true
                        } else {
                            false
                        }
                    }
                    Self::B(self_value) => {
                        if let Test::B(other_value) = other {
                            self_value == other_value
                        } else {
                            false
                        }
                    }
                    Self::C { a, b } => {
                        if let Test::C { a: _a, b: _b } = other {
                            a == _a && b == _b
                        } else {
                            false
                        }
                    }
                }
            }
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
        enum Test<T: TestTrait> {
            A,
            B(String),
            C {
                a: <T as TestTrait>::Associated,
                b: String,
            },
        }

        impl<T: TestTrait> PartialEq<Test<T>> for Archived<Test<T>>
        where
            <T as TestTrait>::Associated: Archive,
            Archived<<T as TestTrait>::Associated>: PartialEq<<T as TestTrait>::Associated>,
        {
            fn eq(&self, other: &Test<T>) -> bool {
                match self {
                    Self::A => {
                        if let Test::A = other {
                            true
                        } else {
                            false
                        }
                    }
                    Self::B(self_value) => {
                        if let Test::B(other_value) = other {
                            self_value == other_value
                        } else {
                            false
                        }
                    }
                    Self::C { a, b } => {
                        if let Test::C { a: _a, b: _b } = other {
                            a == _a && b == _b
                        } else {
                            false
                        }
                    }
                }
            }
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

        let mut serializer = BufferSerializer::new(Aligned([0u8; BUFFER_SIZE]));
        let pos = serializer.serialize(&value).expect("failed to archive value");
        let buf = serializer.into_inner();
        let archived_value = unsafe { archived_value::<Test>(buf.as_ref(), pos) };

        assert_eq!(archived_value, &archived_value.clone());
    }

    #[test]
    fn archived_dyn_size() {
        use rkyv_dyn::ArchivedDyn;

        pub trait Test {}

        assert_eq!(core::mem::size_of::<ArchivedDyn<dyn Test>>(), 16);
    }

    #[test]
    fn manual_archive_dyn() {
        use rkyv::{DeserializeRef, Serializer};
        use rkyv_dyn::{
            register_impl, SerializeDyn, ArchivedDyn, DynDeserializer, RegisteredImpl, DeserializeDyn,
        };

        pub trait TestTrait {
            fn get_id(&self) -> i32;
        }

        pub trait SerializeTestTrait: TestTrait + SerializeDyn {}

        impl<T: Archive + SerializeDyn + TestTrait> SerializeTestTrait for T where
            T::Archived: RegisteredImpl<dyn DeserializeTestTrait>
        {
        }

        pub trait DeserializeTestTrait: TestTrait + DeserializeDyn<dyn SerializeTestTrait> {}

        impl<T: TestTrait + DeserializeDyn<dyn SerializeTestTrait>> DeserializeTestTrait for T {}

        impl TypeName for dyn DeserializeTestTrait {
            fn build_type_name<F: FnMut(&str)>(mut f: F) {
                f("dyn DeserializeTestTrait");
            }
        }

        impl ArchiveRef for dyn SerializeTestTrait {
            type Archived = dyn DeserializeTestTrait;
            type Reference = ArchivedDyn<dyn DeserializeTestTrait>;

            fn resolve_ref(&self, pos: usize, resolver: usize) -> Self::Reference {
                ArchivedDyn::new(self.archived_type_id(), pos, resolver)
            }
        }

        impl<S: Serializer + ?Sized> SerializeRef<S> for dyn SerializeTestTrait {
            fn serialize_ref(
                &self,
                mut serializer: &mut S,
            ) -> Result<usize, S::Error> {
                self.serialize_dyn(&mut serializer)
                    .map_err(|e| *e.downcast::<S::Error>().unwrap())
            }
        }

        impl<D: AllocDeserializer> DeserializeRef<dyn SerializeTestTrait, D> for ArchivedDyn<dyn DeserializeTestTrait> {
            unsafe fn deserialize_ref(
                &self,
                deserializer: &mut D,
            ) -> *mut dyn SerializeTestTrait {
                (*self).deserialize_dyn(deserializer)
            }
        }

        #[derive(Archive, Serialize, Deserialize)]
        // TODO: archive parameter to set typename
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
            ) -> *mut dyn SerializeTestTrait {
                let result = deserializer.alloc_dyn(core::alloc::Layout::new::<Test>()) as *mut Test;
                result.write(self.deserialize(deserializer));
                result as *mut dyn SerializeTestTrait
            }
        }

        impl TestTrait for Archived<Test> {
            fn get_id(&self) -> i32 {
                self.id
            }
        }

        let value: Box<dyn SerializeTestTrait> = Box::new(Test { id: 42 });

        let mut serializer = BufferSerializer::new(Aligned([0u8; BUFFER_SIZE]));
        let pos = serializer.serialize(&value).expect("failed to archive value");
        let buf = serializer.into_inner();
        let archived_value =
            unsafe { archived_value::<Box<dyn SerializeTestTrait>>(buf.as_ref(), pos) };
        assert_eq!(value.get_id(), archived_value.get_id());

        // exercise vtable cache
        assert_eq!(value.get_id(), archived_value.get_id());
        assert_eq!(value.get_id(), archived_value.get_id());

        let deserialized_value: Box<dyn SerializeTestTrait> = archived_value.deserialize(&mut GlobalAllocDeserializer);
        assert_eq!(value.get_id(), deserialized_value.get_id());
    }

    #[test]
    fn archive_dyn() {
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

        let mut serializer = BufferSerializer::new(Aligned([0u8; BUFFER_SIZE]));
        let pos = serializer.serialize(&value).expect("failed to archive value");
        let buf = serializer.into_inner();
        let archived_value = unsafe { archived_value::<Box<dyn STestTrait>>(buf.as_ref(), pos) };
        assert_eq!(value.get_id(), archived_value.get_id());

        // exercise vtable cache
        assert_eq!(value.get_id(), archived_value.get_id());
        assert_eq!(value.get_id(), archived_value.get_id());

        // deserialize
        let deserialized_value: Box<dyn STestTrait> = archived_value.deserialize(&mut GlobalAllocDeserializer);
        assert_eq!(value.get_id(), deserialized_value.get_id());
        assert_eq!(value.get_id(), deserialized_value.get_id());
    }

    #[test]
    fn archive_dyn_generic() {
        use rkyv_dyn::{register_impl, DynDeserializer, DynSerializer};

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

        impl<T: Archive + for<'a> Serialize<dyn DynSerializer + 'a> + core::fmt::Display + TypeName + 'static>
            rkyv_dyn::DeserializeDyn<dyn STestTrait<String>> for ArchivedTest<T>
        where
            ArchivedTest<T>: for<'a> Deserialize<Test<T>, (dyn DynDeserializer + 'a)> + rkyv_dyn::RegisteredImpl<dyn DTestTrait<String>>,
        {
            unsafe fn deserialize_dyn(
                &self,
                deserializer: &mut dyn DynDeserializer,
            ) -> *mut dyn STestTrait<String> {
                let result = deserializer.alloc(core::alloc::Layout::new::<Test<T>>()) as *mut Test<T>;
                result.write(self.deserialize(deserializer));
                result as *mut dyn STestTrait<String>
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

        let mut serializer = BufferSerializer::new(Aligned([0u8; BUFFER_SIZE]));
        let i32_pos = serializer.serialize(&i32_value).expect("failed to archive value");
        let string_pos = serializer
            .serialize(&string_value)
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

        let i32_deserialized_value: Box<dyn STestTrait<i32>> = i32_archived_value.deserialize(&mut GlobalAllocDeserializer);
        assert_eq!(i32_value.get_value(), i32_deserialized_value.get_value());
        assert_eq!(i32_value.get_value(), i32_deserialized_value.get_value());

        assert_eq!(string_value.get_value(), string_archived_value.get_value());
        assert_eq!(string_value.get_value(), string_archived_value.get_value());

        let string_deserialized_value: Box<dyn STestTrait<String>> =
            string_archived_value.deserialize(&mut GlobalAllocDeserializer);
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
        let mut serializer = BufferSerializer::new(Aligned([0u8; 256]));
        let pos = serializer.serialize(&42i32).unwrap();
        let mut buf = serializer.into_inner();
        let mut value = unsafe { archived_value_mut::<i32>(Pin::new(buf.as_mut()), pos) };
        assert_eq!(*value, 42);
        *value = 11;
        assert_eq!(*value, 11);
    }

    #[test]
    fn struct_mutable_refs() {
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

        let mut serializer = BufferSerializer::new(Aligned([0u8; 256]));
        let pos = serializer.serialize(&value).unwrap();
        let mut buf = serializer.into_inner();
        let mut value = unsafe { archived_value_mut::<Test>(Pin::new(buf.as_mut()), pos) };

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

        let mut serializer = BufferSerializer::new(Aligned([0u8; 256]));
        let pos = serializer.serialize(&value).unwrap();
        let mut buf = serializer.into_inner();
        let mut value = unsafe { archived_value_mut::<Test>(Pin::new(buf.as_mut()), pos) };

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

        let mut serializer = BufferSerializer::new(Aligned([0u8; 256]));
        let pos = serializer.serialize(&value).unwrap();
        let mut buf = serializer.into_inner();
        let mut value =
            unsafe { archived_value_mut::<Box<dyn SerializeTestTrait>>(Pin::new(buf.as_mut()), pos) };

        assert_eq!(value.value(), 10);
        value.as_mut().get_pin().set_value(64);
        assert_eq!(value.value(), 64);
    }

    #[test]
    fn recursive_structures() {
        #[derive(Archive, Serialize, PartialEq)]
        enum Node {
            Nil,
            Cons(#[recursive] Box<Node>),
        }

        // Right now Deserialize doesn't apply the right deserializer bounds from Box so we have to
        // implement it manually. Luckily it's not too bad, but hopefully in the future there's a
        // better way to do this.
        impl<D: AllocDeserializer + ?Sized> Deserialize<Node, D> for ArchivedNode {
            fn deserialize(&self, deserializer: &mut D) -> Node {
                match self {
                    ArchivedNode::Nil => Node::Nil,
                    ArchivedNode::Cons(inner) => Node::Cons(inner.deserialize(deserializer)),
                }
            }
        }

        impl PartialEq<Node> for Archived<Node> {
            fn eq(&self, other: &Node) -> bool {
                match self {
                    Archived::<Node>::Nil => match other {
                        Node::Nil => true,
                        _ => false,
                    },
                    Archived::<Node>::Cons(ar_inner) => match other {
                        Node::Cons(inner) => ar_inner == inner,
                        _ => false,
                    },
                }
            }
        }

        test_archive(&Node::Cons(Box::new(Node::Cons(Box::new(Node::Nil)))));
    }

    #[test]
    fn archive_root() {
        #[derive(Archive, Serialize)]
        struct Test {
            a: (),
            b: i32,
            c: String,
            d: Option<i32>,
        }

        impl PartialEq<Test> for Archived<Test> {
            fn eq(&self, other: &Test) -> bool {
                self.a == other.a && self.b == other.b && self.c == other.c && self.d == other.d
            }
        }

        let value = Test {
            a: (),
            b: 42,
            c: "hello world".to_string(),
            d: Some(42),
        };

        let mut serializer = BufferSerializer::new(Aligned([0u8; BUFFER_SIZE]));
        let pos = serializer
            .serialize_root(&value)
            .expect("failed to archive value");
        assert_eq!(pos, 0);
        let buf = serializer.into_inner();
        let archived_value = unsafe { archived_value::<Test>(buf.as_ref(), pos) };
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
        use rkyv::SharedSerializerAdapter;
        use std::rc::Rc;

        #[derive(Archive, Serialize, Eq, PartialEq)]
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

        impl PartialEq<Test> for Archived<Test> {
            fn eq(&self, other: &Test) -> bool {
                *self.a == *other.a && *self.b == *other.b
            }
        }

        let shared = Rc::new(10);
        let value = Test {
            a: shared.clone(),
            b: shared.clone(),
        };

        let mut serializer = SharedSerializerAdapter::new(BufferSerializer::new(Aligned([0u8; BUFFER_SIZE])));
        let pos = serializer.serialize(&value).expect("failed to archive value");
        let mut buf = serializer.into_inner().into_inner();

        let archived = unsafe { archived_value::<Test>(buf.as_ref(), pos) };
        assert!(archived == &value);

        let mut mutable_archived = unsafe { archived_value_mut::<Test>(Pin::new_unchecked(buf.as_mut()), pos) };
        *mutable_archived.as_mut().a().get_pin_unchecked() = 42;

        let archived = unsafe { archived_value::<Test>(buf.as_ref(), pos) };
        assert_eq!(*archived.a, 42);
        assert_eq!(*archived.b, 42);

        let mut mutable_archived = unsafe { archived_value_mut::<Test>(Pin::new_unchecked(buf.as_mut()), pos) };
        *mutable_archived.as_mut().b().get_pin_unchecked() = 17;

        let archived = unsafe { archived_value::<Test>(buf.as_ref(), pos) };
        assert_eq!(*archived.a, 17);
        assert_eq!(*archived.b, 17);

        // assert!(&archived.deserialize() == value);
    }
}
