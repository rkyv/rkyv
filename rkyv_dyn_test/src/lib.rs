#[cfg(feature = "validation")]
mod validation;

// Miri does not support the `ctor` crate, so all of the impls here end up being unregistered.
// See: https://github.com/rust-lang/miri/issues/450
#[cfg(all(test, not(miri)))]
mod tests {
    #[cfg_attr(feature = "wasm", allow(unused_imports))]
    use core::pin::Pin;
    #[cfg_attr(feature = "wasm", allow(unused_imports))]
    use rkyv::{
        archived_root, archived_root_mut,
        ser::{serializers::AllocSerializer, Serializer},
        Archive, Archived, Deserialize, Infallible, Serialize,
    };
    #[cfg_attr(feature = "wasm", allow(unused_imports))]
    use rkyv_dyn::archive_dyn;
    #[cfg_attr(feature = "wasm", allow(unused_imports))]
    use rkyv_typename::TypeName;

    mod isolate {
        #[test]
        #[cfg(not(feature = "wasm"))]
        fn manual_archive_dyn() {
            use core::alloc::Layout;
            use rkyv::{
                archived_root,
                ser::{serializers::AllocSerializer, ScratchSpace, Serializer},
                Archive, ArchivePointee, ArchiveUnsized, Archived,
                ArchivedMetadata, Deserialize, DeserializeUnsized, Fallible,
                Infallible, Serialize, SerializeUnsized,
            };
            use rkyv_dyn::{
                register_impl, ArchivedDynMetadata, DeserializeDyn,
                DynDeserializer, DynError, RegisteredImpl, SerializeDyn,
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
            pub trait DeserializeTestTrait:
                TestTrait + DeserializeDyn<dyn SerializeTestTrait>
            {
            }

            impl<T: TestTrait + DeserializeDyn<dyn SerializeTestTrait>>
                DeserializeTestTrait for T
            {
            }

            impl TypeName for dyn DeserializeTestTrait {
                fn build_type_name<F: FnMut(&str)>(mut f: F) {
                    f("dyn DeserializeTestTrait");
                }
            }

            impl ArchiveUnsized for dyn SerializeTestTrait {
                type Archived = dyn DeserializeTestTrait;
                type MetadataResolver = ();

                unsafe fn resolve_metadata(
                    &self,
                    _: usize,
                    _: Self::MetadataResolver,
                    out: *mut ArchivedMetadata<Self>,
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

            impl<S: ScratchSpace + Serializer + ?Sized> SerializeUnsized<S>
                for dyn SerializeTestTrait
            {
                fn serialize_unsized(
                    &self,
                    mut serializer: &mut S,
                ) -> Result<usize, S::Error> {
                    self.serialize_dyn(&mut serializer)
                        .map_err(|e| *e.downcast::<S::Error>().unwrap())
                }

                fn serialize_metadata(
                    &self,
                    _: &mut S,
                ) -> Result<Self::MetadataResolver, S::Error> {
                    Ok(())
                }
            }

            impl<D> DeserializeUnsized<dyn SerializeTestTrait, D>
                for dyn DeserializeTestTrait
            where
                D: Fallible + ?Sized,
            {
                unsafe fn deserialize_unsized(
                    &self,
                    mut deserializer: &mut D,
                    mut alloc: impl FnMut(Layout) -> *mut u8,
                ) -> Result<*mut (), D::Error> {
                    self.deserialize_dyn(&mut deserializer, &mut alloc)
                        .map_err(|e| *e.downcast().unwrap())
                }

                fn deserialize_metadata(
                    &self,
                    mut deserializer: &mut D,
                ) -> Result<
                    <dyn SerializeTestTrait as ptr_meta::Pointee>::Metadata,
                    D::Error,
                > {
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
                    alloc: &mut dyn FnMut(Layout) -> *mut u8,
                ) -> Result<*mut (), DynError> {
                    let result = alloc(core::alloc::Layout::new::<Test>())
                        .cast::<Test>();
                    assert!(!result.is_null());
                    result.write(self.deserialize(deserializer)?);
                    Ok(result as *mut ())
                }

                fn deserialize_dyn_metadata(
                    &self,
                    _: &mut dyn DynDeserializer,
                ) -> Result<
                    <dyn SerializeTestTrait as ptr_meta::Pointee>::Metadata,
                    DynError,
                > {
                    unsafe {
                        Ok(core::mem::transmute(ptr_meta::metadata(
                            core::ptr::null::<Test>()
                                as *const dyn SerializeTestTrait,
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

            let mut serializer = AllocSerializer::<256>::default();
            serializer.serialize_value(&value).unwrap();
            let buf = serializer.into_serializer().into_inner();
            let archived_value = unsafe {
                archived_root::<Box<dyn SerializeTestTrait>>(buf.as_ref())
            };
            assert_eq!(value.get_id(), archived_value.get_id());

            // exercise vtable cache
            assert_eq!(value.get_id(), archived_value.get_id());
            assert_eq!(value.get_id(), archived_value.get_id());

            let deserialized_value: Box<dyn SerializeTestTrait> =
                archived_value.deserialize(&mut Infallible).unwrap();
            assert_eq!(value.get_id(), deserialized_value.get_id());
        }
    }

    #[test]
    #[cfg(not(feature = "wasm"))]
    fn archive_dyn() {
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

        let mut serializer = AllocSerializer::<256>::default();
        serializer.serialize_value(&value).unwrap();
        let buf = serializer.into_serializer().into_inner();
        let archived_value =
            unsafe { archived_root::<Box<dyn STestTrait>>(buf.as_ref()) };
        assert_eq!(value.get_id(), archived_value.get_id());

        // exercise vtable cache
        assert_eq!(value.get_id(), archived_value.get_id());
        assert_eq!(value.get_id(), archived_value.get_id());

        // deserialize
        let deserialized_value: Box<dyn STestTrait> =
            archived_value.deserialize(&mut Infallible).unwrap();
        assert_eq!(value.get_id(), deserialized_value.get_id());
        assert_eq!(value.get_id(), deserialized_value.get_id());
    }

    #[test]
    #[cfg(not(feature = "wasm"))]
    fn archive_dyn_generic() {
        use core::alloc::Layout;
        use rkyv::archived_value;
        use rkyv_dyn::archive_dyn;
        use rkyv_typename::TypeName;

        use rkyv_dyn::{
            register_impl, DynDeserializer, DynError, DynSerializer,
        };

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

        impl<T> rkyv_dyn::DeserializeDyn<dyn STestTrait<String>> for ArchivedTest<T>
        where
            T: Archive
                + for<'a> Serialize<dyn DynSerializer + 'a>
                + core::fmt::Display
                + TypeName
                + 'static,
            ArchivedTest<T>: for<'a> Deserialize<Test<T>, (dyn DynDeserializer + 'a)>
                + rkyv_dyn::RegisteredImpl<dyn DTestTrait<String>>,
        {
            unsafe fn deserialize_dyn(
                &self,
                deserializer: &mut dyn DynDeserializer,
                alloc: &mut dyn FnMut(Layout) -> *mut u8,
            ) -> Result<*mut (), DynError> {
                let result = alloc(core::alloc::Layout::new::<Test<T>>())
                    .cast::<Test<T>>();
                assert!(!result.is_null());
                result.write(self.deserialize(deserializer)?);
                Ok(result as *mut ())
            }

            fn deserialize_dyn_metadata(
                &self,
                _: &mut dyn DynDeserializer,
            ) -> Result<
                <dyn STestTrait<String> as ptr_meta::Pointee>::Metadata,
                DynError,
            > {
                unsafe {
                    Ok(core::mem::transmute(ptr_meta::metadata(
                        core::ptr::null::<Test<T>>()
                            as *const dyn STestTrait<String>,
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

        let mut serializer = AllocSerializer::<256>::default();
        let i32_pos = serializer.serialize_value(&i32_value).unwrap();
        let string_pos = serializer.serialize_value(&string_value).unwrap();
        let buf = serializer.into_serializer().into_inner();
        let i32_archived_value = unsafe {
            archived_value::<Box<dyn STestTrait<i32>>>(buf.as_ref(), i32_pos)
        };
        let string_archived_value = unsafe {
            archived_value::<Box<dyn STestTrait<String>>>(
                buf.as_ref(),
                string_pos,
            )
        };
        assert_eq!(i32_value.get_value(), i32_archived_value.get_value());
        assert_eq!(string_value.get_value(), string_archived_value.get_value());

        // exercise vtable cache
        assert_eq!(i32_value.get_value(), i32_archived_value.get_value());
        assert_eq!(i32_value.get_value(), i32_archived_value.get_value());

        let i32_deserialized_value: Box<dyn STestTrait<i32>> =
            i32_archived_value.deserialize(&mut Infallible).unwrap();
        assert_eq!(i32_value.get_value(), i32_deserialized_value.get_value());
        assert_eq!(i32_value.get_value(), i32_deserialized_value.get_value());

        assert_eq!(string_value.get_value(), string_archived_value.get_value());
        assert_eq!(string_value.get_value(), string_archived_value.get_value());

        let string_deserialized_value: Box<dyn STestTrait<String>> =
            string_archived_value.deserialize(&mut Infallible).unwrap();
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

        let mut serializer = AllocSerializer::<256>::default();
        serializer.serialize_value(&value).unwrap();
        let mut buf = serializer.into_serializer().into_inner();
        let mut value = unsafe {
            archived_root_mut::<Box<dyn SerializeTestTrait>>(Pin::new(
                buf.as_mut(),
            ))
        };

        assert_eq!(value.value(), 10);
        value.as_mut().get_pin_mut().set_value(64);
        assert_eq!(value.value(), 64);
    }
}
