#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use rkyv::{
        access_unchecked, rancor::Error, ser::writer::IoWriter, serialize,
        to_bytes, util::Align, Archive, Archived, Deserialize, Serialize,
    };
    #[cfg(feature = "wasm")]
    use wasm_bindgen_test::*;

    use crate::util::alloc::*;

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
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

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn archive_hash_map() {
        let mut hash_map = HashMap::new();
        hash_map.insert("hello".to_string(), "world".to_string());
        hash_map.insert("foo".to_string(), "bar".to_string());
        hash_map.insert("baz".to_string(), "bat".to_string());

        let buf = to_bytes::<Error>(&hash_map).unwrap();
        let archived_value = unsafe {
            access_unchecked::<Archived<HashMap<String, String>>>(buf.as_ref())
        };

        assert_eq!(archived_value.len(), hash_map.len());

        for (key, value) in hash_map.iter() {
            assert!(archived_value.contains_key(key.as_str()));
            assert_eq!(&archived_value[key.as_str()], value);
        }

        for (key, value) in archived_value.iter() {
            assert!(hash_map.contains_key(key.as_str()));
            assert_eq!(&hash_map[key.as_str()], value);
        }
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn archive_hash_map_tuple_retrieved_by_get_with() {
        #[derive(Archive, Serialize, Deserialize, Eq, Hash, PartialEq)]
        #[archive_attr(derive(Eq, Hash, PartialEq))]
        pub struct Pair(String, String);

        let mut hash_map = HashMap::new();
        hash_map.insert(
            Pair("my".to_string(), "key".to_string()),
            "value".to_string(),
        );
        hash_map.insert(
            Pair("wrong".to_string(), "key".to_string()),
            "wrong value".to_string(),
        );

        let buf = to_bytes::<Error>(&hash_map).unwrap();
        let archived_value = unsafe {
            access_unchecked::<Archived<HashMap<Pair, String>>>(buf.as_ref())
        };

        let get_with = archived_value
            .get_with(&("my", "key"), |input_key, key| {
                &(key.0.as_str(), key.1.as_str()) == input_key
            })
            .unwrap();

        assert_eq!(get_with.as_str(), "value");
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    #[allow(deprecated)]
    fn archive_hash_map_hasher() {
        use std::collections::HashMap;

        test_archive(&HashMap::<i8, i32, ahash::RandomState>::default());

        let mut hash_map: HashMap<i8, _, ahash::RandomState> =
            HashMap::default();
        hash_map.insert(1, 2);
        hash_map.insert(3, 4);
        hash_map.insert(5, 6);
        hash_map.insert(7, 8);

        test_archive(&hash_map);

        let mut hash_map: HashMap<_, _, ahash::RandomState> =
            HashMap::default();
        hash_map.insert("hello".to_string(), "world".to_string());
        hash_map.insert("foo".to_string(), "bar".to_string());
        hash_map.insert("baz".to_string(), "bat".to_string());

        let buf = to_bytes::<Error>(&hash_map).unwrap();
        let archived_value = unsafe {
            access_unchecked::<
                Archived<HashMap<String, String, ahash::RandomState>>,
            >(buf.as_ref())
        };

        assert_eq!(archived_value.len(), hash_map.len());

        for (key, value) in hash_map.iter() {
            assert!(archived_value.contains_key(key.as_str()));
            assert_eq!(&archived_value[key.as_str()], value);
        }

        for (key, value) in archived_value.iter() {
            assert!(hash_map.contains_key(key.as_str()));
            assert_eq!(&hash_map[key.as_str()], value);
        }
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn archive_hash_set() {
        let mut hash_set = HashSet::new();
        hash_set.insert("hello".to_string());
        hash_set.insert("foo".to_string());
        hash_set.insert("baz".to_string());

        let buf = to_bytes::<Error>(&hash_set).unwrap();
        let archived_value = unsafe {
            access_unchecked::<Archived<HashSet<String>>>(buf.as_ref())
        };

        assert_eq!(archived_value.len(), hash_set.len());

        for key in hash_set.iter() {
            assert!(archived_value.contains(key.as_str()));
        }

        for key in archived_value.iter() {
            assert!(hash_set.contains(key.as_str()));
        }
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    #[allow(deprecated)]
    fn archive_hash_set_hasher() {
        test_archive(&HashSet::<i8, ahash::RandomState>::default());

        let mut hash_set: HashSet<i8, ahash::RandomState> = HashSet::default();
        hash_set.insert(1);
        hash_set.insert(3);
        hash_set.insert(5);
        hash_set.insert(7);

        test_archive(&hash_set);

        let mut hash_set: HashSet<_, ahash::RandomState> = HashSet::default();
        hash_set.insert("hello".to_string());
        hash_set.insert("foo".to_string());
        hash_set.insert("baz".to_string());

        let buf = to_bytes::<Error>(&hash_set).unwrap();
        let archived_value = unsafe {
            access_unchecked::<Archived<HashSet<String, ahash::RandomState>>>(
                buf.as_ref(),
            )
        };

        assert_eq!(archived_value.len(), hash_set.len());

        for key in hash_set.iter() {
            assert!(archived_value.contains(key.as_str()));
        }

        for key in archived_value.iter() {
            assert!(hash_set.contains(key.as_str()));
        }
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn archive_net() {
        use std::net::{
            IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6,
        };

        #[derive(Archive, Serialize, Deserialize, Debug, PartialEq)]
        #[archive(compare(PartialEq))]
        #[archive_attr(derive(Debug))]
        struct TestNet {
            ipv4: Ipv4Addr,
            ipv6: Ipv6Addr,
            ip: IpAddr,
            sockv4: SocketAddrV4,
            sockv6: SocketAddrV6,
            sock: SocketAddr,
        }

        let value = TestNet {
            ipv4: Ipv4Addr::new(31, 41, 59, 26),
            ipv6: Ipv6Addr::new(31, 41, 59, 26, 53, 58, 97, 93),
            ip: IpAddr::V4(Ipv4Addr::new(31, 41, 59, 26)),
            sockv4: SocketAddrV4::new(Ipv4Addr::new(31, 41, 59, 26), 5358),
            sockv6: SocketAddrV6::new(
                Ipv6Addr::new(31, 31, 59, 26, 53, 58, 97, 93),
                2384,
                0,
                0,
            ),
            sock: SocketAddr::V6(SocketAddrV6::new(
                Ipv6Addr::new(31, 31, 59, 26, 53, 58, 97, 93),
                2384,
                0,
                0,
            )),
        };

        test_archive(&value);
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn c_string() {
        use std::ffi::CString;

        let value = unsafe {
            CString::from_vec_unchecked("hello world".to_string().into_bytes())
        };
        test_archive(&value);
    }

    // TODO: figure out errors

    // #[test]
    // #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
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
    // #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
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
    // #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
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
    // #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
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

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn archive_zst_containers() {
        use std::collections::HashSet;

        #[derive(Archive, Deserialize, Serialize, Debug, PartialEq)]
        #[archive(compare(PartialEq))]
        #[archive_attr(derive(Debug))]
        struct MyZST;

        let mut value = HashMap::new();
        value.insert((), 10);
        test_archive(&value);

        let mut value = HashMap::new();
        value.insert((), ());
        test_archive(&value);

        let mut value = HashSet::new();
        value.insert(());
        test_archive(&value);
    }

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
