#[cfg(test)]
mod tests {
    use crate::validation::util::serialize_and_check;
    #[cfg(all(feature = "alloc", not(feature = "std")))]
    use alloc::{
        boxed::Box,
        rc::Rc,
        string::{String, ToString},
        vec,
        vec::Vec,
    };
    use bytecheck::{CheckBytes, Error};
    use core::fmt;
    use rkyv::{
        check_archived_root, check_archived_value,
        ser::{
            adapters::SharedSerializerAdapter,
            serializers::{AlignedSerializer, BufferSerializer},
            Serializer,
        },
        Aligned, AlignedVec, Archive, Serialize,
    };
    #[cfg(feature = "std")]
    use std::rc::Rc;

    #[cfg(feature = "wasm")]
    use wasm_bindgen_test::*;

    const BUFFER_SIZE: usize = 512;

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn basic_functionality() {
        // Regular archiving
        let value = Some("Hello world".to_string());

        let mut serializer = AlignedSerializer::new(AlignedVec::new());
        serializer
            .serialize_value(&value)
            .expect("failed to archive value");
        let buf = serializer.into_inner();

        let result = check_archived_root::<Option<String>>(buf.as_ref());
        result.unwrap();

        #[cfg(feature = "size_16")]
        #[cfg(any(
            all(target_endian = "little", not(feature = "archive_be")),
            feature = "archive_le"
        ))]
        // Synthetic archive (correct)
        let synthetic_buf = Aligned([
            1u8, 0u8, // Some + padding
            4u8, 0u8, // points 4 bytes forward
            11u8, 0u8, // string is 11 characters long
            // "Hello world"
            0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x20, 0x77, 0x6f, 0x72, 0x6c, 0x64,
        ]);

        #[cfg(feature = "size_16")]
        #[cfg(any(
            all(target_endian = "big", not(feature = "archive_le")),
            feature = "archive_be"
        ))]
        // Synthetic archive (correct)
        let synthetic_buf = Aligned([
            1u8, 0u8, // Some + padding
            0u8, 4u8, // points 4 bytes forward
            0u8, 11u8, // string is 11 characters long
            // "Hello world"
            0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x20, 0x77, 0x6f, 0x72, 0x6c, 0x64,
        ]);

        #[cfg(feature = "size_32")]
        #[cfg(any(
            all(target_endian = "little", not(feature = "archive_be")),
            feature = "archive_le"
        ))]
        // Synthetic archive (correct)
        let synthetic_buf = Aligned([
            1u8, 0u8, 0u8, 0u8, // Some + padding
            8u8, 0u8, 0u8, 0u8, // points 8 bytes forward
            11u8, 0u8, 0u8, 0u8, // string is 11 characters long
            // "Hello world"
            0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x20, 0x77, 0x6f, 0x72, 0x6c, 0x64,
        ]);

        #[cfg(feature = "size_32")]
        #[cfg(any(
            all(target_endian = "big", not(feature = "archive_le")),
            feature = "archive_be"
        ))]
        // Synthetic archive (correct)
        let synthetic_buf = Aligned([
            1u8, 0u8, 0u8, 0u8, // Some + padding
            0u8, 0u8, 0u8, 8u8, // points 8 bytes forward
            0u8, 0u8, 0u8, 11u8, // string is 11 characters long
            // "Hello world"
            0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x20, 0x77, 0x6f, 0x72, 0x6c, 0x64,
        ]);

        #[cfg(feature = "size_64")]
        #[cfg(any(
            all(target_endian = "little", not(feature = "archive_be")),
            feature = "archive_le"
        ))]
        // Synthetic archive (correct)
        let synthetic_buf = Aligned([
            1u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, // Some + padding
            16u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, // points 16 bytes forward
            11u8, 0u8, 0u8, 0u8, // string is 11 characters long
            0u8, 0u8, 0u8, 0u8, // padding
            // "Hello world"
            0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x20, 0x77, 0x6f, 0x72, 0x6c, 0x64,
        ]);

        #[cfg(feature = "size_64")]
        #[cfg(any(
            all(target_endian = "big", not(feature = "archive_le")),
            feature = "archive_be"
        ))]
        // Synthetic archive (correct)
        let synthetic_buf = Aligned([
            1u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, // Some + padding
            0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 16u8, // points 16 bytes forward
            0u8, 0u8, 0u8, 11u8, // string is 11 characters long
            0u8, 0u8, 0u8, 0u8, // padding
            // "Hello world"
            0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x20, 0x77, 0x6f, 0x72, 0x6c, 0x64,
        ]);

        let result = check_archived_value::<Option<String>>(synthetic_buf.as_ref(), 0);
        result.unwrap();

        // Various buffer errors:
        use rkyv::validation::{
            validators::{ArchiveBoundsError, ArchiveMemoryError, SharedArchiveError},
            CheckArchiveError,
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

    #[cfg(feature = "size_32")]
    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
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

    #[cfg(feature = "size_32")]
    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
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

    #[cfg(feature = "size_32")]
    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn cycle_detection() {
        use rkyv::{
            validation::{ArchiveBoundsContext, ArchiveMemoryContext},
            Archived,
        };

        #[derive(Archive)]
        #[archive_attr(derive(Debug))]
        struct NodePtr(Box<Node>);

        #[allow(dead_code)]
        #[derive(Archive)]
        #[archive_attr(derive(Debug))]
        enum Node {
            Nil,
            Cons(#[omit_bounds] Box<Node>),
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

        #[cfg(feature = "std")]
        const _: () = {
            use std::error::Error;

            impl Error for NodeError {
                fn source(&self) -> Option<&(dyn Error + 'static)> {
                    Some(self.0.as_error())
                }
            }
        };

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
                        .map_err(|e| NodeError(Box::new(e)))?;
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
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn derive_unit_struct() {
        #[derive(Archive, Serialize)]
        #[archive_attr(derive(CheckBytes))]
        struct Test;

        serialize_and_check(&Test);
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn derive_struct() {
        #[derive(Archive, Serialize)]
        #[archive_attr(derive(CheckBytes))]
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
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn derive_tuple_struct() {
        #[derive(Archive, Serialize)]
        #[archive_attr(derive(CheckBytes))]
        struct Test(u32, String, Box<Vec<String>>);

        serialize_and_check(&Test(
            42,
            "hello world".to_string(),
            Box::new(vec!["yes".to_string(), "no".to_string()]),
        ));
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn derive_enum() {
        #[derive(Archive, Serialize)]
        #[archive_attr(derive(CheckBytes))]
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
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn recursive_type() {
        #[derive(Archive, Serialize)]
        // The derive macros don't apply the right bounds from Box so we have to manually specify
        // what bounds to apply
        #[archive(bound(serialize = "__S: Serializer", deserialize = "__D: Deserializer"))]
        #[archive_attr(derive(CheckBytes))]
        #[archive_attr(check_bytes(
            bound = "__C: ::rkyv::validation::ArchiveBoundsContext + ::rkyv::validation::ArchiveMemoryContext, <__C as ::rkyv::Fallible>::Error: ::bytecheck::Error"
        ))]
        enum Node {
            Nil,
            Cons(
                #[omit_bounds]
                #[archive_attr(omit_bounds)]
                Box<Node>,
            ),
        }

        serialize_and_check(&Node::Cons(Box::new(Node::Cons(Box::new(Node::Nil)))));
    }

    #[test]
    fn check_shared_ptr() {
        #[derive(Archive, Serialize, Eq, PartialEq)]
        #[archive_attr(derive(CheckBytes))]
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
        let buffer = serializer.into_inner().into_inner();

        check_archived_value::<Test>(buffer.as_ref(), pos).unwrap();
    }
}
