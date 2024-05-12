// TODO: uncomment and fix
// #[cfg(feature = "bytecheck")]
// mod validation;

#[cfg(test)]
mod tests {
    // #[cfg_attr(feature = "wasm", allow(unused_imports))]
    // use core::pin::Pin;
    // #[cfg_attr(feature = "wasm", allow(unused_imports))]
    // use rkyv::{
    //     access_unchecked, access_unchecked_mut,
    //     ser::{serializers::AllocSerializer, Serializer},
    //     Archive, Archived, Deserialize, Serialize,
    // };
    // #[cfg_attr(feature = "wasm", allow(unused_imports))]
    // use rkyv_dyn::archive_dyn;

    mod isolate {
        #[test]
        #[cfg(not(feature = "wasm"))]
        fn manual_archive_dyn() {
            use ptr_meta::{DynMetadata, Pointee};
            use rkyv::{
                access_unchecked,
                de::pooling::Pool,
                deserialize,
                rancor::{Error, Fallible, Strategy},
                to_bytes, Archive, ArchivePointee, ArchiveUnsized, Archived,
                ArchivedMetadata, Deserialize, DeserializeUnsized, LayoutRaw,
                Portable, Serialize, SerializeUnsized,
            };
            use rkyv_dyn::{
                register_trait_impls, ArchivedDynMetadata, AsDynDeserializer,
                AsDynSerializer, DeserializeDyn, DynDeserializer,
                DynSerializer, ImplId, RegisteredImpl, SerializeDyn,
            };

            pub trait Id {
                fn get_id(&self) -> i32;
            }

            #[ptr_meta::pointee]
            pub trait SerializeId<SE, DE>: Id + SerializeDyn<SE> {
                fn archived_impl_id(&self) -> ImplId;
            }

            impl<T, SE, DE> SerializeId<SE, DE> for T
            where
                T: Id + for<'a> Serialize<dyn DynSerializer<SE> + 'a>,
                T::Archived: RegisteredImpl<dyn DeserializeId<SE, DE>>,
            {
                fn archived_impl_id(&self) -> ImplId {
                    T::Archived::IMPL_ID
                }
            }

            impl<SE, DE> ArchiveUnsized for dyn SerializeId<SE, DE> {
                type Archived = dyn DeserializeId<SE, DE>;

                fn archived_metadata(&self) -> ArchivedMetadata<Self> {
                    ArchivedDynMetadata::new(self.archived_impl_id())
                }
            }

            impl<SE, DE> LayoutRaw for dyn SerializeId<SE, DE> {
                fn layout_raw(
                    metadata: <Self as Pointee>::Metadata,
                ) -> Result<core::alloc::Layout, core::alloc::LayoutError>
                {
                    Ok(metadata.layout())
                }
            }

            impl<S, DE> SerializeUnsized<S> for dyn SerializeId<S::Error, DE>
            where
                S: Fallible + AsDynSerializer<S::Error> + ?Sized,
            {
                fn serialize_unsized(
                    &self,
                    serializer: &mut S,
                ) -> Result<usize, S::Error> {
                    self.serialize_and_resolve_dyn(
                        serializer.as_dyn_serializer(),
                    )
                }
            }

            #[ptr_meta::pointee]
            pub trait DeserializeId<SE, DE>:
                Id
                + DeserializeDyn<dyn SerializeId<SE, DE>, DE>
                + Portable
            {
            }

            impl<SE, DE> ArchivePointee for dyn DeserializeId<SE, DE> {
                type ArchivedMetadata = ArchivedDynMetadata<Self>;

                fn pointer_metadata(
                    archived: &Self::ArchivedMetadata,
                ) -> <Self as Pointee>::Metadata {
                    archived.lookup_metadata()
                }
            }

            impl<T, SE, DE> DeserializeId<SE, DE> for T where
                T: Id + DeserializeDyn<dyn SerializeId<SE, DE>, DE> + Portable
            {
            }

            impl<SE, D> DeserializeUnsized<dyn SerializeId<SE, D::Error>, D>
                for dyn DeserializeId<SE, D::Error>
            where
                D: Fallible + AsDynDeserializer<D::Error> + ?Sized,
            {
                unsafe fn deserialize_unsized(
                    &self,
                    deserializer: &mut D,
                    out: *mut dyn SerializeId<SE, D::Error>,
                ) -> Result<(), <D as Fallible>::Error> {
                    self.deserialize_dyn(
                        deserializer.as_dyn_deserializer(),
                        out,
                    )
                }

                fn deserialize_metadata(
                    &self,
                    _: &mut D,
                ) -> Result<
                    <dyn SerializeId<SE, D::Error> as ptr_meta::Pointee>
                        ::Metadata,
                    <D as Fallible>::Error,
                >{
                    Ok(self.deserialized_pointer_metadata())
                }
            }

            #[derive(Archive, Serialize, Deserialize)]
            pub struct Test {
                id: i32,
            }

            impl Id for Test {
                fn get_id(&self) -> i32 {
                    self.id
                }
            }

            register_trait_impls! {
                Archived<Test> as dyn DeserializeId<Error, Error>,
            }

            // Hack to make rustfmt happy
            type SId<SE, DE> = dyn SerializeId<SE, DE>;
            impl<SE, DE> DeserializeDyn<SId<SE, DE>, DE> for ArchivedTest
            where
                Archived<Test>:
                    for<'a> Deserialize<Test, dyn DynDeserializer<DE> + 'a>,
                Archived<Test>: RegisteredImpl<dyn DeserializeId<SE, DE>>,
            {
                fn deserialize_dyn(
                    &self,
                    deserializer: &mut dyn DynDeserializer<DE>,
                    out: *mut dyn SerializeId<SE, DE>,
                ) -> Result<(), DE> {
                    unsafe {
                        <
                            Self as DeserializeUnsized<Test, _>
                        >::deserialize_unsized(self, deserializer, out.cast())
                    }
                }

                fn deserialized_pointer_metadata(
                    &self,
                ) -> DynMetadata<dyn SerializeId<SE, DE>> {
                    ptr_meta::metadata(core::ptr::null::<Test>()
                        as *const dyn SerializeId<SE, DE>)
                }
            }

            impl Id for ArchivedTest {
                fn get_id(&self) -> i32 {
                    self.id.into()
                }
            }

            let value: Box<dyn SerializeId<Error, Error>> =
                Box::new(Test { id: 42 });

            let buf = to_bytes::<_>(&value).unwrap();
            type ArchivedType = Archived<Box<dyn SerializeId<Error, Error>>>;
            let archived_value =
                unsafe { access_unchecked::<ArchivedType>(buf.as_ref()) };
            assert_eq!(value.get_id(), archived_value.get_id());

            // exercise vtable cache
            assert_eq!(value.get_id(), archived_value.get_id());
            assert_eq!(value.get_id(), archived_value.get_id());

            let deserialized_value: Box<dyn SerializeId<Error, Error>> =
                deserialize::<Box<dyn SerializeId<Error, Error>>, _, Error>(
                    archived_value,
                    Strategy::wrap(&mut Pool::new()),
                )
                .unwrap();
            assert_eq!(value.get_id(), deserialized_value.get_id());
        }
    }

    // TODO: uncomment and fix
    // #[test]
    // #[cfg(not(feature = "wasm"))]
    // fn archive_dyn() {
    //     #[archive_dyn(serialize = "STestTrait", deserialize = "DTestTrait")]
    //     pub trait TestTrait {
    //         fn get_id(&self) -> i32;
    //     }

    //     #[derive(Archive, Serialize, Deserialize)]
    //     #[archive_attr(derive(TypeName))]
    //     pub struct Test {
    //         id: i32,
    //     }

    //     #[archive_dyn(serialize = "STestTrait", deserialize = "DTestTrait")]
    //     impl TestTrait for Test {
    //         fn get_id(&self) -> i32 {
    //             self.id
    //         }
    //     }

    //     impl TestTrait for ArchivedTest {
    //         fn get_id(&self) -> i32 {
    //             self.id.into()
    //         }
    //     }

    //     let value: Box<dyn STestTrait> = Box::new(Test { id: 42 });

    //     let mut serializer = AllocSerializer::<256>::default();
    //     serializer.serialize_value(&value).unwrap();
    //     let buf = serializer.into_serializer().into_inner();
    //     let archived_value =
    //         unsafe { archived_root::<Box<dyn STestTrait>>(buf.as_ref()) };
    //     assert_eq!(value.get_id(), archived_value.get_id());

    //     // exercise vtable cache
    //     assert_eq!(value.get_id(), archived_value.get_id());
    //     assert_eq!(value.get_id(), archived_value.get_id());

    //     // deserialize
    //     let deserialized_value: Box<dyn STestTrait> =
    //         archived_value.deserialize(&mut Infallible).unwrap();
    //     assert_eq!(value.get_id(), deserialized_value.get_id());
    //     assert_eq!(value.get_id(), deserialized_value.get_id());
    // }

    // #[test]
    // #[cfg(not(feature = "wasm"))]
    // fn archive_dyn_generic() {
    //     use core::alloc::Layout;
    //     use rkyv::archived_value;
    //     use rkyv_dyn::archive_dyn;
    //     use rkyv_typename::TypeName;

    //     use rkyv_dyn::{
    //         trait_impl, DynDeserializer, DynError, DynSerializer,
    //     };

    //     #[archive_dyn(serialize = "STestTrait", deserialize = "DTestTrait")]
    //     pub trait TestTrait<T: TypeName> {
    //         fn get_value(&self) -> T;
    //     }

    //     #[derive(Archive, Serialize, Deserialize)]
    //     #[archive_attr(derive(TypeName))]
    //     pub struct Test<T> {
    //         value: T,
    //     }

    //     #[archive_dyn(serialize = "STestTrait", deserialize = "DTestTrait")]
    //     impl TestTrait<i32> for Test<i32> {
    //         fn get_value(&self) -> i32 {
    //             self.value
    //         }
    //     }

    //     impl TestTrait<i32> for ArchivedTest<i32> {
    //         fn get_value(&self) -> i32 {
    //             self.value.into()
    //         }
    //     }

    //     impl<T: core::fmt::Display> TestTrait<String> for Test<T> {
    //         fn get_value(&self) -> String {
    //             format!("{}", self.value)
    //         }
    //     }

    //     impl<T> rkyv_dyn::DeserializeDyn<dyn STestTrait<String>> for
    // ArchivedTest<T>     where
    //         T: Archive
    //             + for<'a> Serialize<dyn DynSerializer + 'a>
    //             + core::fmt::Display
    //             + TypeName
    //             + 'static,
    //         ArchivedTest<T>: for<'a> Deserialize<Test<T>, (dyn
    // DynDeserializer + 'a)>
    //             + rkyv_dyn::RegisteredImpl<dyn DTestTrait<String>>,
    //     {
    //         unsafe fn deserialize_dyn(
    //             &self,
    //             deserializer: &mut dyn DynDeserializer,
    //             alloc: &mut dyn FnMut(Layout) -> *mut u8,
    //         ) -> Result<*mut (), DynError> {
    //             let result = alloc(core::alloc::Layout::new::<Test<T>>())
    //                 .cast::<Test<T>>();
    //             assert!(!result.is_null());
    //             result.write(self.deserialize(deserializer)?);
    //             Ok(result as *mut ())
    //         }

    //         fn deserialize_dyn_metadata(
    //             &self,
    //             _: &mut dyn DynDeserializer,
    //         ) -> Result<
    //             <dyn STestTrait<String> as ptr_meta::Pointee>::Metadata,
    //             DynError,
    //         > { unsafe { Ok(core::mem::transmute(ptr_meta::metadata(
    //         > core::ptr::null::<Test<T>>() as *const dyn STestTrait<String>,
    //         > ))) }
    //         }
    //     }

    //     impl<T: Archive> TestTrait<String> for ArchivedTest<T>
    //     where
    //         T::Archived: core::fmt::Display,
    //     {
    //         fn get_value(&self) -> String {
    //             format!("{}", self.value)
    //         }
    //     }

    //     trait_impl!(Archived<Test<String>> as dyn DTestTrait<String>);

    //     let i32_value: Box<dyn STestTrait<i32>> = Box::new(Test { value: 42
    // });     let string_value: Box<dyn STestTrait<String>> = Box::new(Test
    // {         value: "hello world".to_string(),
    //     });

    //     let mut serializer = AllocSerializer::<256>::default();
    //     let i32_pos = serializer.serialize_value(&i32_value).unwrap();
    //     let string_pos = serializer.serialize_value(&string_value).unwrap();
    //     let buf = serializer.into_serializer().into_inner();
    //     let i32_archived_value = unsafe {
    //         archived_value::<Box<dyn STestTrait<i32>>>(buf.as_ref(), i32_pos)
    //     };
    //     let string_archived_value = unsafe {
    //         archived_value::<Box<dyn STestTrait<String>>>(
    //             buf.as_ref(),
    //             string_pos,
    //         )
    //     };
    //     assert_eq!(i32_value.get_value(), i32_archived_value.get_value());
    //     assert_eq!(string_value.get_value(),
    // string_archived_value.get_value());

    //     // exercise vtable cache
    //     assert_eq!(i32_value.get_value(), i32_archived_value.get_value());
    //     assert_eq!(i32_value.get_value(), i32_archived_value.get_value());

    //     let i32_deserialized_value: Box<dyn STestTrait<i32>> =
    //         i32_archived_value.deserialize(&mut Infallible).unwrap();
    //     assert_eq!(i32_value.get_value(),
    // i32_deserialized_value.get_value());     assert_eq!(i32_value.
    // get_value(), i32_deserialized_value.get_value());

    //     assert_eq!(string_value.get_value(),
    // string_archived_value.get_value());     assert_eq!(string_value.
    // get_value(), string_archived_value.get_value());

    //     let string_deserialized_value: Box<dyn STestTrait<String>> =
    //         string_archived_value.deserialize(&mut Infallible).unwrap();
    //     assert_eq!(
    //         string_value.get_value(),
    //         string_deserialized_value.get_value()
    //     );
    //     assert_eq!(
    //         string_value.get_value(),
    //         string_deserialized_value.get_value()
    //     );
    // }

    // #[test]
    // #[cfg(not(feature = "wasm"))]
    // fn mutable_dyn_ref() {
    //     use rkyv_dyn::archive_dyn;
    //     use rkyv_typename::TypeName;

    //     #[archive_dyn]
    //     trait TestTrait {
    //         fn value(&self) -> i32;
    //         fn set_value(self: Pin<&mut Self>, value: i32);
    //     }

    //     #[derive(Archive, Serialize)]
    //     #[archive_attr(derive(TypeName))]
    //     struct Test(i32);

    //     #[archive_dyn]
    //     impl TestTrait for Test {
    //         fn value(&self) -> i32 {
    //             self.0
    //         }
    //         fn set_value(self: Pin<&mut Self>, value: i32) {
    //             unsafe {
    //                 let s = self.get_unchecked_mut();
    //                 s.0 = value;
    //             }
    //         }
    //     }

    //     impl TestTrait for ArchivedTest {
    //         fn value(&self) -> i32 {
    //             self.0.into()
    //         }
    //         fn set_value(self: Pin<&mut Self>, value: i32) {
    //             unsafe {
    //                 let s = self.get_unchecked_mut();
    //                 s.0 = value.into();
    //             }
    //         }
    //     }

    //     let value = Box::new(Test(10)) as Box<dyn SerializeTestTrait>;

    //     let mut serializer = AllocSerializer::<256>::default();
    //     serializer.serialize_value(&value).unwrap();
    //     let mut buf = serializer.into_serializer().into_inner();
    //     let mut value = unsafe {
    //         archived_root_mut::<Box<dyn SerializeTestTrait>>(Pin::new(
    //             buf.as_mut(),
    //         ))
    //     };

    //     assert_eq!(value.value(), 10);
    //     value.as_mut().get_pin_mut().set_value(64);
    //     assert_eq!(value.value(), 64);
    // }
}
