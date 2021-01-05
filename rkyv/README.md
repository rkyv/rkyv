# rkyv &emsp; [![Latest Version]][crates.io] [![License]][license path] [![requires: rustc 1.47+]][Rust 1.47]

[Latest Version]: https://img.shields.io/crates/v/rkyv.svg
[crates.io]: https://crates.io/crates/rkyv
[License]: https://img.shields.io/badge/license-MIT-blue.svg
[license path]: https://github.com/djkoloski/rkyv/blob/master/LICENSE
[requires: rustc 1.47+]: https://img.shields.io/badge/rustc-1.47+-lightgray.svg
[Rust 1.47]: https://blog.rust-lang.org/2020/10/08/Rust-1.47.html

rkyv (*archive*) is a zero-copy deserialization framework for Rust.

---

You may be looking for:

- [rkyv](https://docs.rs/rkyv), the core library
- [rkyv_dyn](https://docs.rs/rkyv_dyn), which adds trait object support to rkyv
- [rkyv_typename](https://docs.rs/rkyv_typename), a type naming library

## rkyv in action

```rust
use rkyv::{Aligned, Archive, ArchiveBuffer, Archived, archived_value, Write};

#[derive(Archive)]
struct Test {
    int: u8,
    string: String,
    option: Option<Vec<i32>>,
}

fn main() {
    let mut writer = ArchiveBuffer::new(Aligned([0u8; 256]));
    let value = Test {
        int: 42,
        string: "hello world".to_string(),
        option: Some(vec![1, 2, 3, 4]),
    };
    let pos = writer.archive(&value)
        .expect("failed to archive test");
    let buf = writer.into_inner();
    let archived = unsafe { archived_value::<Test>(buf.as_ref(), pos) };
    assert_eq!(archived.int, value.int);
    assert_eq!(archived.string, value.string);
    assert_eq!(archived.option, value.option);
}
```