# rkyv &emsp; [![Latest Version]][crates.io] [![License]][license path] [![requires: rustc 1.50+]][Rust 1.50]

[Latest Version]: https://img.shields.io/crates/v/rkyv.svg
[crates.io]: https://crates.io/crates/rkyv
[License]: https://img.shields.io/badge/license-MIT-blue.svg
[license path]: https://github.com/djkoloski/rkyv/blob/master/LICENSE
[requires: rustc 1.50+]: https://img.shields.io/badge/rustc-1.50+-lightgray.svg
[Rust 1.50]: https://blog.rust-lang.org/2020/10/08/Rust-1.50.html

rkyv (*archive*) is a zero-copy deserialization framework for Rust.

---

## API Documentation

- [rkyv](https://docs.rs/rkyv), the core library
- [rkyv_dyn](https://docs.rs/rkyv_dyn), which adds trait object support to rkyv
- [rkyv_typename](https://docs.rs/rkyv_typename), a type naming library

## Book

- The [rkyv book](https://djkoloski.github.io/rkyv) covers the motivation and architecture of rkyv

## Sister Crates:

- [bytecheck](https://github.com/djkoloski/bytecheck), which rkyv uses for validation
- [ptr_meta](https://github.com/djkoloski/ptr_meta), which rkyv uses for pointer manipulation

---

## rkyv in action

```rust
use rkyv::{
    archived_root,
    de::deserializers::AllocDeserializer,
    ser::{serializers::AlignedSerializer, Serializer},
    AlignedVec, Archive, Deserialize, Serialize,
};

#[derive(Archive, Deserialize, Serialize, Debug, PartialEq)]
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

let mut serializer = AlignedSerializer::new(AlignedVec::new());
serializer
    .serialize_value(&value)
    .expect("failed to serialize value");
let buf = serializer.into_inner();

let archived = unsafe { archived_root::<Test>(buf.as_ref()) };
assert_eq!(archived.int, value.int);
assert_eq!(archived.string, value.string);
assert_eq!(archived.option, value.option);

let mut deserializer = AllocDeserializer;
let deserialized = archived
    .deserialize(&mut deserializer)
    .expect("failed to deserialize value");
assert_eq!(deserialized, value);
```