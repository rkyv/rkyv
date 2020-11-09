#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use archive::{
        Archive,
        ArchiveBuffer,
        Archived,
        ArchiveRef,
        Resolve,
        Write,
        WriteExt,
    };
    use archive_dyn::{
        ArchiveDyn,
        ArchivedDyn,
        ArchiveDynImpl,
        ArchiveDynImplVTable,
        DynError,
        DynResolver,
        DynWrite,
        DynWriter,
        inventory,
        TraitObject,
        vtable,
    };

    #[repr(align(16))]
    struct Aligned<T>(T);

    impl<T: AsRef<[U]>, U> AsRef<[U]> for Aligned<T> {
        fn as_ref(&self) -> &[U] {
            self.0.as_ref()
        }
    }

    impl<T: AsMut<[U]>, U> AsMut<[U]> for Aligned<T> {
        fn as_mut(&mut self) -> &mut [U] {
            self.0.as_mut()
        }
    }

    const BUFFER_SIZE: usize = 256;

    fn test_archive<T: Archive<Archived = U>, U: PartialEq<T>>(value: &T) {
        let mut writer = ArchiveBuffer::new(Aligned([0u8; BUFFER_SIZE]));
        let pos = writer.archive(value).expect("failed to archive value");
        let buf = writer.into_inner();
        let archived_value = unsafe { &*buf.as_ref().as_ptr().offset(pos as isize).cast::<U>() };
        assert!(archived_value == value);
    }

    fn test_archive_ref<T: ArchiveRef<Archived = U> + ?Sized, U: PartialEq<T> + ?Sized>(value: &T) {
        let mut writer = ArchiveBuffer::new(Aligned([0u8; BUFFER_SIZE]));
        let pos = writer.archive_ref(value).expect("failed to archive ref");
        let buf = writer.into_inner();
        let archived_ref = unsafe { &*buf.as_ref().as_ptr().offset(pos as isize).cast::<T::Reference>() };
        assert!(&**archived_ref == value);
    }

    fn test_archive_container<T: Archive<Archived = U> + core::ops::Deref<Target = TV>, TV: ?Sized, U: core::ops::Deref<Target = TU>, TU: PartialEq<TV> + ?Sized>(value: &T) {
        let mut writer = ArchiveBuffer::new(Aligned([0u8; BUFFER_SIZE]));
        let pos = writer.archive(value).expect("failed to archive ref");
        let buf = writer.into_inner();
        let archived_ref = unsafe { &*buf.as_ref().as_ptr().offset(pos as isize).cast::<U>() };
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
        let archived_value = unsafe { &*buf.as_ref().as_ptr().offset(pos as isize).cast::<Archived<HashMap<String, String>>>() };

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
        let archived_value = unsafe { &*buf.as_ref().as_ptr().offset(pos as isize).cast::<Archived<Test>>() };

        assert_eq!(archived_value, &archived_value.clone());
    }

    #[test]
    fn archived_dyn_size() {
        pub trait Test {}

        assert_eq!(core::mem::size_of::<ArchivedDyn<dyn Test>>(), 16);
    }

    #[test]
    fn archive_dyn() {
        pub trait TestTrait {
            fn get_id(&self) -> i32;
        }

        // trait macro
        pub trait ArchiveTestTrait: TestTrait + ArchiveDyn {}

        impl<T: TestTrait + ArchiveDyn> ArchiveTestTrait for T {}

        impl<'a> From<TraitObject> for &'a (dyn TestTrait + 'static) {
            fn from(trait_object: TraitObject) -> &'a (dyn TestTrait + 'static) {
                unsafe { core::mem::transmute(trait_object) }
            }
        }

        impl Resolve<dyn ArchiveTestTrait> for DynResolver {
            type Archived = ArchivedDyn<dyn TestTrait>;

            fn resolve(self, pos: usize, _value: &dyn ArchiveTestTrait) -> Self::Archived {
                ArchivedDyn::new(pos, self)
            }
        }

        impl ArchiveRef for dyn ArchiveTestTrait {
            type Archived = dyn TestTrait;
            type Reference = ArchivedDyn<dyn TestTrait>;
            type Resolver = DynResolver;

            fn archive_ref<W: Write + ?Sized>(&self, writer: &mut W) -> Result<Self::Resolver, W::Error> {
                self.archive_dyn(&mut DynWriter::new(writer)).map_err(|e| *e.downcast().unwrap())
            }
        }
        // end trait macro

        #[derive(Archive)]
        pub struct Test {
            id: i32,
        }

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

        // impl macro
        impl ArchiveDyn for Test
        where
            Test: Archive,
        {
            fn archive_dyn(&self, writer: &mut dyn DynWrite) -> Result<DynResolver, DynError> {
                Ok(DynResolver::new(
                    writer.archive(self)?,
                    ArchiveDynImpl::new::<Archived<Test>>("TestTrait")
                ))
            }
        }

        inventory::submit! {
            ArchiveDynImplVTable::new(
                ArchiveDynImpl::new::<Archived<Test>>("TestTrait"),
                vtable!(Archived<Test>, TestTrait).into()
            )
        }
        // end impl macro

        let value: Box<dyn ArchiveTestTrait> = Box::new(Test { id: 42 });

        let mut writer = ArchiveBuffer::new(Aligned([0u8; BUFFER_SIZE]));
        let pos = writer.archive(&value).expect("failed to archive value");
        let buf = writer.into_inner();
        let archived_value = unsafe { &*buf.as_ref().as_ptr().offset(pos as isize).cast::<Archived<Box<dyn ArchiveTestTrait>>>() };
        assert_eq!(value.get_id(), archived_value.get_id());

        // exercise vtable cache
        assert_eq!(value.get_id(), archived_value.get_id());
        assert_eq!(value.get_id(), archived_value.get_id());
    }
}
