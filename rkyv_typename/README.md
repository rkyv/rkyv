# rkyv_typename &emsp; [![Latest Version]][crates.io] [![License]][license path] [![requires: rustc 1.54+]][Rust 1.54]

[Latest Version]: https://img.shields.io/crates/v/rkyv_typename.svg
[crates.io]: https://crates.io/crates/rkyv_typename
[License]: https://img.shields.io/badge/license-MIT-blue.svg
[license path]: https://github.com/rkyv/rkyv/blob/master/LICENSE
[requires: rustc 1.54+]: https://img.shields.io/badge/rustc-1.54+-lightgray.svg
[Rust 1.54]: https://blog.rust-lang.org/2021/07/29/Rust-1.54.0.html 

rkyv_typename adds type names for rkyv_dyn.

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
use rkyv_typename::TypeName;

#[derive(TypeName)]
#[typename = "CoolType"]
struct Example<T>(T);

fn main() {
    let mut type_name = String::new();
    Example::<i32>::build_type_name(|piece| type_name += piece);
    assert_eq!(type_name, "CoolType<i32>");
}
```
