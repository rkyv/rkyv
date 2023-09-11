#[cfg(test)]
mod tests {
    use crate::util::alloc::*;
    use core::pin::Pin;
    use rkyv::{
        archived_root, archived_root_mut,
        ser::{
            serializers::{AlignedSerializer, BufferSerializer},
            Serializer,
        },
        AlignedBytes, AlignedVec, Archive, Archived, Deserialize, Fallible,
        Infallible, Serialize,
    };

    #[cfg(not(feature = "std"))]
    use alloc::{
        borrow::Cow,
        boxed::Box,
        collections::{BTreeMap, BTreeSet},
        rc::{Rc, Weak},
        string::{String, ToString},
        vec,
        vec::Vec,
    };
    #[cfg(feature = "std")]
    use std::{
        borrow::Cow,
        collections::{BTreeMap, BTreeSet},
        rc::{Rc, Weak},
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
    fn archive_option() {
        test_archive(&Some(Box::new(42)));
        test_archive(&Some("hello world".to_string().into_boxed_str()));
        test_archive(&Some(vec![1, 2, 3, 4].into_boxed_slice()));
        test_archive(&Some("hello world".to_string()));
        test_archive(&Some(vec![1, 2, 3, 4]));
        test_archive(&Some(Box::new(vec![1, 2, 3, 4])));
    }

    #[test]
    fn option_is_copy() {
        #[derive(
            Clone, Copy, Debug, PartialEq, Archive, Serialize, Deserialize,
        )]
        #[archive(compare(PartialEq))]
        #[archive_attr(derive(Clone, Copy, Debug))]
        enum ExampleEnum {
            Foo,
            Bar(u64),
        }

        #[derive(
            Clone, Copy, Debug, PartialEq, Archive, Serialize, Deserialize,
        )]
        #[archive(compare(PartialEq))]
        #[archive_attr(derive(Clone, Copy, Debug))]
        struct Example {
            x: i32,
            y: Option<ExampleEnum>,
        }

        let _ = Example {
            x: 0,
            y: Some(ExampleEnum::Bar(0)),
        };
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn archive_result() {
        test_archive::<Result<_, ()>>(&Ok(Box::new(42)));
        test_archive::<Result<_, ()>>(&Ok("hello world"
            .to_string()
            .into_boxed_str()));
        test_archive::<Result<_, ()>>(&Ok(vec![1, 2, 3, 4].into_boxed_slice()));
        test_archive::<Result<_, ()>>(&Ok("hello world".to_string()));
        test_archive::<Result<_, ()>>(&Ok(vec![1, 2, 3, 4]));
        test_archive::<Result<_, ()>>(&Ok(Box::new(vec![1, 2, 3, 4])));
        test_archive::<Result<(), _>>(&Err(Box::new(42)));
        test_archive::<Result<(), _>>(&Err("hello world"
            .to_string()
            .into_boxed_str()));
        test_archive::<Result<(), _>>(
            &Err(vec![1, 2, 3, 4].into_boxed_slice()),
        );
        test_archive::<Result<(), _>>(&Err("hello world".to_string()));
        test_archive::<Result<(), _>>(&Err(vec![1, 2, 3, 4]));
        test_archive::<Result<(), _>>(&Err(Box::new(vec![1, 2, 3, 4])));
    }

    #[cfg(all(feature = "std", feature = "validation"))]
    mod isolate {
        #[cfg(feature = "wasm")]
        use wasm_bindgen_test::*;

        #[test]
        #[allow(unused_variables)]
        #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
        fn archive_example() {
            use rkyv::{Archive, Deserialize, Serialize};

            #[derive(Archive, Deserialize, Serialize, Debug, PartialEq)]
            #[archive(
                // This will generate a PartialEq impl between our unarchived
                // and archived types:
                compare(PartialEq),
                // bytecheck can be used to validate your data if you want. To
                // use the safe API, you have to derive CheckBytes for the
                // archived type:
                check_bytes,
            )]
            // Derives can be passed through to the generated type:
            #[archive_attr(derive(Debug))]
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

            // Serializing is as easy as a single function call
            let bytes = rkyv::to_bytes::<_, 256>(&value).unwrap();

            // Or you can customize your serialization for better performance
            // and compatibility with #![no_std] environments
            use rkyv::ser::{serializers::AllocSerializer, Serializer};

            let mut serializer = AllocSerializer::<0>::default();
            serializer.serialize_value(&value).unwrap();
            let bytes = serializer.into_serializer().into_inner();

            // You can use the safe API for fast zero-copy deserialization
            let archived =
                rkyv::check_archived_root::<Test>(&bytes[..]).unwrap();
            assert_eq!(archived, &value);

            // Or you can use the unsafe API for maximum performance
            let archived = unsafe { rkyv::archived_root::<Test>(&bytes[..]) };
            assert_eq!(archived, &value);

            // And you can always deserialize back to the original type
            let deserialized: Test =
                archived.deserialize(&mut rkyv::Infallible).unwrap();
            assert_eq!(deserialized, value);
        }
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn archive_unit_struct() {
        #[derive(Archive, Serialize, Deserialize, Debug, PartialEq)]
        #[archive(compare(PartialEq))]
        #[archive_attr(derive(Debug))]
        struct Test;

        test_archive(&Test);
        test_archive(&vec![Test, Test]);
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn archive_tuple_struct() {
        #[derive(Archive, Serialize, Deserialize, Debug, PartialEq)]
        #[archive(compare(PartialEq))]
        #[archive_attr(derive(Debug))]
        struct Test((), i32, String, Option<i32>);

        test_archive(&Test((), 42, "hello world".to_string(), Some(42)));
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn archive_simple_struct() {
        #[derive(Archive, Serialize, Deserialize, Debug, PartialEq)]
        #[archive(compare(PartialEq))]
        #[archive_attr(derive(Debug))]
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
        use core::fmt;

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

        impl<T: TestTrait> fmt::Debug for Test<T>
        where
            T::Associated: fmt::Debug,
        {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_struct("Test")
                    .field("a", &self.a)
                    .field("b", &self.b)
                    .field("c", &self.c)
                    .field("d", &self.d)
                    .finish()
            }
        }

        impl<T: TestTrait> fmt::Debug for ArchivedTest<T>
        where
            T::Associated: Archive,
            <T::Associated as Archive>::Archived: fmt::Debug,
        {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_struct("Test")
                    .field("a", &self.a)
                    .field("b", &self.b)
                    .field("c", &self.c)
                    .field("d", &self.d)
                    .finish()
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
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn archive_enum() {
        #[derive(Archive, Serialize, Deserialize, Debug, PartialEq)]
        #[archive(compare(PartialEq))]
        #[archive_attr(derive(Debug))]
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
        use core::fmt;

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

        impl<T: TestTrait> fmt::Debug for Test<T>
        where
            T::Associated: fmt::Debug,
        {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                match self {
                    Test::A => f.debug_tuple("Test::A").finish(),
                    Test::B(value) => {
                        f.debug_tuple("Test::B").field(value).finish()
                    }
                    Test::C { a, b } => f
                        .debug_struct("Test::C")
                        .field("a", a)
                        .field("b", b)
                        .finish(),
                }
            }
        }

        impl<T: TestTrait> fmt::Debug for ArchivedTest<T>
        where
            T::Associated: Archive,
            <T::Associated as Archive>::Archived: fmt::Debug,
        {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                match self {
                    ArchivedTest::A => {
                        f.debug_tuple("ArchivedTest::A").finish()
                    }
                    ArchivedTest::B(value) => {
                        f.debug_tuple("ArchivedTest::B").field(value).finish()
                    }
                    ArchivedTest::C { a, b } => f
                        .debug_struct("ArchivedTest::C")
                        .field("a", a)
                        .field("b", b)
                        .finish(),
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
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    #[cfg(feature = "copy")]
    fn archive_copy() {
        use core::fmt;

        #[derive(Archive, Serialize, Deserialize, Debug, PartialEq)]
        #[archive(copy_safe, compare(PartialEq))]
        #[archive_attr(derive(Debug))]
        struct TestUnit;

        test_archive(&TestUnit);

        #[derive(Archive, Serialize, Deserialize, Debug, PartialEq)]
        // This is not technically copy safe but we're here to test
        #[archive(copy_safe, compare(PartialEq))]
        #[archive_attr(derive(Debug))]
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

        #[derive(Archive, Serialize, Deserialize, Debug, PartialEq)]
        // This is not technically copy safe but we're here to test
        #[archive(copy_safe, compare(PartialEq))]
        #[archive_attr(derive(Debug))]
        struct TestTuple((), i32, bool, f32, TestUnit);

        test_archive(&TestTuple((), 42, true, 3.14f32, TestUnit));

        #[derive(Archive, Serialize, Deserialize, Debug, PartialEq)]
        // This is not technically copy safe but we're here to test
        #[archive(copy_safe, compare(PartialEq))]
        #[archive_attr(derive(Debug))]
        #[repr(u8)]
        enum TestEnum {
            A((), i32, bool, f32, TestUnit),
        }

        test_archive(&TestEnum::A((), 42, true, 3.14f32, TestUnit));

        #[derive(Archive, Serialize, Deserialize, Debug, PartialEq)]
        // This is not technically copy safe but we're here to test
        #[archive(copy_safe, compare(PartialEq))]
        struct TestGeneric<T>(T);

        impl<T: Archive> fmt::Debug for ArchivedTestGeneric<T>
        where
            T::Archived: fmt::Debug,
        {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_tuple("TestGeneric").field(&self.0).finish()
            }
        }

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
            ArchivedTestEnum, ArchivedTestStruct, ArchivedTestTuple, TestEnum,
            TestStruct, TestTuple,
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
        let mut value =
            unsafe { archived_root_mut::<i32>(Pin::new(buf.as_mut())) };
        assert_eq!(*value, 42);
        *value = 11.into();
        assert_eq!(*value, 11);
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn struct_mutable_refs() {
        #[derive(Archive, Serialize)]
        struct Test {
            a: Box<i32>,
            b: Vec<String>,
        }

        impl ArchivedTest {
            fn a(self: Pin<&mut Self>) -> Pin<&mut Archived<Box<i32>>> {
                unsafe { self.map_unchecked_mut(|s| &mut s.a) }
            }

            fn b(self: Pin<&mut Self>) -> Pin<&mut Archived<Vec<String>>> {
                unsafe { self.map_unchecked_mut(|s| &mut s.b) }
            }
        }

        let value = Test {
            a: Box::new(10),
            b: vec!["hello".to_string(), "world".to_string()],
        };

        let mut serializer = DefaultSerializer::default();
        serializer.serialize_value(&value).unwrap();
        let mut buf = serializer.into_serializer().into_inner();
        let mut value =
            unsafe { archived_root_mut::<Test>(Pin::new(buf.as_mut())) };

        assert_eq!(*value.a, 10);
        assert_eq!(value.b.len(), 2);
        assert_eq!(value.b[0], "hello");
        assert_eq!(value.b[1], "world");

        *value.as_mut().a().get_pin_mut() = 50.into();
        assert_eq!(*value.a, 50);

        value
            .as_mut()
            .b()
            .index_pin(0)
            .pin_mut_str()
            .make_ascii_uppercase();
        value
            .as_mut()
            .b()
            .index_pin(1)
            .pin_mut_str()
            .make_ascii_uppercase();
        assert_eq!(value.b[0], "HELLO");
        assert_eq!(value.b[1], "WORLD");
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
        let mut value =
            unsafe { archived_root_mut::<Test>(Pin::new(buf.as_mut())) };

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
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn recursive_structures() {
        #[derive(Archive, Serialize, Deserialize, Debug, PartialEq)]
        #[archive(compare(PartialEq))]
        #[archive_attr(derive(Debug))]
        // The derive macros don't apply the right bounds from Box so we have to manually specify
        // what bounds to apply
        #[archive(bound(serialize = "__S: Serializer"))]
        enum Node {
            Nil,
            Cons(#[omit_bounds] Box<Node>),
        }

        test_archive(&Node::Cons(Box::new(Node::Cons(Box::new(Node::Nil)))));
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn recursive_self_types() {
        #[derive(Archive, Serialize, Deserialize, Debug, PartialEq)]
        #[archive(compare(PartialEq))]
        #[archive_attr(derive(Debug))]
        // The derive macros don't apply the right bounds from Box so we have to manually specify
        // what bounds to apply
        #[archive(bound(serialize = "__S: Serializer"))]
        pub enum LinkedList<T: Archive>
        where
            T::Archived: core::fmt::Debug,
        {
            Empty,
            Node {
                val: T,
                #[omit_bounds]
                next: Box<Self>,
            },
        }

        test_archive(&LinkedList::Node {
            val: 42i32,
            next: Box::new(LinkedList::Node {
                val: 100i32,
                next: Box::new(LinkedList::Empty),
            }),
        });
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn complex_bounds() {
        use core::marker::PhantomData;

        trait MyTrait {}

        impl MyTrait for i32 {}

        struct MyStruct<T> {
            _phantom: PhantomData<T>,
        }

        impl<T: Archive + MyTrait> Archive for MyStruct<T> {
            type Archived = MyStruct<T::Archived>;
            type Resolver = ();

            unsafe fn resolve(
                &self,
                _: usize,
                _: Self::Resolver,
                _: *mut Self::Archived,
            ) {
            }
        }

        impl<T: Archive + MyTrait, S: Fallible + MyTrait + ?Sized> Serialize<S>
            for MyStruct<T>
        {
            fn serialize(&self, _: &mut S) -> Result<Self::Resolver, S::Error> {
                Ok(())
            }
        }

        impl<T, D> Deserialize<MyStruct<T>, D> for Archived<MyStruct<T>>
        where
            T: Archive + MyTrait,
            D: Fallible + MyTrait + ?Sized,
        {
            fn deserialize(&self, _: &mut D) -> Result<MyStruct<T>, D::Error> {
                Ok(MyStruct {
                    _phantom: PhantomData,
                })
            }
        }

        #[derive(Archive, Serialize, Deserialize)]
        #[archive(bound(archive = "T: MyTrait"))]
        #[archive(bound(serialize = "__S: MyTrait"))]
        #[archive(bound(deserialize = "__D: MyTrait"))]
        enum Node<T> {
            Nil,
            Cons {
                value: T,
                #[omit_bounds]
                next: MyStruct<Self>,
            },
        }

        impl<T: MyTrait> MyTrait for Node<T> {}
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn archive_more_std() {
        use core::{
            num::NonZeroU8,
            ops::{
                Range, RangeFrom, RangeFull, RangeInclusive, RangeTo,
                RangeToInclusive,
            },
        };

        #[derive(Archive, Serialize, Deserialize, Debug, PartialEq)]
        #[archive(compare(PartialEq))]
        #[archive_attr(derive(Debug))]
        struct Test {
            a: NonZeroU8,
            b: RangeFull,
            c: Range<i32>,
            d: RangeInclusive<i32>,
            e: RangeFrom<i32>,
            f: RangeTo<i32>,
            g: RangeToInclusive<i32>,
        }

        let value = Test {
            a: NonZeroU8::new(8).unwrap(),
            b: RangeFull,
            c: Range { start: 14, end: 46 },
            d: RangeInclusive::new(12, 22),
            e: RangeFrom { start: 60 },
            f: RangeTo { end: 35 },
            g: RangeToInclusive { end: 87 },
        };

        test_archive(&value);
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn archive_shared_ptr() {
        #[derive(Debug, Eq, PartialEq, Archive, Deserialize, Serialize)]
        #[archive(compare(PartialEq))]
        #[archive_attr(derive(Debug))]
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

        let mut serializer = DefaultSerializer::default();
        serializer.serialize_value(&value).unwrap();
        let mut buf = serializer.into_serializer().into_inner();

        let archived = unsafe { archived_root::<Test>(buf.as_ref()) };
        assert_eq!(archived, &value);

        let mut mutable_archived = unsafe {
            archived_root_mut::<Test>(Pin::new_unchecked(buf.as_mut()))
        };
        unsafe {
            *mutable_archived.as_mut().a().get_pin_mut_unchecked() =
                42u32.into();
        }

        let archived = unsafe { archived_root::<Test>(buf.as_ref()) };
        assert_eq!(*archived.a, 42);
        assert_eq!(*archived.b, 42);

        let mut mutable_archived = unsafe {
            archived_root_mut::<Test>(Pin::new_unchecked(buf.as_mut()))
        };
        unsafe {
            *mutable_archived.as_mut().b().get_pin_mut_unchecked() =
                17u32.into();
        }

        let archived = unsafe { archived_root::<Test>(buf.as_ref()) };
        assert_eq!(*archived.a, 17);
        assert_eq!(*archived.b, 17);

        let mut deserializer = DefaultDeserializer::default();
        let deserialized: Test =
            archived.deserialize(&mut deserializer).unwrap();

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
        #[derive(Archive, Serialize, Deserialize, Debug, PartialEq)]
        #[archive(compare(PartialEq))]
        #[archive_attr(derive(Debug))]
        struct Test {
            a: Rc<[String]>,
            b: Rc<[String]>,
        }

        let rc_slice = Rc::<[String]>::from(
            vec!["hello".to_string(), "world".to_string()].into_boxed_slice(),
        );
        let value = Test {
            a: rc_slice.clone(),
            b: rc_slice,
        };

        test_archive(&value);
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn archive_unsized_shared_ptr_empty() {
        #[derive(Archive, Serialize, Deserialize, Debug, PartialEq)]
        #[archive(compare(PartialEq))]
        #[archive_attr(derive(Debug))]
        struct Test {
            a: Rc<[u32]>,
            b: Rc<[u32]>,
        }

        let a_rc_slice = Rc::<[u32]>::from(vec![].into_boxed_slice());
        let b_rc_slice = Rc::<[u32]>::from(vec![100].into_boxed_slice());
        let value = Test {
            a: a_rc_slice,
            b: b_rc_slice.clone(),
        };

        test_archive(&value);
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn archive_weak_ptr() {
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

        let mut serializer = DefaultSerializer::default();
        serializer.serialize_value(&value).unwrap();
        let mut buf = serializer.into_serializer().into_inner();

        let archived = unsafe { archived_root::<Test>(buf.as_ref()) };
        assert_eq!(*archived.a, 10);
        assert!(archived.b.upgrade().is_some());
        assert_eq!(**archived.b.upgrade().unwrap(), 10);

        let mut mutable_archived = unsafe {
            archived_root_mut::<Test>(Pin::new_unchecked(buf.as_mut()))
        };
        unsafe {
            *mutable_archived.as_mut().a().get_pin_mut_unchecked() =
                42u32.into();
        }

        let archived = unsafe { archived_root::<Test>(buf.as_ref()) };
        assert_eq!(*archived.a, 42);
        assert!(archived.b.upgrade().is_some());
        assert_eq!(**archived.b.upgrade().unwrap(), 42);

        let mut mutable_archived = unsafe {
            archived_root_mut::<Test>(Pin::new_unchecked(buf.as_mut()))
        };
        unsafe {
            *mutable_archived
                .as_mut()
                .b()
                .upgrade_pin_mut()
                .unwrap()
                .get_pin_mut_unchecked() = 17u32.into();
        }

        let archived = unsafe { archived_root::<Test>(buf.as_ref()) };
        assert_eq!(*archived.a, 17);
        assert!(archived.b.upgrade().is_some());
        assert_eq!(**archived.b.upgrade().unwrap(), 17);

        let mut deserializer = DefaultDeserializer::default();
        let deserialized: Test =
            archived.deserialize(&mut deserializer).unwrap();

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
        #[derive(Archive, Debug, PartialEq)]
        #[archive(archived = "ATest", resolver = "RTest", compare(PartialEq))]
        #[archive_attr(derive(Debug))]
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
            fn deserialize(
                &self,
                deserializer: &mut D,
            ) -> Result<Test, D::Error> {
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
        fn check<T: Serializer>() {}

        check::<BufferSerializer<[u8; 256]>>();
        check::<BufferSerializer<&mut [u8; 256]>>();
        check::<BufferSerializer<&mut [u8]>>();
        check::<BufferSerializer<AlignedBytes<256>>>();
        check::<BufferSerializer<&mut AlignedBytes<256>>>();
        check::<AlignedSerializer<AlignedVec>>();
        check::<AlignedSerializer<&mut AlignedVec>>();
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn buffer_serializer_zeroes_padding() {
        use core::mem::size_of;

        #[derive(Archive, Serialize)]
        pub struct PaddedExample {
            a: u8,
            b: u64,
        }
        let mut serializer = BufferSerializer::<[u8; 256]>::new([0xccu8; 256]);
        serializer
            .serialize_value(&PaddedExample { a: 0u8, b: 0u64 })
            .unwrap();
        let bytes = serializer.into_inner();
        assert!(bytes[0..size_of::<Archived<PaddedExample>>()]
            .iter()
            .all(|&b| b == 0));
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn const_generics() {
        #[derive(Archive, Deserialize, Serialize, Debug, PartialEq)]
        #[archive(compare(PartialEq))]
        #[archive_attr(derive(Debug))]
        pub struct Const<const N: usize>;

        test_archive(&Const::<1>);
        test_archive(&Const::<2>);
        test_archive(&Const::<3>);

        #[derive(Archive, Deserialize, Serialize)]
        pub struct Array<T, const N: usize>([T; N]);
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn arbitrary_enum_discriminant() {
        use rkyv::Infallible;

        #[derive(Archive, Deserialize, Serialize, Debug, PartialEq)]
        #[archive_attr(derive(Debug, PartialEq))]
        #[rustfmt::skip]
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

        #[cfg(any(
            feature = "native_endian",
            all(target_endian = "little", feature = "little_endian"),
            all(target_endian = "big", feature = "big_endian")
        ))]
        assert_eq!(ArchivedReallyBigEnum::V100 as u16, 0x100u16);
        #[cfg(any(
            all(target_endian = "little", feature = "big_endian"),
            all(target_endian = "big", feature = "little_endian")
        ))]
        assert_eq!(ArchivedReallyBigEnum::V100 as u16, 0x1u16);

        let mut serializer = DefaultSerializer::default();
        serializer.serialize_value(&ReallyBigEnum::V100).unwrap();
        let buf = serializer.into_serializer().into_inner();

        let archived = unsafe { archived_root::<ReallyBigEnum>(buf.as_ref()) };
        assert_eq!(archived, &ArchivedReallyBigEnum::V100);

        let deserialized: ReallyBigEnum =
            archived.deserialize(&mut Infallible).unwrap();
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
        #[archive_attr(repr(transparent))]
        pub struct Test {
            a: u32,
        }

        assert_eq!(core::mem::size_of::<ArchivedTest>(), 4);
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn repr_c() {
        #[derive(Archive)]
        #[archive_attr(repr(C))]
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
        #[archive_attr(repr(u16))]
        #[allow(dead_code)]
        pub enum ExplicitRepr {
            V0,
            V1,
        }

        assert_eq!(core::mem::size_of::<ArchivedExplicitRepr>(), 2);
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn repr_c_packed() {
        #[derive(Archive)]
        #[archive_attr(repr(C, packed))]
        struct CPackedRepr {
            a: u8,
            b: u32,
            c: u8,
        }

        assert_eq!(core::mem::size_of::<ArchivedCPackedRepr>(), 6);

        #[derive(Archive)]
        #[archive_attr(repr(C))]
        #[archive_attr(repr(packed))]
        struct CPackedRepr2 {
            a: u8,
            b: u32,
            c: u8,
        }

        assert_eq!(core::mem::size_of::<ArchivedCPackedRepr2>(), 6);
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn repr_c_align() {
        #[derive(Archive)]
        #[archive_attr(repr(C, align(8)))]
        struct CAlignRepr {
            a: u8,
        }

        assert_eq!(core::mem::align_of::<ArchivedCAlignRepr>(), 8);

        #[derive(Archive)]
        #[archive_attr(repr(C))]
        #[archive_attr(repr(align(8)))]
        struct CAlignRepr2 {
            a: u8,
        }

        assert_eq!(core::mem::align_of::<ArchivedCAlignRepr>(), 8);
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn archive_as() {
        // Struct

        #[derive(Archive, Serialize, Deserialize, Debug)]
        #[archive(as = "ExampleStruct<T::Archived>")]
        #[repr(transparent)]
        struct ExampleStruct<T> {
            value: T,
        }

        impl<T: PartialEq<U>, U> PartialEq<ExampleStruct<U>> for ExampleStruct<T> {
            fn eq(&self, other: &ExampleStruct<U>) -> bool {
                self.value.eq(&other.value)
            }
        }

        let value = ExampleStruct {
            value: "hello world".to_string(),
        };

        test_archive(&value);

        // Tuple struct

        #[derive(Archive, Serialize, Deserialize, Debug)]
        #[archive(as = "ExampleTupleStruct<T::Archived>")]
        #[repr(transparent)]
        struct ExampleTupleStruct<T>(T);

        impl<T: PartialEq<U>, U> PartialEq<ExampleTupleStruct<U>>
            for ExampleTupleStruct<T>
        {
            fn eq(&self, other: &ExampleTupleStruct<U>) -> bool {
                self.0.eq(&other.0)
            }
        }

        let value = ExampleTupleStruct("hello world".to_string());

        test_archive(&value);

        // Unit struct

        #[derive(Archive, Serialize, Deserialize, Debug, PartialEq)]
        #[archive(as = "ExampleUnitStruct")]
        struct ExampleUnitStruct;

        test_archive(&ExampleUnitStruct);

        // Enum

        #[derive(Archive, Serialize, Deserialize, Debug)]
        #[archive(as = "ExampleEnum<T::Archived>")]
        #[repr(u8)]
        enum ExampleEnum<T> {
            A(T),
            B,
        }

        impl<T: PartialEq<U>, U> PartialEq<ExampleEnum<U>> for ExampleEnum<T> {
            fn eq(&self, other: &ExampleEnum<U>) -> bool {
                match self {
                    ExampleEnum::A(value) => {
                        if let ExampleEnum::A(other) = other {
                            value.eq(other)
                        } else {
                            false
                        }
                    }
                    ExampleEnum::B => {
                        if let ExampleEnum::B = other {
                            true
                        } else {
                            false
                        }
                    }
                }
            }
        }

        let value = ExampleEnum::A("hello world".to_string());

        test_archive(&value);
    }

    mod with {
        #[cfg(not(feature = "std"))]
        use alloc::string::{String, ToString};
        use core::str::FromStr;
        use rkyv::{
            archived_root,
            ser::serializers::AlignedSerializer,
            ser::Serializer,
            with::{ArchiveWith, DeserializeWith, SerializeWith},
            AlignedVec, Archive, Archived, Deserialize, Fallible, Infallible,
            Serialize,
        };

        #[cfg(feature = "wasm")]
        use wasm_bindgen_test::*;

        struct ConvertToString;

        impl<T: ToString> ArchiveWith<T> for ConvertToString {
            type Archived = <String as Archive>::Archived;
            type Resolver = <String as Archive>::Resolver;

            unsafe fn resolve_with(
                value: &T,
                pos: usize,
                resolver: Self::Resolver,
                out: *mut Self::Archived,
            ) {
                value.to_string().resolve(pos, resolver, out);
            }
        }

        impl<T: ToString, S: Serializer + ?Sized> SerializeWith<T, S>
            for ConvertToString
        {
            fn serialize_with(
                value: &T,
                serializer: &mut S,
            ) -> Result<Self::Resolver, S::Error> {
                Ok(value.to_string().serialize(serializer)?)
            }
        }

        impl<T: FromStr, D: Fallible + ?Sized>
            DeserializeWith<Archived<String>, T, D> for ConvertToString
        where
            <T as FromStr>::Err: core::fmt::Debug,
        {
            fn deserialize_with(
                value: &Archived<String>,
                _: &mut D,
            ) -> Result<T, D::Error> {
                Ok(T::from_str(value.as_str()).unwrap())
            }
        }

        #[test]
        #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
        fn with_struct() {
            #[derive(Archive, Serialize, Deserialize)]
            struct Test {
                #[with(ConvertToString)]
                value: i32,
                other: i32,
            }

            let value = Test {
                value: 10,
                other: 10,
            };
            let mut serializer = AlignedSerializer::new(AlignedVec::new());
            serializer.serialize_value(&value).unwrap();
            let result = serializer.into_inner();
            let archived = unsafe { archived_root::<Test>(result.as_slice()) };

            assert_eq!(archived.value, "10");
            assert_eq!(archived.other, 10);

            let deserialized: Test =
                archived.deserialize(&mut Infallible).unwrap();
            assert_eq!(deserialized.value, 10);
            assert_eq!(deserialized.other, 10);
        }

        #[test]
        #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
        fn with_tuple_struct() {
            #[derive(Archive, Serialize, Deserialize)]
            struct Test(#[with(ConvertToString)] i32, i32);

            let value = Test(10, 10);
            let mut serializer = AlignedSerializer::new(AlignedVec::new());
            serializer.serialize_value(&value).unwrap();
            let result = serializer.into_inner();
            let archived = unsafe { archived_root::<Test>(result.as_slice()) };

            assert_eq!(archived.0, "10");
            assert_eq!(archived.1, 10);

            let deserialized: Test =
                archived.deserialize(&mut Infallible).unwrap();
            assert_eq!(deserialized.0, 10);
            assert_eq!(deserialized.1, 10);
        }

        #[test]
        #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
        fn with_enum() {
            #[derive(Archive, Serialize, Deserialize)]
            enum Test {
                A {
                    #[with(ConvertToString)]
                    value: i32,
                    other: i32,
                },
                B(#[with(ConvertToString)] i32, i32),
            }

            let value = Test::A {
                value: 10,
                other: 10,
            };
            let mut serializer = AlignedSerializer::new(AlignedVec::new());
            serializer.serialize_value(&value).unwrap();
            let result = serializer.into_inner();
            let archived = unsafe { archived_root::<Test>(result.as_slice()) };

            if let ArchivedTest::A { value, other } = archived {
                assert_eq!(*value, "10");
                assert_eq!(*other, 10);
            } else {
                panic!("expected variant A");
            };

            let deserialized: Test =
                archived.deserialize(&mut Infallible).unwrap();
            if let Test::A { value, other } = &deserialized {
                assert_eq!(*value, 10);
                assert_eq!(*other, 10);
            } else {
                panic!("expected variant A");
            };

            let value = Test::B(10, 10);
            let mut serializer = AlignedSerializer::new(AlignedVec::new());
            serializer.serialize_value(&value).unwrap();
            let result = serializer.into_inner();
            let archived = unsafe { archived_root::<Test>(result.as_slice()) };

            if let ArchivedTest::B(value, other) = archived {
                assert_eq!(*value, "10");
                assert_eq!(*other, 10);
            } else {
                panic!("expected variant B");
            };

            let deserialized: Test =
                archived.deserialize(&mut Infallible).unwrap();
            if let Test::B(value, other) = &deserialized {
                assert_eq!(*value, 10);
                assert_eq!(*other, 10);
            } else {
                panic!("expected variant B");
            };
        }
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn with_atomic_load() {
        use core::sync::atomic::{AtomicU32, Ordering};
        use rkyv::with::{AtomicLoad, Relaxed};

        #[derive(Archive, Debug, Deserialize, Serialize)]
        #[archive_attr(derive(Debug))]
        struct Test {
            #[with(AtomicLoad<Relaxed>)]
            a: AtomicU32,
        }

        impl PartialEq for Test {
            fn eq(&self, other: &Self) -> bool {
                self.a.load(Ordering::Relaxed)
                    == other.a.load(Ordering::Relaxed)
            }
        }

        impl PartialEq<Test> for ArchivedTest {
            fn eq(&self, other: &Test) -> bool {
                self.a == other.a.load(Ordering::Relaxed)
            }
        }

        let value = Test {
            a: AtomicU32::new(42),
        };
        test_archive(&value);
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn with_as_atomic() {
        use core::sync::atomic::{AtomicU32, Ordering};
        use rkyv::with::{AsAtomic, Relaxed};

        #[derive(Archive, Serialize, Deserialize)]
        struct Test {
            #[with(AsAtomic<Relaxed, Relaxed>)]
            value: AtomicU32,
        }

        let value = Test {
            value: AtomicU32::new(42),
        };
        let mut serializer = AlignedSerializer::new(AlignedVec::new());
        serializer.serialize_value(&value).unwrap();
        let mut result = serializer.into_inner();
        // NOTE: with(Atomic) is only sound if the backing memory is mutable, use with caution!
        let archived = unsafe {
            archived_root_mut::<Test>(Pin::new(result.as_mut_slice()))
        };

        assert_eq!(archived.value.load(Ordering::Relaxed), 42);
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn with_inline() {
        use rkyv::with::Inline;

        #[derive(Archive, Serialize, Deserialize)]
        struct Test<'a> {
            #[with(Inline)]
            value: &'a i32,
        }

        let a = 42;
        let value = Test { value: &a };
        let mut serializer = AlignedSerializer::new(AlignedVec::new());
        serializer.serialize_value(&value).unwrap();
        let result = serializer.into_inner();
        let archived = unsafe { archived_root::<Test>(result.as_slice()) };

        assert_eq!(archived.value, 42);
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn with_boxed() {
        use rkyv::with::Boxed;

        #[derive(Archive, Serialize, Deserialize)]
        struct Test {
            #[with(Boxed)]
            value: i32,
        }

        let value = Test { value: 42 };
        let mut serializer = AlignedSerializer::new(AlignedVec::new());
        serializer.serialize_value(&value).unwrap();
        let result = serializer.into_inner();
        let archived = unsafe { archived_root::<Test>(result.as_slice()) };

        assert_eq!(archived.value.get(), &42);
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn with_boxed_inline() {
        use rkyv::with::BoxedInline;

        #[derive(Archive, Serialize, Deserialize)]
        struct Test<'a> {
            #[with(BoxedInline)]
            value: &'a str,
        }

        let a = "hello world";
        let value = Test { value: &a };
        let mut serializer = AlignedSerializer::new(AlignedVec::new());
        serializer.serialize_value(&value).unwrap();
        let result = serializer.into_inner();
        let archived = unsafe { archived_root::<Test>(result.as_slice()) };

        assert_eq!(archived.value.as_ref(), "hello world");
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn with_as_owned() {
        use rkyv::with::AsOwned;

        #[derive(Archive, Serialize, Deserialize)]
        struct Test<'a> {
            #[with(AsOwned)]
            a: Cow<'a, u32>,
            #[with(AsOwned)]
            b: Cow<'a, [u32]>,
            #[with(AsOwned)]
            c: Cow<'a, str>,
        }

        let value = Test {
            a: Cow::Borrowed(&100),
            b: Cow::Borrowed(&[1, 2, 3, 4, 5, 6]),
            c: Cow::Borrowed("hello world"),
        };
        let mut serializer = DefaultSerializer::default();
        serializer.serialize_value(&value).unwrap();
        let result = serializer.into_serializer().into_inner();
        let archived = unsafe { archived_root::<Test>(result.as_slice()) };

        assert_eq!(archived.a, 100);
        assert_eq!(archived.b, [1, 2, 3, 4, 5, 6]);
        assert_eq!(archived.c, "hello world");
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn with_as_vec() {
        #[cfg(not(feature = "std"))]
        use alloc::collections::{BTreeMap, BTreeSet};
        use rkyv::{collections::util::Entry, with::AsVec};
        #[cfg(feature = "std")]
        use std::collections::{BTreeMap, BTreeSet};

        #[derive(Archive, Serialize, Deserialize)]
        struct Test {
            #[with(AsVec)]
            a: BTreeMap<String, String>,
            #[with(AsVec)]
            b: BTreeSet<String>,
            #[with(AsVec)]
            c: BTreeMap<String, String>,
        }

        let mut a = BTreeMap::new();
        a.insert("foo".to_string(), "hello".to_string());
        a.insert("bar".to_string(), "world".to_string());
        a.insert("baz".to_string(), "bat".to_string());

        let mut b = BTreeSet::new();
        b.insert("foo".to_string());
        b.insert("hello world!".to_string());
        b.insert("bar".to_string());
        b.insert("fizzbuzz".to_string());

        let c = BTreeMap::new();

        let value = Test { a, b, c };

        let mut serializer = DefaultSerializer::default();
        serializer.serialize_value(&value).unwrap();
        let result = serializer.into_serializer().into_inner();
        let archived = unsafe { archived_root::<Test>(result.as_slice()) };

        assert_eq!(archived.a.len(), 3);
        assert!(archived
            .a
            .iter()
            .find(|&e| e
                == &Entry {
                    key: "foo",
                    value: "hello"
                })
            .is_some());
        assert!(archived
            .a
            .iter()
            .find(|&e| e
                == &Entry {
                    key: "bar",
                    value: "world"
                })
            .is_some());
        assert!(archived
            .a
            .iter()
            .find(|&e| e
                == &Entry {
                    key: "baz",
                    value: "bat"
                })
            .is_some());

        assert_eq!(archived.b.len(), 4);
        assert!(archived.b.iter().find(|&e| e == "foo").is_some());
        assert!(archived.b.iter().find(|&e| e == "hello world!").is_some());
        assert!(archived.b.iter().find(|&e| e == "bar").is_some());
        assert!(archived.b.iter().find(|&e| e == "fizzbuzz").is_some());
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn with_niche() {
        use core::mem::size_of;
        use rkyv::with::Niche;

        #[derive(Archive, Serialize, Deserialize)]
        struct Test {
            #[with(Niche)]
            inner: Option<Box<String>>,
        }

        #[derive(Archive, Serialize, Deserialize)]
        struct TestNoNiching {
            inner: Option<Box<String>>,
        }

        let value = Test {
            inner: Some(Box::new("hello world".to_string())),
        };
        let mut serializer = DefaultSerializer::default();
        serializer.serialize_value(&value).unwrap();
        let result = serializer.into_serializer().into_inner();
        let archived = unsafe { archived_root::<Test>(result.as_slice()) };

        assert!(archived.inner.is_some());
        assert_eq!(&**archived.inner.as_ref().unwrap(), "hello world");
        assert_eq!(archived.inner, value.inner);

        let value = Test { inner: None };
        let mut serializer = DefaultSerializer::default();
        serializer.serialize_value(&value).unwrap();
        let result = serializer.into_serializer().into_inner();
        let archived = unsafe { archived_root::<Test>(result.as_slice()) };

        assert!(archived.inner.is_none());
        assert_eq!(archived.inner, value.inner);

        assert!(
            size_of::<Archived<Test>>() < size_of::<Archived<TestNoNiching>>()
        );
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn with_niche_nonzero() {
        use core::{
            mem::size_of,
            num::{
                NonZeroI32, NonZeroI8, NonZeroIsize, NonZeroU32, NonZeroU8,
                NonZeroUsize,
            },
        };
        use rkyv::with::Niche;

        #[derive(Archive, Serialize, Deserialize)]
        struct Test {
            #[with(Niche)]
            a: Option<NonZeroI8>,
            #[with(Niche)]
            b: Option<NonZeroI32>,
            #[with(Niche)]
            c: Option<NonZeroIsize>,
            #[with(Niche)]
            d: Option<NonZeroU8>,
            #[with(Niche)]
            e: Option<NonZeroU32>,
            #[with(Niche)]
            f: Option<NonZeroUsize>,
        }

        #[derive(Archive, Serialize, Deserialize)]
        struct TestNoNiching {
            a: Option<NonZeroI8>,
            b: Option<NonZeroI32>,
            c: Option<NonZeroIsize>,
            d: Option<NonZeroU8>,
            e: Option<NonZeroU32>,
            f: Option<NonZeroUsize>,
        }

        let value = Test {
            a: Some(NonZeroI8::new(10).unwrap()),
            b: Some(NonZeroI32::new(10).unwrap()),
            c: Some(NonZeroIsize::new(10).unwrap()),
            d: Some(NonZeroU8::new(10).unwrap()),
            e: Some(NonZeroU32::new(10).unwrap()),
            f: Some(NonZeroUsize::new(10).unwrap()),
        };
        let mut serializer = DefaultSerializer::default();
        serializer.serialize_value(&value).unwrap();
        let result = serializer.into_serializer().into_inner();
        let archived = unsafe { archived_root::<Test>(result.as_slice()) };

        assert!(archived.a.is_some());
        assert_eq!(archived.a.as_ref().unwrap().get(), 10);
        assert!(archived.b.is_some());
        assert_eq!(archived.b.as_ref().unwrap().get(), 10);
        assert!(archived.c.is_some());
        assert_eq!(archived.c.as_ref().unwrap().get(), 10);
        assert!(archived.d.is_some());
        assert_eq!(archived.d.as_ref().unwrap().get(), 10);
        assert!(archived.e.is_some());
        assert_eq!(archived.e.as_ref().unwrap().get(), 10);
        assert!(archived.f.is_some());
        assert_eq!(archived.f.as_ref().unwrap().get(), 10);

        let value = Test {
            a: None,
            b: None,
            c: None,
            d: None,
            e: None,
            f: None,
        };
        let mut serializer = DefaultSerializer::default();
        serializer.serialize_value(&value).unwrap();
        let result = serializer.into_serializer().into_inner();
        let archived = unsafe { archived_root::<Test>(result.as_slice()) };

        assert!(archived.a.is_none());
        assert!(archived.b.is_none());
        assert!(archived.c.is_none());
        assert!(archived.d.is_none());
        assert!(archived.e.is_none());
        assert!(archived.f.is_none());

        assert!(
            size_of::<Archived<Test>>() < size_of::<Archived<TestNoNiching>>()
        );
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn with_copy_optimize() {
        use rkyv::with::CopyOptimize;

        #[derive(Archive, Serialize, Deserialize)]
        struct Test {
            #[with(CopyOptimize)]
            bytes: Vec<u8>,
            #[with(CopyOptimize)]
            words: Box<[u32]>,
        }

        let value = Test {
            bytes: vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
            words: vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9].into_boxed_slice(),
        };
        let mut serializer = DefaultSerializer::default();
        serializer.serialize_value(&value).unwrap();
        let result = serializer.into_serializer().into_inner();
        let archived = unsafe { archived_root::<Test>(result.as_slice()) };

        assert_eq!(archived.bytes, value.bytes);

        let deserialized: Test = archived.deserialize(&mut Infallible).unwrap();
        assert_eq!(deserialized.bytes, value.bytes);
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn with_boxed_inline_copy_optimize() {
        use rkyv::with::{BoxedInline, CopyOptimize};

        #[derive(Archive, Serialize, Deserialize)]
        struct Test<'a> {
            #[with(CopyOptimize, BoxedInline)]
            bytes: &'a [u8],
        }

        let bytes = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        let value = Test {
            bytes: bytes.as_ref(),
        };
        let mut serializer = DefaultSerializer::default();
        serializer.serialize_value(&value).unwrap();
        let result = serializer.into_serializer().into_inner();
        let archived = unsafe { archived_root::<Test>(result.as_slice()) };

        assert_eq!(&*archived.bytes, value.bytes);
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn with_raw() {
        use rkyv::with::Raw;

        #[derive(Archive, Serialize, Deserialize)]
        struct Test {
            #[with(Raw)]
            bytes: Vec<u8>,
        }

        let value = Test {
            bytes: vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
        };
        let mut serializer = DefaultSerializer::default();
        serializer.serialize_value(&value).unwrap();
        let result = serializer.into_serializer().into_inner();
        let archived = unsafe { archived_root::<Test>(result.as_slice()) };

        assert_eq!(&*archived.bytes, value.bytes);
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn with_unsafe() {
        use core::cell::UnsafeCell;
        use rkyv::{primitive::ArchivedU32, with::Unsafe};

        #[derive(Archive, Serialize, Deserialize)]
        struct Test {
            #[with(Unsafe)]
            inner: UnsafeCell<u32>,
        }

        let value = Test {
            inner: UnsafeCell::new(100),
        };
        let mut serializer = DefaultSerializer::default();
        serializer.serialize_value(&value).unwrap();
        let mut result = serializer.into_serializer().into_inner();
        let bytes = unsafe { Pin::new_unchecked(result.as_mut_slice()) };
        let archived = unsafe { archived_root_mut::<Test>(bytes) };

        unsafe {
            assert_eq!(*archived.inner.get(), 100);
            *archived.inner.get() = ArchivedU32::from_native(42u32);
            assert_eq!(*archived.inner.get(), 42);
        }

        let deserialized: Test = (&*archived)
            .deserialize(&mut DefaultDeserializer::default())
            .unwrap();
        unsafe {
            assert_eq!(*deserialized.inner.get(), 42);
            *deserialized.inner.get() = 88;
            assert_eq!(*deserialized.inner.get(), 88);
        }
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn archive_crate_path() {
        use rkyv as alt_path;

        #[derive(Archive, Deserialize, Serialize)]
        #[archive(crate = "alt_path")]
        struct Test<'a> {
            #[with(alt_path::with::BoxedInline)]
            value: &'a str,
            other: i32,
        }
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn archive_btree_map() {
        let mut value = BTreeMap::new();
        value.insert("foo".to_string(), 10);
        value.insert("bar".to_string(), 20);
        value.insert("baz".to_string(), 40);
        value.insert("bat".to_string(), 80);

        let mut serializer = AlignedSerializer::new(AlignedVec::new());
        serializer.serialize_value(&value).unwrap();
        let result = serializer.into_inner();
        let archived = unsafe {
            archived_root::<BTreeMap<String, i32>>(result.as_slice())
        };

        assert_eq!(archived.len(), 4);
        for (k, v) in value.iter() {
            let (ak, av) = archived
                .get_key_value(k.as_str())
                .expect("failed to find key in archived B-tree map");
            assert_eq!(k, ak);
            assert_eq!(v, av);
        }
        assert!(archived.get_key_value("wrong!").is_none());

        let deserialized: BTreeMap<_, _> =
            archived.deserialize(&mut Infallible).unwrap();
        assert_eq!(value, deserialized);
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn archive_empty_btree_map() {
        let value: BTreeMap<String, i32> = BTreeMap::new();

        let mut serializer = AlignedSerializer::new(AlignedVec::new());
        serializer.serialize_value(&value).unwrap();
        let result = serializer.into_inner();
        let archived = unsafe {
            archived_root::<BTreeMap<String, i32>>(result.as_slice())
        };

        assert_eq!(archived.len(), 0);
        for _ in archived.iter() {
            panic!("there should be no values in the archived empty btree");
        }
        assert!(archived.get_key_value("wrong!").is_none());

        let deserialized: BTreeMap<_, _> =
            archived.deserialize(&mut Infallible).unwrap();
        assert_eq!(value, deserialized);
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn archive_btree_set() {
        let mut value = BTreeSet::new();
        value.insert("foo".to_string());
        value.insert("bar".to_string());
        value.insert("baz".to_string());
        value.insert("bat".to_string());

        let mut serializer = AlignedSerializer::new(AlignedVec::new());
        serializer.serialize_value(&value).unwrap();
        let result = serializer.into_inner();
        let archived =
            unsafe { archived_root::<BTreeSet<String>>(result.as_slice()) };

        assert_eq!(archived.len(), 4);
        for k in value.iter() {
            let ak = archived
                .get(k.as_str())
                .expect("failed to find key in archived B-tree map");
            assert_eq!(k, ak);
        }
        assert!(archived.get("wrong!").is_none());

        let deserialized: BTreeSet<_> =
            archived.deserialize(&mut Infallible).unwrap();
        assert_eq!(value, deserialized);
    }

    #[test]
    // This test is unfortunately too slow to run through miri
    #[cfg_attr(miri, ignore)]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    // This test creates structures too big to fit in 16-bit offsets
    #[cfg(not(feature = "size_16"))]
    fn archive_btree_map_large() {
        let mut value = BTreeMap::new();
        for i in 0..100_000 {
            value.insert(i.to_string(), i);
        }

        let mut serializer = AlignedSerializer::new(AlignedVec::new());
        serializer.serialize_value(&value).unwrap();
        let result = serializer.into_inner();
        let archived = unsafe {
            archived_root::<BTreeMap<String, i32>>(result.as_slice())
        };

        assert_eq!(archived.len(), 100_000);

        for ((k, v), (ak, av)) in value.iter().zip(archived.iter()) {
            assert_eq!(k, ak);
            assert_eq!(v, av);
        }

        for (k, v) in value.iter() {
            let av = archived
                .get(k.as_str())
                .expect("failed to find key in archived B-tree map");
            assert_eq!(v, av);
        }
        assert!(archived.get("wrong!").is_none());

        let deserialized: BTreeMap<_, _> =
            archived.deserialize(&mut Infallible).unwrap();
        assert_eq!(value, deserialized);
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn archive_zst_containers() {
        #[derive(Archive, Deserialize, Serialize, Debug, PartialEq)]
        #[archive(compare(PartialEq))]
        #[archive_attr(derive(Debug))]
        struct MyZST;

        test_archive(&Box::new(MyZST));

        test_archive(&Vec::<MyZST>::new().into_boxed_slice());
        test_archive(&vec![MyZST, MyZST, MyZST, MyZST].into_boxed_slice());

        test_archive(&Vec::<MyZST>::new());
        test_archive(&vec![MyZST, MyZST, MyZST, MyZST]);

        let mut value = BTreeMap::new();
        value.insert(0, ());
        value.insert(1, ());
        test_archive(&value);

        let mut value = BTreeMap::new();
        value.insert((), 10);
        test_archive(&value);

        let mut value = BTreeMap::new();
        value.insert((), ());
        test_archive(&value);

        let mut value = BTreeSet::new();
        value.insert(());
        test_archive(&value);

        #[derive(Archive, Deserialize, Serialize, Debug, PartialEq)]
        #[archive(compare(PartialEq))]
        #[archive_attr(derive(Debug))]
        struct TestRcZST {
            a: Rc<()>,
            b: Rc<()>,
        }

        let rc_zst = Rc::new(());
        test_archive(&TestRcZST {
            a: rc_zst.clone(),
            b: rc_zst.clone(),
        });
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn scratch_tracker() {
        use rkyv::ser::serializers::{
            AlignedSerializer, AllocScratch, CompositeSerializer,
            ScratchTracker,
        };

        type TrackerSerializer = CompositeSerializer<
            AlignedSerializer<AlignedVec>,
            ScratchTracker<AllocScratch>,
            Infallible,
        >;
        fn track_serialize<T>(value: &T) -> ScratchTracker<AllocScratch>
        where
            T: Serialize<TrackerSerializer>,
        {
            let mut serializer = CompositeSerializer::new(
                AlignedSerializer::<AlignedVec>::default(),
                ScratchTracker::new(AllocScratch::default()),
                Infallible,
            );
            serializer
                .serialize_value(value)
                .expect("failed to serialize value");
            serializer.into_components().1
        }

        let tracker = track_serialize(&42);
        assert_eq!(tracker.max_bytes_allocated(), 0);
        assert_eq!(tracker.max_allocations(), 0);
        assert_eq!(tracker.max_alignment(), 1);
        assert_eq!(tracker.min_buffer_size(), 0);
        assert_eq!(tracker.min_buffer_size_max_error(), 0);

        let tracker = track_serialize(&vec![1, 2, 3, 4]);
        assert_eq!(tracker.max_bytes_allocated(), 0);
        assert_eq!(tracker.max_allocations(), 0);
        assert_eq!(tracker.max_alignment(), 1);
        assert_eq!(tracker.min_buffer_size(), 0);
        assert_eq!(tracker.min_buffer_size_max_error(), 0);

        let tracker = track_serialize(&vec![vec![1, 2], vec![3, 4]]);
        assert_ne!(tracker.max_bytes_allocated(), 0);
        assert_eq!(tracker.max_allocations(), 1);
        assert_ne!(tracker.min_buffer_size(), 0);
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn archive_manually_drop() {
        use core::mem::ManuallyDrop;

        let vec = ManuallyDrop::new(vec![
            "hello world".to_string(),
            "me too!".to_string(),
        ]);

        let mut serializer = DefaultSerializer::default();
        serializer.serialize_value(&vec).unwrap();
        let result = serializer.into_serializer().into_inner();
        let archived = unsafe {
            archived_root::<ManuallyDrop<Vec<String>>>(result.as_slice())
        };

        assert_eq!(archived.len(), vec.len());
        for (a, b) in archived.iter().zip(vec.iter()) {
            assert_eq!(a, b);
        }

        drop(ManuallyDrop::into_inner(vec));
    }
}
