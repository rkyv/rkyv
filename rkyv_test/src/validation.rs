use bytecheck::CheckBytes;
use core::fmt;
use rkyv::{
    check_archived_root, check_archived_value,
    ser::{
        adapters::SharedSerializerAdapter,
        serializers::{BufferSerializer, WriteSerializer},
        Serializer,
    },
    validation::DefaultArchiveValidator,
    Aligned, AlignedVec, Archive, Serialize,
};
use std::{
    collections::{HashMap, HashSet},
    error::Error,
};

const BUFFER_SIZE: usize = 512;

fn serialize_and_check<T: Serialize<WriteSerializer<AlignedVec>>>(value: &T)
where
    T::Archived: CheckBytes<DefaultArchiveValidator>,
{
    let mut serializer = WriteSerializer::new(AlignedVec::new());
    serializer
        .serialize_value(value)
        .expect("failed to archive value");
    let buf = serializer.into_inner();
    check_archived_root::<T>(buf.as_ref()).unwrap();
}

#[test]
fn basic_functionality() {
    // Regular archiving
    let value = Some("Hello world".to_string());

    let mut serializer = WriteSerializer::new(AlignedVec::new());
    serializer
        .serialize_value(&value)
        .expect("failed to archive value");
    let buf = serializer.into_inner();

    let result = check_archived_root::<Option<String>>(buf.as_ref());
    result.unwrap();

    #[cfg(not(feature = "size_64"))]
    // Synthetic archive (correct)
    let synthetic_buf = Aligned([
        1u8, 0u8, 0u8, 0u8, // Some + padding
        8u8, 0u8, 0u8, 0u8, // points 8 bytes forward
        11u8, 0u8, 0u8, 0u8, // string is 11 characters long
        // "Hello world"
        0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x20, 0x77, 0x6f, 0x72, 0x6c, 0x64,
    ]);

    #[cfg(feature = "size_64")]
    // Synthetic archive (correct)
    let synthetic_buf = Aligned([
        1u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, // Some + padding
        16u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, // points 16 bytes forward
        11u8, 0u8, 0u8, 0u8, // string is 11 characters long
        0u8, 0u8, 0u8, 0u8, // padding
        // "Hello world"
        0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x20, 0x77, 0x6f, 0x72, 0x6c, 0x64,
    ]);

    let result = check_archived_value::<Option<String>>(synthetic_buf.as_ref(), 0);
    result.unwrap();

    // Various buffer errors:
    use rkyv::validation::{
        ArchiveBoundsError, ArchiveMemoryError, CheckArchiveError, SharedArchiveError,
    };
    // Out of bounds
    match check_archived_value::<u32>(Aligned([0, 1, 2, 3, 4]).as_ref(), 8) {
        Err(CheckArchiveError::ContextError(SharedArchiveError::Inner(
            ArchiveMemoryError::Inner(ArchiveBoundsError::OutOfBounds { .. }),
        ))) => (),
        other => panic!("expected out of bounds error, got {:?}", other),
    }
    // Overrun
    match check_archived_value::<u32>(Aligned([0, 1, 2, 3, 4]).as_ref(), 4) {
        Err(CheckArchiveError::ContextError(SharedArchiveError::Inner(
            ArchiveMemoryError::Inner(ArchiveBoundsError::Overrun { .. }),
        ))) => (),
        other => panic!("expected overrun error, got {:?}", other),
    }
    // Unaligned
    match check_archived_value::<u32>(Aligned([0, 1, 2, 3, 4]).as_ref(), 1) {
        Err(CheckArchiveError::ContextError(SharedArchiveError::Inner(
            ArchiveMemoryError::Inner(ArchiveBoundsError::Unaligned { .. }),
        ))) => (),
        other => panic!("expected unaligned error, got {:?}", other),
    }
    // Underaligned
    match check_archived_value::<u32>(&Aligned([0, 1, 2, 3, 4]).as_ref()[1..], 0) {
        Err(CheckArchiveError::ContextError(SharedArchiveError::Inner(
            ArchiveMemoryError::Inner(ArchiveBoundsError::Underaligned { .. }),
        ))) => (),
        other => panic!("expected underaligned error, got {:?}", other),
    }
}

#[test]
fn invalid_tags() {
    // Invalid archive (invalid tag)
    let synthetic_buf = Aligned([
        2u8, 0u8, 0u8, 0u8, // invalid tag + padding
        8u8, 0u8, 0u8, 0u8, // points 8 bytes forward
        11u8, 0u8, 0u8, 0u8, // string is 11 characters long
        // "Hello world"
        0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x20, 0x77, 0x6f, 0x72, 0x6c, 0x64,
    ]);

    let result = check_archived_value::<Option<String>>(synthetic_buf.as_ref(), 0);
    result.unwrap_err();
}

#[test]
fn overlapping_claims() {
    // Invalid archive (overlapping claims)
    let synthetic_buf = Aligned([
        // First string
        16u8, 0u8, 0u8, 0u8, // points 16 bytes forward
        11u8, 0u8, 0u8, 0u8, // string is 11 characters long
        // Second string
        8u8, 0u8, 0u8, 0u8, // points 8 bytes forward
        11u8, 0u8, 0u8, 0u8, // string is 11 characters long
        // "Hello world"
        0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x20, 0x77, 0x6f, 0x72, 0x6c, 0x64,
    ]);

    check_archived_value::<[String; 2]>(synthetic_buf.as_ref(), 0).unwrap_err();
}

#[test]
fn cycle_detection() {
    use rkyv::{
        validation::{ArchiveBoundsContext, ArchiveMemoryContext},
        Archived,
    };

    #[derive(Archive)]
    #[archive(derive(Debug))]
    struct NodePtr(Box<Node>);

    #[allow(dead_code)]
    #[derive(Archive)]
    #[archive(derive(Debug))]
    enum Node {
        Nil,
        Cons(#[recursive] Box<Node>),
    }

    impl<S: Serializer + ?Sized> Serialize<S> for Node {
        fn serialize(&self, serializer: &mut S) -> Result<NodeResolver, S::Error> {
            Ok(match self {
                Node::Nil => NodeResolver::Nil,
                Node::Cons(inner) => NodeResolver::Cons(inner.serialize(serializer)?),
            })
        }
    }

    #[derive(Debug)]
    struct NodeError(Box<dyn Error>);

    impl fmt::Display for NodeError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "node error: {}", self.0)
        }
    }

    impl Error for NodeError {
        fn source(&self) -> Option<&(dyn Error + 'static)> {
            Some(&*self.0)
        }
    }

    impl<C: ArchiveBoundsContext + ArchiveMemoryContext + ?Sized> CheckBytes<C> for ArchivedNode
    where
        C::Error: Error,
    {
        type Error = NodeError;

        unsafe fn check_bytes<'a>(
            value: *const Self,
            context: &mut C,
        ) -> Result<&'a Self, Self::Error> {
            let bytes = value.cast::<u8>();
            let tag = *bytes;
            match tag {
                0 => (),
                1 => {
                    <Archived<Box<Node>> as CheckBytes<C>>::check_bytes(
                        bytes.add(4).cast(),
                        context,
                    )
                    .map_err(|e| NodeError(e.into()))?;
                }
                _ => panic!(),
            }
            Ok(&*bytes.cast())
        }
    }

    // Invalid archive (cyclic claims)
    let synthetic_buf = Aligned([
        // First node
        1u8, 0u8, 0u8, 0u8, // Cons
        4u8, 0u8, 0u8, 0u8, // Node is 4 bytes forward
        // Second string
        1u8, 0u8, 0u8, 0u8, // Cons
        244u8, 255u8, 255u8, 255u8, // Node is 12 bytes back
    ]);

    check_archived_value::<Node>(synthetic_buf.as_ref(), 0).unwrap_err();
}

#[test]
fn derive_unit_struct() {
    #[derive(Archive, Serialize)]
    #[archive(derive(CheckBytes))]
    struct Test;

    serialize_and_check(&Test);
}

#[test]
fn derive_struct() {
    #[derive(Archive, Serialize)]
    #[archive(derive(CheckBytes))]
    struct Test {
        a: u32,
        b: String,
        c: Box<Vec<String>>,
    }

    serialize_and_check(&Test {
        a: 42,
        b: "hello world".to_string(),
        c: Box::new(vec!["yes".to_string(), "no".to_string()]),
    });
}

#[test]
fn derive_tuple_struct() {
    #[derive(Archive, Serialize)]
    #[archive(derive(CheckBytes))]
    struct Test(u32, String, Box<Vec<String>>);

    serialize_and_check(&Test(
        42,
        "hello world".to_string(),
        Box::new(vec!["yes".to_string(), "no".to_string()]),
    ));
}

#[test]
fn derive_enum() {
    #[derive(Archive, Serialize)]
    #[archive(derive(CheckBytes))]
    enum Test {
        A(u32),
        B(String),
        C(Box<Vec<String>>),
    }

    serialize_and_check(&Test::A(42));
    serialize_and_check(&Test::B("hello world".to_string()));
    serialize_and_check(&Test::C(Box::new(vec![
        "yes".to_string(),
        "no".to_string(),
    ])));
}

#[test]
fn hashmap() {
    let mut map = HashMap::new();
    map.insert("Hello".to_string(), 12);
    map.insert("world".to_string(), 34);
    map.insert("foo".to_string(), 56);
    map.insert("bar".to_string(), 78);
    map.insert("baz".to_string(), 90);
    serialize_and_check(&map);

    let mut set = HashSet::new();
    set.insert("Hello".to_string());
    set.insert("world".to_string());
    set.insert("foo".to_string());
    set.insert("bar".to_string());
    set.insert("baz".to_string());
    serialize_and_check(&set);
}

#[test]
fn check_dyn() {
    use rkyv::Archived;
    use rkyv_dyn::archive_dyn;
    use rkyv_typename::TypeName;

    #[archive_dyn]
    pub trait TestTrait {
        fn get_id(&self) -> i32;
    }

    #[derive(Archive, Serialize)]
    #[archive(derive(CheckBytes, TypeName))]
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

    let value: Box<dyn SerializeTestTrait> = Box::new(Test { id: 42 });

    serialize_and_check(&value);

    #[derive(Archive, Serialize)]
    #[archive(derive(TypeName))]
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
            self.id
        }
    }

    let value: Box<dyn SerializeTestTrait> = Box::new(TestUnchecked { id: 42 });

    let mut serializer = WriteSerializer::new(AlignedVec::new());
    serializer
        .serialize_value(&value)
        .expect("failed to archive value");
    let buf = serializer.into_inner();
    if let Ok(_) = check_archived_root::<Box<dyn SerializeTestTrait>>(buf.as_ref()) {
        panic!("check passed for type that does not implement CheckBytes");
    }
}

#[test]
fn check_shared_ptr() {
    use std::rc::Rc;

    #[derive(Archive, Serialize, Eq, PartialEq)]
    #[archive(derive(CheckBytes))]
    struct Test {
        a: Rc<u32>,
        b: Rc<u32>,
    }

    let shared = Rc::new(10);
    let value = Test {
        a: shared.clone(),
        b: shared.clone(),
    };

    // FIXME: A `BufferSerializer` is used here because `Seek` is required. For most purposes,
    // we should use a `Vec` and wrap it in a `Cursor` to get `Seek`. In this case,
    // `Cursor<AlignedVec>` can't implement `Write` because it's not implemented in this crate
    // so we use a buffer serializer instead.
    let mut serializer =
        SharedSerializerAdapter::new(BufferSerializer::new(Aligned([0u8; BUFFER_SIZE])));
    let pos = serializer
        .serialize_value(&value)
        .expect("failed to archive value");
    let buf = serializer.into_inner().into_inner();

    check_archived_value::<Test>(buf.as_ref(), pos).unwrap();
}
