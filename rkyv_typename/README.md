# rkyv_typename &emsp; [![Latest Version]][crates.io] [![License]][license path]

[Latest Version]: https://img.shields.io/crates/v/rkyv_typename.svg
[crates.io]: https://crates.io/crates/rkyv_typename
[License]: https://img.shields.io/badge/license-MIT-blue.svg
[license path]: https://github.com/djkoloski/rkyv/blob/master/LICENSE

rkyv_typename adds type names for rkyv_dyn.

---

You may be looking for:

- [rkyv](https://docs.rs/rkyv), the core library
- [rkyv_dyn](https://docs.rs/rkyv_dyn), which adds trait object support to rkyv
- [rkyv_typename](https://docs.rs/rkyv_typename), a type naming library

## rkyv_typename in action

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