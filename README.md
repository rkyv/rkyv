<p align="center">
    <img src="https://raw.githubusercontent.com/rkyv/rkyv/master/media/logo_text_color.svg" alt="rkyv">
</p>
<p align="center">
    rkyv (<em>archive</em>) is a zero-copy deserialization framework for Rust
</p>
<p align="center">
    <a href="https://discord.gg/65F6MdnbQh">
        <img src="https://img.shields.io/discord/822925794249539645" alt="Discord">
    </a>
    <a href="https://docs.rs/rkyv">
        <img src="https://img.shields.io/docsrs/rkyv.svg" alt="docs.rs">
    </a>
    <a href="https://crates.io/crates/rkyv">
        <img src="https://img.shields.io/crates/v/rkyv.svg" alt="crates.io">
    </a>
    <a href="https://github.com/rkyv/rkyv/blob/master/LICENSE">
        <img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="MIT license">
    </a>
    <a href="https://blog.rust-lang.org/2021/05/06/Rust-1.52.0.html">
        <img src="https://img.shields.io/badge/rustc-1.52+-lightgray.svg" alt="rustc 1.52+">
    </a>
</p>

# Resources

## Learning Materials

- The [rkyv book](https://rkyv.github.io/rkyv) covers the motivation, architecture, and major
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

- [bytecheck](https://github.com/rkyv/bytecheck), which rkyv uses for validation
- [ptr_meta](https://github.com/rkyv/ptr_meta), which rkyv uses for pointer manipulation
- [rend](https://github.com/rkyv/rend), which rkyv uses for endian-agnostic features

# Example

```rust
use rkyv::{
    archived_root,
    ser::{serializers::AllocSerializer, Serializer},
    Archive, Deserialize, Infallible, Serialize,
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

let mut serializer = AllocSerializer::<256>::default();
serializer.serialize_value(&value).unwrap();
let bytes = serializer.into_serializer().into_inner();

let archived = unsafe { archived_root::<Test>(&bytes[..]) };
assert_eq!(archived.int, value.int);
assert_eq!(archived.string, value.string);
assert_eq!(archived.option, value.option);

let deserialized: Test = archived.deserialize(&mut Infallible).unwrap()
assert_eq!(deserialized, value);
```

# Thanks

Thanks to all the sponsors that keep development sustainable. Special thanks to the following sponsors for going above and beyond supporting rkyv:

## Platinum Sponsors

<p align="center">
    <a href="https://dusk.network">
        <img src="https://raw.githubusercontent.com/rkyv/rkyv/master/media/sponsors/dusk_network.png" alt="Dusk Network">
    </a>
</p>

> Dusk Network is the first privacy blockchain for financial applications. Our mission is to enable any size enterprise to collaborate at scale, meet compliance requirements, and ensure that transaction data remains confidential.

## Bronze Sponsors

<p align="center">
    <a href="https://traverseresearch.nl/">
        <img src="https://raw.githubusercontent.com/rkyv/rkyv/master/media/sponsors/traverse_research.png" alt="Traverse Research">
    </a>
</p>
