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
