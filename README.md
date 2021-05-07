<p align="center">
    <img src="https://raw.githubusercontent.com/djkoloski/rkyv/master/media/logo_text_color.svg">
</p>
<p align="center">
    rkyv (<em>archive</em>) is a zero-copy deserialization framework for Rust
</p>
<p align="center">
    <a href="https://discord.gg/65F6MdnbQh">
        <img src="https://img.shields.io/discord/822925794249539645" alt="Discord">
    </a>
    <a href="https://docs.rs/rkyv">
        <img src="https://img.shields.io/docsrs/rkyv.svg">
    </a>
    <a href="https://crates.io/crates/rkyv">
        <img src="https://img.shields.io/crates/v/rkyv.svg">
    </a>
    <a href="https://github.com/djkoloski/rkyv/blob/master/LICENSE">
        <img src="https://img.shields.io/badge/license-MIT-blue.svg">
    </a>
    <a href="https://blog.rust-lang.org/2020/10/08/Rust-1.50.html">
        <img src="https://img.shields.io/badge/rustc-1.50+-lightgray.svg">
    </a>
</p>

# Resources

## Learning Materials

- The [rkyv book](https://djkoloski.github.io/rkyv) covers the motivation, architecture, and major
  features of rkyv
- The [rkyv discord](https://discord.gg/65F6MdnbQh) is a great place to get help with specific issues and meet
  other people using rkyv

## Documentation

- [rkyv](https://docs.rs/rkyv), the core library
- [rkyv_dyn](https://docs.rs/rkyv_dyn), which adds trait object support to rkyv
- [rkyv_typename](https://docs.rs/rkyv_typename), a type naming library

## Benchmarks

- The [rust serialization benchmark](https://github.com/djkoloski/rust_serialization_benchmark) is a
  shootout style benchmark comparing many rust serialization solutions. It includes special
  benchmarks for zero-copy serialization solutions like rkyv.

## Sister Crates

- [bytecheck](https://github.com/djkoloski/bytecheck), which rkyv uses for validation
- [ptr_meta](https://github.com/djkoloski/ptr_meta), which rkyv uses for pointer manipulation
- [rend](https://github.com/djkoloski/rend), which rkyv uses for endian-agnostic features

# Example

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
let bytes = serializer.into_inner();

let archived = unsafe { archived_root::<Test>(&bytes[..]) };
assert_eq!(archived.int, value.int);
assert_eq!(archived.string, value.string);
assert_eq!(archived.option, value.option);

let deserialized = archived
    .deserialize(&mut AllocDeserializer)
    .expect("failed to deserialize value");
assert_eq!(deserialized, value);
```
