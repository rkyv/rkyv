#[cfg(test)]
mod tests {
    use rkyv::{
        rancor::Error, ser::writer::IoWriter, serialize, util::Align, Archive,
        Deserialize, Serialize,
    };
    #[cfg(feature = "wasm")]
    use wasm_bindgen_test::*;

    #[test]
    fn write_serializer() {
        #[derive(Archive, Serialize)]
        #[archive_attr(repr(C))]
        struct Example {
            x: i32,
        }

        let mut buf = Align([0u8; 3]);
        let mut ser = IoWriter::new(&mut buf[..]);
        let foo = Example { x: 100 };
        serialize::<_, Error>(&foo, &mut ser)
            .expect_err("serialized to an undersized buffer must fail");
    }

    // TODO: figure out errors

    // #[test]
    // fn mutex() {
    //     use rkyv::with::Lock;
    //     use std::sync::Mutex;

    //     #[derive(Archive, Serialize, Deserialize)]
    //     struct Test {
    //         #[with(Lock)]
    //         value: Mutex<i32>,
    //     }

    //     let value = Test {
    //         value: Mutex::new(10),
    //     };
    //     let mut serializer = AlignedSerializer::new(AlignedVec::new());
    //     serializer.serialize_value(&value).unwrap();
    //     let result = serializer.into_inner();
    //     let archived = unsafe { archived_root::<Test>(result.as_slice()) };

    //     assert_eq!(*archived.value, 10);

    //     let deserialized: Test = archived.deserialize(&mut
    // Infallible).unwrap();

    //     assert_eq!(*deserialized.value.lock().unwrap(), 10);
    // }

    // #[test]
    // fn rwlock() {
    //     use rkyv::with::Lock;
    //     use std::sync::RwLock;

    //     #[derive(Archive, Serialize, Deserialize)]
    //     struct Test {
    //         #[with(Lock)]
    //         value: RwLock<i32>,
    //     }

    //     let value = Test {
    //         value: RwLock::new(10),
    //     };
    //     let mut serializer = AlignedSerializer::new(AlignedVec::new());
    //     serializer.serialize_value(&value).unwrap();
    //     let result = serializer.into_inner();
    //     let archived = unsafe { archived_root::<Test>(result.as_slice()) };

    //     assert_eq!(*archived.value, 10);

    //     let deserialized: Test = archived.deserialize(&mut
    // Infallible).unwrap();

    //     assert_eq!(*deserialized.value.read().unwrap(), 10);
    // }

    // #[test]
    // fn os_string() {
    //     use rkyv::with::ToString;
    //     use core::str::FromStr;
    //     use std::ffi::OsString;

    //     #[derive(Archive, Serialize, Deserialize)]
    //     struct Test {
    //         #[with(ToString)]
    //         value: OsString,
    //     }

    //     let value = Test {
    //         value: OsString::from_str("hello world").unwrap(),
    //     };
    //     let mut serializer = AlignedSerializer::new(AlignedVec::new());
    //     serializer.serialize_value(&value).unwrap();
    //     let result = serializer.into_inner();
    //     let archived = unsafe { archived_root::<Test>(result.as_slice()) };

    //     assert_eq!(archived.value, "hello world");

    //     let deserialized: Test = archived.deserialize(&mut
    // Infallible).unwrap();

    //     assert_eq!(deserialized.value, "hello world");
    // }

    // #[test]
    // fn path_buf() {
    //     use rkyv::with::ToString;
    //     use core::str::FromStr;
    //     use std::path::PathBuf;

    //     #[derive(Archive, Serialize, Deserialize)]
    //     struct Test {
    //         #[with(ToString)]
    //         value: PathBuf,
    //     }

    //     let value = Test {
    //         value: PathBuf::from_str("hello world").unwrap(),
    //     };
    //     let mut serializer = AlignedSerializer::new(AlignedVec::new());
    //     serializer.serialize_value(&value).unwrap();
    //     let result = serializer.into_inner();
    //     let archived = unsafe { archived_root::<Test>(result.as_slice()) };

    //     assert_eq!(archived.value, "hello world");

    //     let deserialized: Test = archived.deserialize(&mut
    // Infallible).unwrap();

    //     assert_eq!(deserialized.value.to_str().unwrap(), "hello world");
    // }

    // Test case for deriving attributes on an object containing an Option. The
    // Option impls appear not to pass through attributes correctly.
    #[test]
    fn pass_thru_derive_with_option() {
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
}
