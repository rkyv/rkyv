Trait object serialization for rkyv.

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
    archived_value,
    ser::{
        serializers::AllocSerializer,
        Serializer,
    },
    Archive,
    Archived,
    Deserialize,
    Infallible,
    Serialize,
};
use rkyv_dyn::archive_dyn;
use rkyv_typename::TypeName;

#[archive_dyn(deserialize)]
trait ExampleTrait {
    fn value(&self) -> String;
}

#[derive(Archive, Serialize, Deserialize)]
#[archive_attr(derive(TypeName))]
struct StringStruct(String);

#[archive_dyn(deserialize)]
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

#[derive(Archive, Serialize, Deserialize)]
#[archive_attr(derive(TypeName))]
struct IntStruct(i32);

#[archive_dyn(deserialize)]
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

#[test]
fn main() {
    let boxed_int = Box::new(IntStruct(42)) as Box<dyn SerializeExampleTrait>;
    let boxed_string = Box::new(StringStruct("hello world".to_string())) as Box<dyn SerializeExampleTrait>;
    let mut serializer = AllocSerializer::<256>::default();

    let int_pos = serializer.serialize_value(&boxed_int).unwrap();
    let string_pos = serializer.serialize_value(&boxed_string).unwrap();
    let buf = serializer.into_serializer().into_inner();

    let archived_int = unsafe { archived_value::<Box<dyn SerializeExampleTrait>>(buf.as_ref(), int_pos) };
    let archived_string = unsafe { archived_value::<Box<dyn SerializeExampleTrait>>(buf.as_ref(), string_pos) };
    assert_eq!(archived_int.value(), "42");
    assert_eq!(archived_string.value(), "hello world");

    let deserialized_int: Box<dyn SerializeExampleTrait> = archived_int.deserialize(&mut Infallible).unwrap();
    let deserialized_string: Box<dyn SerializeExampleTrait> = archived_string.deserialize(&mut Infallible).unwrap();
    assert_eq!(deserialized_int.value(), "42");
    assert_eq!(deserialized_string.value(), "hello world");
}
```