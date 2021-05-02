<p align="center">
    <img src="data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='270.33' height='85.917' viewBox='0 0 71.525 22.732' xmlns:v='https://vecta.io/nano'%3E%3Cpath d='M27.768 8.963h-.346q-.249-.092-.8-.139-.552-.046-.919-.046-.833 0-1.471.116-.638.116-1.374.393v8.686h-3.894V5.001h3.894v1.906q1.287-1.178 2.239-1.559.952-.393 1.752-.393.206 0 .465.012.26.012.454.035zm15.123 9.01h-4.532l-3.407-5.648-1.06 1.375v4.274h-3.894V0h3.894v10.731l4.197-5.729h4.489l-4.37 5.591zm7.475-4.586l2.769-8.386h4.002l-6.75 17.731h-4.219l1.925-4.851-4.727-12.879h4.089zm21.158-8.386L66.84 17.973h-4.413L57.776 5.001h4.121l2.813 8.917 2.78-8.917zM0 3.418V17.97h14.552V3.418zm5.046 2.075l4.459 4.458 1.486-1.486 1.486 7.431-7.431-1.486 1.487-1.486-4.459-4.459z'/%3E%3C/svg%3E">
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
    <a href="https://blog.rust-lang.org/2020/10/08/Rust-1.47.html">
        <img src="https://img.shields.io/badge/rustc-1.47+-lightgray.svg">
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
