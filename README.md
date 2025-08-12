<p align="center">
    <img src="https://raw.githubusercontent.com/rkyv/rkyv/master/media/logo_text_color.svg" alt="rkyv">
</p>
<p align="center">
    rkyv (<em>archive</em>) is a zero-copy deserialization framework for Rust
</p>
<p align="center">
    <a href="https://discord.gg/65F6MdnbQh"><img src="https://img.shields.io/discord/822925794249539645" alt="Discord"></a>
    <a href="https://crates.io/crates/rkyv"><img src="https://img.shields.io/crates/v/rkyv.svg" alt="crates.io"></a>
    <a href="https://docs.rs/rkyv"><img src="https://img.shields.io/docsrs/rkyv.svg" alt="docs.rs"></a>
    <a href="https://github.com/rkyv/rkyv/blob/master/LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="MIT license"></a>
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

## Benchmarks

- The [rust serialization benchmark](https://github.com/djkoloski/rust_serialization_benchmark) is a
  shootout style benchmark comparing many rust serialization solutions. It includes special
  benchmarks for zero-copy serialization solutions like rkyv.

## Sister Crates

- [rend](https://github.com/rkyv/rend), which rkyv uses for endian-agnostic features
- [bytecheck](https://github.com/rkyv/bytecheck), which rkyv uses for validation
- [rancor](https://github.com/rkyv/rancor), which rkyv uses for error handling
- [ptr_meta](https://github.com/rkyv/ptr_meta), which rkyv uses for pointer manipulation

# Example

```rust
use rkyv::{deserialize, rancor::Error, Archive, Deserialize, Serialize};

#[derive(Archive, Deserialize, Serialize, Debug, PartialEq)]
#[rkyv(
    // This will generate a PartialEq impl between our unarchived
    // and archived types
    compare(PartialEq),
    // Derives can be passed through to the generated type:
    derive(Debug),
)]
struct Test {
    int: u8,
    string: String,
    option: Option<Vec<i32>>,
}

fn main() {
    let value = Test {
        int: 42,
        string: "hello world".to_string(),
        option: Some(vec![1, 2, 3, 4]),
    };

    // Serializing is as easy as a single function call
    let _bytes = rkyv::to_bytes::<Error>(&value).unwrap();

    // Or you can customize your serialization for better performance or control
    // over resource usage
    use rkyv::{api::high::to_bytes_with_alloc, ser::allocator::Arena};

    let mut arena = Arena::new();
    let bytes =
        to_bytes_with_alloc::<_, Error>(&value, arena.acquire()).unwrap();

    // You can use the safe API for fast zero-copy deserialization
    let archived = rkyv::access::<ArchivedTest, Error>(&bytes[..]).unwrap();
    assert_eq!(archived, &value);

    // Or you can use the unsafe API for maximum performance
    let archived =
        unsafe { rkyv::access_unchecked::<ArchivedTest>(&bytes[..]) };
    assert_eq!(archived, &value);

    // And you can always deserialize back to the original type
    let deserialized = deserialize::<Test, Error>(archived).unwrap();
    assert_eq!(deserialized, value);
}
```

_Note: the safe API requires the `bytecheck` feature (enabled by default)_
_Read more about [available features](https://docs.rs/rkyv/latest/rkyv/#features)._

# Thanks

Thanks to all the sponsors that keep development sustainable.
