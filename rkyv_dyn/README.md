# rkyv_dyn &emsp; [![Latest Version]][crates.io] [![License]][license path] [![requires: rustc 1.47+]][Rust 1.47]

[Latest Version]: https://img.shields.io/crates/v/rkyv_dyn.svg
[crates.io]: https://crates.io/crates/rkyv_dyn
[License]: https://img.shields.io/badge/license-MIT-blue.svg
[license path]: https://github.com/djkoloski/rkyv/blob/master/LICENSE
[requires: rustc 1.47+]: https://img.shields.io/badge/rustc-1.47+-lightgray.svg
[Rust 1.47]: https://blog.rust-lang.org/2020/10/08/Rust-1.47.html

Trait object serialization for rkyv.

---

## API Documentation

- [rkyv](https://docs.rs/rkyv), the core library
- [rkyv_dyn](https://docs.rs/rkyv_dyn), which adds trait object support to rkyv
- [rkyv_typename](https://docs.rs/rkyv_typename), a type naming library

## Book

- The [rkyv book](https://djkoloski.github.io/rkyv) covers the motivation and architecture of rkyv

## Sister Crates:

- [bytecheck](https://github.com/djkoloski/bytecheck), which rkyv uses for validation

---

## rkyv_dyn in action

```rust
use rkyv::{
    Aligned,
    Archive,
    ArchiveBuffer,
    Archived,
    archived_value,
    Write,
};
use rkyv_dyn::archive_dyn;
use rkyv_typename::TypeName;

#[archive_dyn]
trait ExampleTrait {
    fn value(&self) -> String;
}

#[derive(Archive)]
#[archive(derive(TypeName))]
struct StringStruct(String);

#[archive_dyn]
impl ExampleTrait for StringStruct {
    fn value(&self) -> String {
        self.0.clone()
    }
}

impl ExampleTrait for Archived<StringStruct> {
    fn value(&self) -> String {
        self.0.as_str().to_string()
    }
}

#[derive(Archive)]
#[archive(derive(TypeName))]
struct IntStruct(i32);

#[archive_dyn]
impl ExampleTrait for IntStruct {
    fn value(&self) -> String {
        format!("{}", self.0)
    }
}

impl ExampleTrait for Archived<IntStruct> {
    fn value(&self) -> String {
        format!("{}", self.0)
    }
}

fn main() {
    let boxed_int = Box::new(IntStruct(42)) as Box<dyn ArchiveExampleTrait>;
    let boxed_string = Box::new(StringStruct("hello world".to_string())) as Box<dyn ArchiveExampleTrait>;

    let mut writer = ArchiveBuffer::new(Aligned([0u8; 256]));
    let int_pos = writer.archive(&boxed_int)
        .expect("failed to archive boxed int");
    let string_pos = writer.archive(&boxed_string)
        .expect("failed to archive boxed string");
    let buf = writer.into_inner();

    let archived_int = unsafe { archived_value::<Box<dyn ArchiveExampleTrait>>(buf.as_ref(), int_pos) };
    let archived_string = unsafe { archived_value::<Box<dyn ArchiveExampleTrait>>(buf.as_ref(), string_pos) };
    assert_eq!(archived_int.value(), "42");
    assert_eq!(archived_string.value(), "hello world");
}
```