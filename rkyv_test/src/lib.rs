#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use rkyv::{
        Aligned,
        Archive,
        ArchiveBuffer,
        Archived,
        archived_value,
        archived_ref,
        ArchiveRef,
        WriteExt,
    };
    use rkyv_dyn::{
        archive_dyn,
        register_vtable,
    };
    use rkyv_typename::TypeName;

    const BUFFER_SIZE: usize = 256;

    fn test_archive<T: Archive<Archived = U>, U: PartialEq<T>>(value: &T) {
        let mut writer = ArchiveBuffer::new(Aligned([0u8; BUFFER_SIZE]));
        let pos = writer.archive(value).expect("failed to archive value");
        let buf = writer.into_inner();
        let archived_value = unsafe { archived_value::<T>(buf.as_ref(), pos) };
        assert!(archived_value == value);
    }

    fn test_archive_ref<T: ArchiveRef<Archived = U> + ?Sized, U: PartialEq<T> + ?Sized>(value: &T) {
        let mut writer = ArchiveBuffer::new(Aligned([0u8; BUFFER_SIZE]));
        let pos = writer.archive_ref(value).expect("failed to archive ref");
        let buf = writer.into_inner();
        let archived_ref = unsafe { archived_ref::<T>(buf.as_ref(), pos) };
        assert!(&**archived_ref == value);
    }

    fn test_archive_container<T: Archive<Archived = U> + core::ops::Deref<Target = TV>, TV: ?Sized, U: core::ops::Deref<Target = TU>, TU: PartialEq<TV> + ?Sized>(value: &T) {
        let mut writer = ArchiveBuffer::new(Aligned([0u8; BUFFER_SIZE]));
        let pos = writer.archive(value).expect("failed to archive ref");
        let buf = writer.into_inner();
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
        test_archive(&(24, true, 16f32));
        test_archive(&[1, 2, 3, 4, 5, 6]);

        test_archive(&Option::<()>::None);
        test_archive(&Some(42));
    }

    #[test]
    fn archive_refs() {
        test_archive_ref::<[i32; 4], _>(&[1, 2, 3, 4]);
        test_archive_ref::<str, _>("hello world");
        test_archive_ref::<[i32], _>([1, 2, 3, 4].as_ref());
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

        let mut writer = ArchiveBuffer::new(Aligned([0u8; BUFFER_SIZE]));
        let pos = writer.archive(&hash_map).expect("failed to archive value");
        let buf = writer.into_inner();
        let archived_value = unsafe { archived_value::<HashMap<String, String>>(buf.as_ref(), pos) };

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
        #[derive(Archive, PartialEq)]
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
        #[derive(Archive)]
        struct Test((), i32, String, Option<i32>);

        impl PartialEq<Test> for Archived<Test> {
            fn eq(&self, other: &Test) -> bool {
                self.0 == other.0 && self.1 == other.1 && self.2 == other.2 && self.3 == other.3
            }
        }

        test_archive(&Test(
            (),
            42,
            "hello world".to_string(),
            Some(42),
        ));
    }

    #[test]
    fn archive_simple_struct() {
        #[derive(Archive)]
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
            }
        ]);
    }

    #[test]
    fn archive_generic_struct() {
        pub trait TestTrait {
            type Associated;
        }

        impl TestTrait for () {
            type Associated = i32;
        }

        #[derive(Archive)]
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
            }
        ]);
    }

    #[test]
    fn archive_enum() {
        #[derive(Archive)]
        enum Test {
            A,
            B(String),
            C {
                a: i32,
                b: String,
            }
        }

        impl PartialEq<Test> for Archived<Test> {
            fn eq(&self, other: &Test) -> bool {
                match self {
                    Self::A => if let Test::A = other { true } else { false },
                    Self::B(self_value) => if let Test::B(other_value) = other { self_value == other_value } else { false },
                    Self::C { a, b } => if let Test::C { a: _a, b: _b } = other { a == _a && b == _b } else { false },
                }
            }
        }

        test_archive(&Test::A);
        test_archive(&Test::B("hello_world".to_string()));
        test_archive(&Test::C { a: 42, b: "hello world".to_string() });
        test_archive(&vec![
            Test::A,
            Test::B("hello world".to_string()),
            Test::C { a: 42, b: "hello world".to_string() },
        ]);
    }

    #[test]
    fn archive_generic_enum() {
        pub trait TestTrait {
            type Associated;
        }

        impl TestTrait for () {
            type Associated = i32;
        }

        #[derive(Archive)]
        enum Test<T: TestTrait> {
            A,
            B(String),
            C {
                a: <T as TestTrait>::Associated,
                b: String,
            }
        }

        impl<T: TestTrait> PartialEq<Test<T>> for Archived<Test<T>>
        where
            <T as TestTrait>::Associated: Archive,
            Archived<<T as TestTrait>::Associated>: PartialEq<<T as TestTrait>::Associated>,
        {
            fn eq(&self, other: &Test<T>) -> bool {
                match self {
                    Self::A => if let Test::A = other { true } else { false },
                    Self::B(self_value) => if let Test::B(other_value) = other { self_value == other_value } else { false },
                    Self::C { a, b } => if let Test::C { a: _a, b: _b } = other { a == _a && b == _b } else { false },
                }
            }
        }

        test_archive(&Test::<()>::A);
        test_archive(&Test::<()>::B("hello_world".to_string()));
        test_archive(&Test::<()>::C { a: 42, b: "hello world".to_string() });
        test_archive(&vec![
            Test::<()>::A,
            Test::<()>::B("hello world".to_string()),
            Test::<()>::C { a: 42, b: "hello world".to_string() },
        ]);
    }

    #[test]
    fn archive_self() {
        #[derive(Archive, Clone, Copy, PartialEq)]
        #[archive(self)]
        struct TestUnit;

        test_archive(&TestUnit);

        #[derive(Archive, Clone, Copy, PartialEq)]
        #[archive(self)]
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

        #[derive(Archive, Clone, Copy, PartialEq)]
        #[archive(self)]
        struct TestTuple((), i32, bool, f32, TestUnit);

        test_archive(&TestTuple((), 42, true, 3.14f32, TestUnit));

        #[derive(Archive, Clone, Copy, PartialEq)]
        #[repr(u8)]
        #[archive(self)]
        enum TestEnum {
            A((), i32, bool, f32, TestUnit),
        }

        test_archive(&TestEnum::A((), 42, true, 3.14f32, TestUnit));

        #[derive(Archive, Clone, Copy, PartialEq)]
        #[archive(self)]
        struct TestGeneric<T>(T);

        test_archive(&TestGeneric(42));
    }

    #[test]
    fn archive_derives() {
        #[derive(Archive, Clone)]
        #[archive(derive(Clone, Debug, PartialEq))]
        struct Test(i32);

        let value = Test(42);

        let mut writer = ArchiveBuffer::new(Aligned([0u8; BUFFER_SIZE]));
        let pos = writer.archive(&value).expect("failed to archive value");
        let buf = writer.into_inner();
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
    fn archive_dyn() {
        #[archive_dyn]
        pub trait TestTrait {
            fn get_id(&self) -> i32;
        }

        #[derive(Archive, TypeName)]
        pub struct Test {
            id: i32,
        }

        #[archive_dyn]
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

        let value: Box<dyn ArchiveTestTrait> = Box::new(Test { id: 42 });

        let mut writer = ArchiveBuffer::new(Aligned([0u8; BUFFER_SIZE]));
        let pos = writer.archive(&value).expect("failed to archive value");
        let buf = writer.into_inner();
        let archived_value = unsafe { archived_value::<Box<dyn ArchiveTestTrait>>(buf.as_ref(), pos) };
        assert_eq!(value.get_id(), archived_value.get_id());

        // exercise vtable cache
        assert_eq!(value.get_id(), archived_value.get_id());
        assert_eq!(value.get_id(), archived_value.get_id());
    }

    #[test]
    fn archive_dyn_generic() {
        #[archive_dyn(trait = "ArchiveableTestTrait")]
        pub trait TestTrait<T> {
            fn get_value(&self) -> T;
        }

        #[derive(Archive, TypeName)]
        #[archive(archived = "ArchivedTest")]
        pub struct Test<T> {
            value: T,
        }

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

        impl<T: Archive> TestTrait<String> for ArchivedTest<T>
        where
            Archived<T>: core::fmt::Display,
        {
            fn get_value(&self) -> String {
                format!("{}", self.value)
            }
        }

        register_vtable!(Test<i32> as dyn TestTrait<i32>);
        register_vtable!(Test<String> as dyn TestTrait<String>);

        let i32_value: Box<dyn ArchiveableTestTrait<i32>> = Box::new(Test { value: 42 });
        let string_value: Box<dyn ArchiveableTestTrait<String>> = Box::new(Test { value: "hello world".to_string() });

        let mut writer = ArchiveBuffer::new(Aligned([0u8; BUFFER_SIZE]));
        let i32_pos = writer.archive(&i32_value).expect("failed to archive value");
        let string_pos = writer.archive(&string_value).expect("failed to archive value");
        let buf = writer.into_inner();
        let i32_archived_value = unsafe { archived_value::<Box<dyn ArchiveableTestTrait<i32>>>(buf.as_ref(), i32_pos) };
        let string_archived_value = unsafe { archived_value::<Box<dyn ArchiveableTestTrait<String>>>(buf.as_ref(), string_pos) };
        assert_eq!(i32_value.get_value(), i32_archived_value.get_value());
        assert_eq!(string_value.get_value(), string_archived_value.get_value());

        // exercise vtable cache
        assert_eq!(i32_value.get_value(), i32_archived_value.get_value());
        assert_eq!(i32_value.get_value(), i32_archived_value.get_value());

        assert_eq!(string_value.get_value(), string_archived_value.get_value());
        assert_eq!(string_value.get_value(), string_archived_value.get_value());
    }

    #[test]
    fn derive_visibility() {
        mod inner {
            #[derive(super::Archive)]
            #[archive(archived = "ArchivedTestTuple")]
            pub struct TestTuple(pub i32);

            #[derive(super::Archive)]
            #[archive(archived = "ArchivedTestStruct")]
            pub struct TestStruct {
                pub value: i32,
            }

            #[derive(super::Archive)]
            #[archive(archived = "ArchivedTestEnum")]
            pub enum TestEnum {
                B(i32),
                C { value: i32 },
            }
        }

        use inner::{
            ArchivedTestEnum,
            ArchivedTestStruct,
            ArchivedTestTuple,
            TestEnum,
            TestStruct,
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
}