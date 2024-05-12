#[cfg(test)]
mod tests {
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
    use core::{convert::Infallible, pin::Pin};
    #[cfg(feature = "std")]
    use std::{
        borrow::Cow,
        collections::{BTreeMap, BTreeSet},
        rc::{Rc, Weak},
    };

    use rkyv::{
        access_unchecked, access_unchecked_mut,
        de::Pool,
        rancor::{Error, Fallible, Source, Strategy},
        ser::{
            allocator::{AllocationStats, Arena},
            sharing::Share,
            Serializer, Writer,
        },
        to_bytes,
        util::{deserialize, serialize_into, AlignedVec},
        Archive, Archived, Deserialize, Place, Portable, Serialize,
    };
    #[cfg(feature = "wasm")]
    use wasm_bindgen_test::*;

    use crate::util::alloc::*;

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn archive_containers() {
        test_archive_with(&Box::new(42), |a, b| **a == **b);
        test_archive_with(&"".to_string().into_boxed_str(), |a, b| **a == **b);
        test_archive_with(
            &"hello world".to_string().into_boxed_str(),
            |a, b| **a == **b,
        );
        test_archive_with(&Vec::<i32>::new().into_boxed_slice(), |a, b| {
            **a == **b
        });
        test_archive_with(&vec![1, 2, 3, 4].into_boxed_slice(), |a, b| {
            **a == **b
        });
        test_archive_with(&"".to_string(), |a, b| **a == **b);
        test_archive_with(&"hello world".to_string(), |a, b| **a == **b);
        test_archive_with(&Vec::<i32>::new(), |a, b| **a == **b);
        test_archive_with(&vec![1, 2, 3, 4], |a, b| **a == **b);
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

    #[cfg(all(feature = "std", feature = "bytecheck"))]
    mod isolate {
        #[cfg(feature = "wasm")]
        use wasm_bindgen_test::*;

        #[test]
        #[allow(unused_variables)]
        #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
        fn archive_example() {
            use rkyv::{
                deserialize, rancor::Error, util::serialize_into, Archive,
                Deserialize, Serialize,
            };

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
            let bytes = rkyv::to_bytes::<Error>(&value).unwrap();

            // Or you can customize your serialization for better performance
            // and compatibility with #![no_std] environments
            use rkyv::{
                ser::{allocator::Arena, sharing::Share, Serializer},
                util::AlignedVec,
            };

            let mut arena = Arena::new();
            let serializer = serialize_into::<_, Error>(
                &value,
                Serializer::new(
                    AlignedVec::new(),
                    arena.acquire(),
                    Share::new(),
                ),
            )
            .unwrap();
            let bytes = serializer.into_writer();

            // You can use the safe API for fast zero-copy deserialization
            let archived =
                rkyv::access::<ArchivedTest, Error>(&bytes[..]).unwrap();
            assert_eq!(archived, &value);

            // Or you can use the unsafe API for maximum performance
            let archived =
                unsafe { rkyv::access_unchecked::<ArchivedTest>(&bytes[..]) };
            assert_eq!(archived, &value);

            // And you can always deserialize back to the original type
            let deserialized =
                deserialize::<Test, _, Error>(archived, &mut ()).unwrap();
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
    fn archive_derives() {
        #[derive(Archive, Serialize, Clone)]
        #[archive_attr(derive(Clone, Debug, PartialEq))]
        struct Test(i32);

        let value = Test(42);

        let buf = to_bytes::<Error>(&value).expect("failed to archive value");
        let archived_value =
            unsafe { access_unchecked::<ArchivedTest>(buf.as_ref()) };

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
        let mut buf = to_bytes::<Error>(&42i32).unwrap();
        let mut value =
            unsafe { access_unchecked_mut::<Archived<i32>>(buf.as_mut()) };
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

        let mut buf = to_bytes::<Error>(&value).unwrap();
        let mut value =
            unsafe { access_unchecked_mut::<ArchivedTest>(buf.as_mut()) };

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

        let mut buf = to_bytes::<Error>(&value).unwrap();
        let mut value =
            unsafe { access_unchecked_mut::<ArchivedTest>(buf.as_mut()) };

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
        // The derive macros don't apply the right bounds from Box so we have to
        // manually specify what bounds to apply
        #[archive(serialize_bounds(__S: Writer))]
        #[archive(deserialize_bounds(__D::Error: Source))]
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
        // The derive macros don't apply the right bounds from Box so we have to
        // manually specify what bounds to apply
        #[archive(serialize_bounds(__S: Writer))]
        #[archive(deserialize_bounds(__D::Error: Source))]
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

        #[derive(Portable)]
        #[repr(transparent)]
        struct MyStruct<T> {
            _phantom: PhantomData<T>,
        }

        impl<T: Archive + MyTrait> Archive for MyStruct<T> {
            type Archived = MyStruct<T::Archived>;
            type Resolver = ();

            fn resolve(&self, _: Self::Resolver, _: Place<Self::Archived>) {}
        }

        impl<T, S> Serialize<S> for MyStruct<T>
        where
            T: Archive + MyTrait,
            S: Fallible + MyTrait + ?Sized,
        {
            fn serialize(&self, _: &mut S) -> Result<Self::Resolver, S::Error> {
                Ok(())
            }
        }

        impl<T, D> Deserialize<MyStruct<T>, D> for MyStruct<T::Archived>
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
        #[archive(
            archive_bounds(T: MyTrait),
            serialize_bounds(__S: MyTrait),
            deserialize_bounds(__D: MyTrait),
        )]
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

        let mut buf = to_bytes::<Error>(&value).unwrap();

        let archived =
            unsafe { access_unchecked::<ArchivedTest>(buf.as_ref()) };
        assert_eq!(archived, &value);

        let mut mutable_archived =
            unsafe { access_unchecked_mut::<ArchivedTest>(buf.as_mut()) };
        unsafe {
            *mutable_archived.as_mut().a().get_pin_mut_unchecked() =
                42u32.into();
        }

        let archived =
            unsafe { access_unchecked::<ArchivedTest>(buf.as_ref()) };
        assert_eq!(*archived.a, 42);
        assert_eq!(*archived.b, 42);

        let mut mutable_archived =
            unsafe { access_unchecked_mut::<ArchivedTest>(buf.as_mut()) };
        unsafe {
            *mutable_archived.as_mut().b().get_pin_mut_unchecked() =
                17u32.into();
        }

        let archived =
            unsafe { access_unchecked::<ArchivedTest>(buf.as_ref()) };
        assert_eq!(*archived.a, 17);
        assert_eq!(*archived.b, 17);

        let mut deserializer = Pool::new();
        let deserialized =
            deserialize::<Test, _, Error>(archived, &mut deserializer).unwrap();

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

        let mut buf = to_bytes::<Error>(&value).unwrap();

        let archived =
            unsafe { access_unchecked::<ArchivedTest>(buf.as_ref()) };
        assert_eq!(*archived.a, 10);
        assert!(archived.b.upgrade().is_some());
        assert_eq!(**archived.b.upgrade().unwrap(), 10);

        let mut mutable_archived =
            unsafe { access_unchecked_mut::<ArchivedTest>(buf.as_mut()) };
        unsafe {
            *mutable_archived.as_mut().a().get_pin_mut_unchecked() =
                42u32.into();
        }

        let archived =
            unsafe { access_unchecked::<ArchivedTest>(buf.as_ref()) };
        assert_eq!(*archived.a, 42);
        assert!(archived.b.upgrade().is_some());
        assert_eq!(**archived.b.upgrade().unwrap(), 42);

        let mut mutable_archived =
            unsafe { access_unchecked_mut::<ArchivedTest>(buf.as_mut()) };
        unsafe {
            *mutable_archived
                .as_mut()
                .b()
                .upgrade_pin_mut()
                .unwrap()
                .get_pin_mut_unchecked() = 17u32.into();
        }

        let archived =
            unsafe { access_unchecked::<ArchivedTest>(buf.as_ref()) };
        assert_eq!(*archived.a, 17);
        assert!(archived.b.upgrade().is_some());
        assert_eq!(**archived.b.upgrade().unwrap(), 17);

        let mut deserializer = Pool::new();
        let deserialized =
            deserialize::<Test, _, Error>(archived, &mut deserializer).unwrap();

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
        #[archive(archived = ATest, resolver = RTest, compare(PartialEq))]
        #[archive_attr(derive(Debug))]
        struct Test {
            a: i32,
            b: Option<u32>,
            c: String,
            d: Vec<i32>,
        }

        impl<S> Serialize<S> for Test
        where
            S: Fallible + ?Sized,
            S::Error: Source,
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

        impl<D> Deserialize<Test, D> for ATest
        where
            D: Fallible + ?Sized,
            D::Error: Source,
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

        #[derive(Archive, Serialize, Deserialize, Debug, Portable)]
        #[archive(as = "ExampleStruct<T::Archived>")]
        #[repr(transparent)]
        struct ExampleStruct<T> {
            value: T,
        }

        impl<T, U> PartialEq<ExampleStruct<U>> for ExampleStruct<T>
        where
            T: PartialEq<U>,
        {
            fn eq(&self, other: &ExampleStruct<U>) -> bool {
                self.value.eq(&other.value)
            }
        }

        let value = ExampleStruct {
            value: "hello world".to_string(),
        };

        test_archive(&value);

        // Tuple struct

        #[derive(Archive, Serialize, Deserialize, Portable, Debug)]
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

        #[derive(
            Archive, Serialize, Deserialize, Debug, Portable, PartialEq,
        )]
        #[archive(as = "ExampleUnitStruct")]
        #[repr(C)]
        struct ExampleUnitStruct;

        test_archive(&ExampleUnitStruct);

        // Enum

        #[derive(Archive, Serialize, Deserialize, Portable, Debug)]
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
        use core::{convert::Infallible, str::FromStr};

        use rkyv::{
            access_unchecked, deserialize,
            rancor::{Error, Fallible},
            ser::Writer,
            util::{serialize_into, AlignedVec},
            with::{ArchiveWith, DeserializeWith, SerializeWith},
            Archive, Archived, Deserialize, Place, Serialize,
        };
        #[cfg(feature = "wasm")]
        use wasm_bindgen_test::*;

        struct ConvertToString;

        impl<T: ToString> ArchiveWith<T> for ConvertToString {
            type Archived = <String as Archive>::Archived;
            type Resolver = <String as Archive>::Resolver;

            fn resolve_with(
                value: &T,
                resolver: Self::Resolver,
                out: Place<Self::Archived>,
            ) {
                value.to_string().resolve(resolver, out);
            }
        }

        impl<T: ToString, S: Fallible + Writer + ?Sized> SerializeWith<T, S>
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
            let result =
                serialize_into::<_, Error>(&value, AlignedVec::new()).unwrap();
            let archived =
                unsafe { access_unchecked::<ArchivedTest>(result.as_slice()) };

            assert_eq!(archived.value, "10");
            assert_eq!(archived.other, 10);

            let deserialized =
                deserialize::<Test, _, Infallible>(archived, &mut ()).unwrap();
            assert_eq!(deserialized.value, 10);
            assert_eq!(deserialized.other, 10);
        }

        #[test]
        #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
        fn with_tuple_struct() {
            #[derive(Archive, Serialize, Deserialize)]
            struct Test(#[with(ConvertToString)] i32, i32);

            let value = Test(10, 10);
            let result =
                serialize_into::<_, Error>(&value, AlignedVec::new()).unwrap();
            let archived =
                unsafe { access_unchecked::<ArchivedTest>(result.as_slice()) };

            assert_eq!(archived.0, "10");
            assert_eq!(archived.1, 10);

            let deserialized =
                deserialize::<Test, _, Infallible>(archived, &mut ()).unwrap();
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
            let result =
                serialize_into::<_, Error>(&value, AlignedVec::new()).unwrap();
            let archived =
                unsafe { access_unchecked::<ArchivedTest>(result.as_slice()) };

            if let ArchivedTest::A { value, other } = archived {
                assert_eq!(*value, "10");
                assert_eq!(*other, 10);
            } else {
                panic!("expected variant A");
            };

            let deserialized =
                deserialize::<Test, _, Infallible>(archived, &mut ()).unwrap();
            if let Test::A { value, other } = &deserialized {
                assert_eq!(*value, 10);
                assert_eq!(*other, 10);
            } else {
                panic!("expected variant A");
            };

            let value = Test::B(10, 10);
            let result =
                serialize_into::<_, Error>(&value, AlignedVec::new()).unwrap();
            let archived =
                unsafe { access_unchecked::<ArchivedTest>(result.as_slice()) };

            if let ArchivedTest::B(value, other) = archived {
                assert_eq!(*value, "10");
                assert_eq!(*other, 10);
            } else {
                panic!("expected variant B");
            };

            let deserialized =
                deserialize::<Test, _, Infallible>(archived, &mut ()).unwrap();
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
        let mut result =
            serialize_into::<_, Error>(&value, AlignedVec::new()).unwrap();
        // NOTE: with(Atomic) is only sound if the backing memory is mutable,
        // use with caution!
        let archived = unsafe {
            access_unchecked_mut::<ArchivedTest>(result.as_mut_slice())
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
        let result =
            serialize_into::<_, Error>(&value, AlignedVec::new()).unwrap();
        let archived =
            unsafe { access_unchecked::<ArchivedTest>(result.as_slice()) };

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
        let result =
            serialize_into::<_, Error>(&value, AlignedVec::new()).unwrap();
        let archived =
            unsafe { access_unchecked::<ArchivedTest>(result.as_slice()) };

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
        let result =
            serialize_into::<_, Error>(&value, AlignedVec::new()).unwrap();
        let archived =
            unsafe { access_unchecked::<ArchivedTest>(result.as_slice()) };

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
        let bytes = to_bytes::<Error>(&value).unwrap();
        let archived = unsafe { access_unchecked::<ArchivedTest>(&bytes) };

        assert_eq!(archived.a, 100);
        assert_eq!(archived.b, [1, 2, 3, 4, 5, 6]);
        assert_eq!(archived.c, "hello world");
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn with_as_vec() {
        #[cfg(not(feature = "std"))]
        use alloc::collections::{BTreeMap, BTreeSet};
        #[cfg(feature = "std")]
        use std::collections::{BTreeMap, BTreeSet};

        use rkyv::with::AsVec;

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

        let bytes = to_bytes::<Error>(&value).unwrap();
        let archived = unsafe { access_unchecked::<ArchivedTest>(&bytes) };

        assert_eq!(archived.a.len(), 3);
        assert!(archived
            .a
            .iter()
            .find(|&e| e.key == "foo" && e.value == "hello")
            .is_some());
        assert!(archived
            .a
            .iter()
            .find(|&e| e.key == "bar" && e.value == "world")
            .is_some());
        assert!(archived
            .a
            .iter()
            .find(|&e| e.key == "baz" && e.value == "bat")
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
        let bytes = to_bytes::<Error>(&value).unwrap();
        let archived = unsafe { access_unchecked::<ArchivedTest>(&bytes) };

        assert!(archived.inner.is_some());
        assert_eq!(&**archived.inner.as_ref().unwrap(), "hello world");
        assert_eq!(archived.inner, value.inner);

        let value = Test { inner: None };
        let bytes = to_bytes::<Error>(&value).unwrap();
        let archived = unsafe { access_unchecked::<ArchivedTest>(&bytes) };

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
        let bytes = to_bytes::<Error>(&value).unwrap();
        let archived = unsafe { access_unchecked::<ArchivedTest>(&bytes) };

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
        let bytes = to_bytes::<Error>(&value).unwrap();
        let archived = unsafe { access_unchecked::<ArchivedTest>(&bytes) };

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
        let mut bytes = to_bytes::<Error>(&value).unwrap();
        let archived =
            unsafe { access_unchecked_mut::<ArchivedTest>(&mut bytes) };

        unsafe {
            assert_eq!(*archived.inner.get(), 100);
            *archived.inner.get() = ArchivedU32::from_native(42u32);
            assert_eq!(*archived.inner.get(), 42);
        }

        let deserialized =
            deserialize::<Test, _, Error>(&*archived, &mut Pool::new())
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
        #[archive(crate = alt_path)]
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

        let result = to_bytes::<Error>(&value).unwrap();
        let archived = unsafe {
            access_unchecked::<Archived<BTreeMap<String, i32>>>(
                result.as_slice(),
            )
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

        let deserialized =
            deserialize::<BTreeMap<_, _>, _, Infallible>(archived, &mut ())
                .unwrap();
        assert_eq!(value, deserialized);
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn archive_empty_btree_map() {
        let value: BTreeMap<String, i32> = BTreeMap::new();

        let result = to_bytes::<Error>(&value).unwrap();
        let archived = unsafe {
            access_unchecked::<Archived<BTreeMap<String, i32>>>(
                result.as_slice(),
            )
        };

        assert_eq!(archived.len(), 0);
        #[allow(clippy::never_loop)]
        archived.visit(|_, _| -> ::core::ops::ControlFlow<()> {
            panic!("there should be no values in the archived empty btree");
        });
        assert!(archived.get_key_value("wrong!").is_none());

        let deserialized =
            deserialize::<BTreeMap<_, _>, _, Infallible>(archived, &mut ())
                .unwrap();
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

        let result = to_bytes::<Error>(&value).unwrap();
        let archived = unsafe {
            access_unchecked::<Archived<BTreeSet<String>>>(result.as_slice())
        };

        assert_eq!(archived.len(), 4);
        for k in value.iter() {
            let ak = archived
                .get(k.as_str())
                .expect("failed to find key in archived B-tree map");
            assert_eq!(k, ak);
        }
        assert!(archived.get("wrong!").is_none());

        let deserialized =
            deserialize::<BTreeSet<_>, _, Infallible>(archived, &mut ())
                .unwrap();
        assert_eq!(value, deserialized);
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn archive_btree_map_large() {
        use core::ops::ControlFlow;

        // This test creates structures too big to fit in 16-bit offsets, and
        // MIRI can't run it quickly enough.
        #[cfg(any(feature = "size_16", miri))]
        const ENTRIES: usize = 100;
        #[cfg(not(miri))]
        const ENTRIES: usize = 100_000;

        let mut value = BTreeMap::new();
        for i in 0..ENTRIES {
            value.insert(i.to_string(), i as i32);
        }

        let result = to_bytes::<Error>(&value).unwrap();
        let archived = unsafe {
            access_unchecked::<Archived<BTreeMap<String, i32>>>(
                result.as_slice(),
            )
        };

        assert_eq!(archived.len(), ENTRIES);
        let mut iter = value.iter();
        archived.visit(|ak, av| {
            let (k, v) = iter.next().unwrap();
            assert_eq!(k, ak);
            assert_eq!(v, av);
            ControlFlow::<()>::Continue(())
        });

        for (k, v) in value.iter() {
            let av = archived
                .get(k.as_str())
                .expect("failed to find key in archived B-tree map");
            assert_eq!(v, av);
        }
        assert!(archived.get("wrong!").is_none());

        let deserialized =
            deserialize::<BTreeMap<_, _>, _, Infallible>(archived, &mut ())
                .unwrap();
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
        use rkyv::ser::{
            allocator::{AllocationTracker, Arena, ArenaHandle},
            Serializer,
        };

        type TrackerSerializer<'a, E> = Strategy<
            Serializer<AlignedVec, AllocationTracker<ArenaHandle<'a>>, ()>,
            E,
        >;

        fn track_serialize<T>(value: &T) -> AllocationStats
        where
            T: for<'a> Serialize<TrackerSerializer<'a, Error>>,
        {
            let mut arena = Arena::new();

            let serializer = serialize_into(
                value,
                Serializer::new(
                    AlignedVec::new(),
                    AllocationTracker::new(arena.acquire()),
                    (),
                ),
            )
            .unwrap();
            serializer.into_raw_parts().1.into_stats()
        }

        let stats = track_serialize(&42);
        assert_eq!(stats.max_bytes_allocated, 0);
        assert_eq!(stats.max_allocations, 0);
        assert_eq!(stats.max_alignment, 1);
        assert_eq!(stats.min_arena_capacity(), 0);
        assert_eq!(stats.min_arena_capacity_max_error(), 0);

        let stats = track_serialize(&vec![1, 2, 3, 4]);
        assert_eq!(stats.max_bytes_allocated, 0);
        assert_eq!(stats.max_allocations, 0);
        assert_eq!(stats.max_alignment, 1);
        assert_eq!(stats.min_arena_capacity(), 0);
        assert_eq!(stats.min_arena_capacity_max_error(), 0);

        let stats = track_serialize(&vec![vec![1, 2], vec![3, 4]]);
        assert_ne!(stats.max_bytes_allocated, 0);
        assert_eq!(stats.max_allocations, 1);
        assert_ne!(stats.min_arena_capacity(), 0);
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn archive_manually_drop() {
        use core::mem::ManuallyDrop;

        let vec = ManuallyDrop::new(vec![
            "hello world".to_string(),
            "me too!".to_string(),
        ]);

        let bytes = to_bytes::<Error>(&vec).unwrap();
        let archived = unsafe {
            access_unchecked::<Archived<ManuallyDrop<Vec<String>>>>(&bytes)
        };

        assert_eq!(archived.len(), vec.len());
        for (a, b) in archived.iter().zip(vec.iter()) {
            assert_eq!(a, b);
        }

        drop(ManuallyDrop::into_inner(vec));
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn archive_bound() {
        use core::ops::Bound;

        test_archive(&Bound::Included("hello world".to_string()));
        test_archive(&Bound::Excluded("hello world".to_string()));
        test_archive(&Bound::<String>::Unbounded);
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn ambiguous_niched_archived_box() {
        use rkyv::{rancor::Failure, with::Niche};

        #[derive(Archive, Deserialize, Serialize)]
        struct HasNiche {
            #[with(Niche)]
            inner: Option<Box<[u32]>>,
        }

        let some_empty = HasNiche {
            inner: Some(Box::<[u32]>::from([])),
        };
        let bytes = rkyv::to_bytes::<Failure>(&some_empty).unwrap();
        eprintln!("some bytes: {:?}", bytes);
        let archived =
            unsafe { rkyv::access_unchecked::<ArchivedHasNiche>(&bytes) };
        let deser =
            deserialize::<HasNiche, _, Failure>(archived, &mut ()).unwrap();

        assert!(archived.inner.is_some());
        assert!(deser.inner.is_some());
        assert_eq!(some_empty.inner, deser.inner);
        assert_eq!(archived.inner.as_ref().unwrap().len(), 0);
        assert_eq!(deser.inner.as_ref().unwrap().len(), 0);

        let none = HasNiche { inner: None };
        let bytes = rkyv::to_bytes::<Failure>(&none).unwrap();
        eprintln!("none bytes: {:?}", bytes);
        let archived =
            unsafe { rkyv::access_unchecked::<ArchivedHasNiche>(&bytes) };
        let deser =
            deserialize::<HasNiche, _, Failure>(archived, &mut ()).unwrap();

        assert!(archived.inner.is_none());
        assert!(deser.inner.is_none());
        assert_eq!(none.inner, deser.inner);
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn reuse_arena() {
        let mut bytes = AlignedVec::with_capacity(1024);
        let mut arena = Arena::with_capacity(2);

        let value = vec![
            "hello".to_string(),
            "world".to_string(),
            "foo".to_string(),
            "bar".to_string(),
            "baz".to_string(),
        ];

        for _ in 0..10 {
            let mut buffer = core::mem::take(&mut bytes);
            buffer.clear();

            serialize_into::<_, Error>(
                &value,
                Serializer::new(buffer, arena.acquire(), Share::new()),
            )
            .unwrap();
        }
    }
}
