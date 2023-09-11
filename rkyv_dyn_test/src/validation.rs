#[cfg(test)]
mod tests {
    #[cfg_attr(feature = "wasm", allow(unused_imports))]
    use bytecheck::CheckBytes;
    #[cfg_attr(feature = "wasm", allow(unused_imports))]
    use rkyv::{
        check_archived_root,
        ser::{serializers::AllocSerializer, Serializer},
        validation::validators::DefaultValidator,
        Archive, Archived, Serialize,
    };
    #[cfg_attr(feature = "wasm", allow(unused_imports))]
    use rkyv_dyn::archive_dyn;
    #[cfg_attr(feature = "wasm", allow(unused_imports))]
    use rkyv_typename::TypeName;

    #[cfg_attr(feature = "wasm", allow(dead_code))]
    fn serialize_and_check<T: Serialize<AllocSerializer<256>>>(value: &T)
    where
        T::Archived: for<'a> CheckBytes<DefaultValidator<'a>>,
    {
        let mut serializer = AllocSerializer::<256>::default();
        serializer.serialize_value(value).unwrap();
        let buf = serializer.into_serializer().into_inner();
        check_archived_root::<T>(buf.as_ref()).unwrap();
    }

    #[test]
    #[cfg(not(feature = "wasm"))]
    #[cfg_attr(miri, ignore = "miri does not support ctor, see lib.rs")]
    fn check_dyn() {
        #[archive_dyn]
        pub trait TestTrait {
            fn get_id(&self) -> i32;
        }

        #[derive(Archive, Serialize)]
        #[archive_attr(derive(CheckBytes, TypeName))]
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
                self.id.into()
            }
        }

        let value: Box<dyn SerializeTestTrait> = Box::new(Test { id: 42 });

        serialize_and_check(&value);

        #[derive(Archive, Serialize)]
        #[archive_attr(derive(TypeName))]
        pub struct TestUnchecked {
            id: i32,
        }

        #[archive_dyn]
        impl TestTrait for TestUnchecked {
            fn get_id(&self) -> i32 {
                self.id
            }
        }

        impl TestTrait for Archived<TestUnchecked> {
            fn get_id(&self) -> i32 {
                self.id.into()
            }
        }

        let value: Box<dyn SerializeTestTrait> =
            Box::new(TestUnchecked { id: 42 });

        let mut serializer = AllocSerializer::<256>::default();
        serializer.serialize_value(&value).unwrap();
        let buf = serializer.into_serializer().into_inner();
        if let Ok(_) =
            check_archived_root::<Box<dyn SerializeTestTrait>>(buf.as_ref())
        {
            panic!("check passed for type that does not implement CheckBytes");
        }
    }
}
